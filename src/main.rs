// Copyright (C) 2022-2023 Tanner Reese
/* This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use libloading::Library;
use std::fs::{canonicalize, File};
use std::io::{empty, sink, stderr, stdin, stdout, Error, Read, Write};
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{arg, error::ErrorKind, value_parser, Command};

extern crate afed_objects;

pub mod docmt;
pub mod expr;
pub mod pkgs;

use self::pkgs::LoadedPkgs;

#[derive(Debug, Clone)]
enum Stream {
    Void, // Same as /dev/null
    Std,  // STDIN or STDOUT
    Stderr,
    Path(PathBuf),
}

impl Stream {
    fn new(path: PathBuf) -> Stream {
        if path.as_path() == Path::new("-") {
            Stream::Std
        } else if path.as_path() == Path::new("-2") {
            Stream::Stderr
        } else {
            Stream::Path(path)
        }
    }

    // Try to convert Stream into a PathBuf
    fn get_path(&self) -> Option<PathBuf> {
        match self {
            Stream::Void | Stream::Std | Stream::Stderr => None,
            Stream::Path(buf) => Some(buf.clone()),
        }
    }

    // Create Reader using appropriate interface for each type of pipe
    fn to_reader(&self) -> Box<dyn Read> {
        match self {
            Stream::Void => Box::new(empty()),
            Stream::Std => Box::new(stdin()),
            Stream::Path(p) => Box::new(File::open(p).unwrap_or_else(|err| {
                eprintln!("IO Error while opening reader for {}: {}", p.display(), err);
                exit(1);
            })),

            Stream::Stderr => panic!("Cannot read from STDERR"),
        }
    }

    // Create Writer using appropriate interface for each type of pipe
    fn to_writer(&self) -> Box<dyn Write> {
        match self {
            Stream::Void => Box::new(sink()),
            Stream::Std => Box::new(stdout()),
            Stream::Stderr => Box::new(stderr()),
            Stream::Path(p) => Box::new(File::create(p).unwrap_or_else(|err| {
                eprintln!("IO Error while opening writer for {}: {}", p.display(), err);
                exit(1);
            })),
        }
    }
}

impl PartialEq for Stream {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Stream::Void, Stream::Void) => true,
            (Stream::Std, Stream::Std) => true,
            (Stream::Stderr, Stream::Stderr) => true,
            (Stream::Path(p1), Stream::Path(p2)) => canonicalize(p1)
                .and_then(|abs1| canonicalize(p2).map(|abs2| abs1 == abs2))
                .unwrap_or(false),
            _ => false,
        }
    }
}

struct Params {
    input: Stream,
    output: Stream,
    errors: Stream,
    clear: bool,
    input_path: Option<PathBuf>,
    pkg_dirs: Vec<PathBuf>,
    no_local_pkgs: bool,
}

fn get_params() -> Params {
    let mut prog = Command::new("afed")
    .about("Functionally evaluate expressions in place")
    .args(&[
        arg!([INPUT] "Input file to evaluate")
            .default_value("-")
            .value_parser(value_parser!(PathBuf)),
        arg!([OUTPUT] "Output file to store result to. Defaults to the INPUT")
            .value_parser(value_parser!(PathBuf)),
        arg!(-C --check "Don't output file. Only check for errors"),
        arg!(-d --clear "Clear the content of every substitution (eg. = ``)"),
        arg!(-n --"no-clobber"  "Make sure INPUT is not used as the output"),
        arg!(-e --errors <ERR_FILE> "File to send errors to. Defaults to STDERR")
            .default_value("-2")
            .value_parser(value_parser!(PathBuf)),
        arg!(-E --"no-errors"  "Don't print any error messages"),
        arg!(-f --filename <INPUT_PATH> "Indicate the internal name to use for the input")
            .value_parser(value_parser!(PathBuf)),
        arg!(-L --pkg <DIRECTORY> ... "Search in given directory for more packages to load")
            .value_parser(value_parser!(PathBuf)),
        arg!(--"no-local-pkgs"  "Don't attempt to load any packages from default locations")
    ])
    .after_help(concat!(
        "'-' may be used with -o, -i, or -e to indicate STDOUT, STDIN, or STDOUT, respectively\n",
        "'-2' may be used with -o and -e to indicate STDERR\n",
        "-f exists primarily for when input is STDIN, but you want to assign an actual name to the file.\n",
        "\n",
        "Examples:\n",
        "  # Read and Eval from STDIN, outputing and printing errors to STDOUT\n",
        "  afed -e - -\n",
        "  # Read and Eval file.af, printing back to file.af\n",
        "  afed -i file.af\n",
        "  # Read and Eval file.af, but don't output result\n",
        "  afed file.af -C\n",
        "  # Parse and clear all \"= ``\" expressions\n",
        "  afed file.af -d\n",
        "  # Read and Eval file.af and output to output.af\n",
        "  afed file.af -o output.af\n",
    ));

    let err_stderr_input = prog.error(ErrorKind::InvalidValue, "Input file cannot be STDERR");
    let err_no_clobber = prog.error(
        ErrorKind::ArgumentConflict,
        "Input and output files match, but --no-clobber is on",
    );
    let mut matches = prog.get_matches();

    let input = Stream::new(matches.remove_one("INPUT").unwrap());
    // Make sure input isn't being pulled from STDERR
    if input == Stream::Stderr {
        err_stderr_input.exit();
    }

    // Use path of `input` as default internal name
    let input_path = matches
        .remove_one::<PathBuf>("filename")
        .or_else(|| input.get_path());

    // Use `input` as default for `output`
    let output = if matches.remove_one("check").unwrap() {
        Stream::Void
    } else if let Some(out) = matches.remove_one("OUTPUT") {
        Stream::new(out)
    } else {
        input.clone()
    };

    // Use STDERR as default for `errors`
    let errors = if matches.remove_one("no-errors").unwrap() {
        Stream::Void
    } else {
        Stream::new(matches.remove_one("errors").unwrap())
    };

    // Check that input file isn't getting clobbered
    if matches.remove_one("no-clobber").unwrap() && input != Stream::Std && input == output {
        err_no_clobber.exit();
    }

    Params {
        input,
        output,
        errors,
        clear: matches.remove_one("clear").unwrap(),
        input_path,
        pkg_dirs: matches
            .remove_many::<PathBuf>("pkg")
            .map_or(vec![], |dirs| dirs.collect()),
        no_local_pkgs: matches.remove_one("no-local-pkgs").unwrap(),
    }
}

fn parse_and_eval(prms: Params, libs: &mut Vec<Library>) -> Result<(), Error> {
    let mut prog = String::new();
    prms.input.to_reader().read_to_string(&mut prog)?;

    let mut doc = docmt::Docmt::new(prog, prms.input_path);
    doc.only_clear = prms.clear;
    let mut any_errors = false;
    let mut errout = prms.errors.to_writer();

    // Load packages and builtins
    let mut pkgs = LoadedPkgs::new(libs);
    pkgs.build_bltns(&mut errout); // Generate builtins
    for folder in prms.pkg_dirs {
        // Load packages from user provided folders
        pkgs.load_from_folder(&mut errout, folder.as_path());
    }
    // Load packages and builtins
    // Load the packages from pkgs folder in config folder
    if !prms.no_local_pkgs {
        pkgs.load_from_config(&mut errout);
    }

    let pkg = pkgs.into_pkg();

    // Parse program and print parse errors to `errout`
    if let Err(count) = doc.parse(&mut errout, pkg) {
        any_errors = true;
        write!(
            &mut errout,
            "{} Parse Error{} encountered\n\n\n",
            count,
            if count == 1 { "" } else { "s" }
        )?;
    }

    // Evaluate AST and print eval errors to `errout`
    if let Err(count) = doc.eval(&mut errout) {
        any_errors = true;
        writeln!(
            &mut errout,
            "{} Eval Error{} encountered",
            count,
            if count == 1 { "" } else { "s" }
        )?;
    }

    // Still print something even if successful
    if !any_errors {
        writeln!(&mut errout, "No Errors encountered")?;
    }

    write!(prms.output.to_writer(), "{}", doc)?;
    Ok(())
}

fn main() {
    /* WARNING: Do not move the instantiation of libs inside `parse_and_eval`
     * Instantiating it here ensures that all of the loaded libraries
     * will have lifetimes that outlive the `Docmt` and `ExprArena`.
     */
    let mut libs = Vec::new();
    if let Err(err) = parse_and_eval(get_params(), &mut libs) {
        eprintln!("IO Error: {}", err);
        exit(1);
    }
}
