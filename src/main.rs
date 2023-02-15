use libloading::Library;
use std::fs::{canonicalize, File};
use std::io::{empty, sink, stderr, stdin, stdout, Error, Read, Write};
use std::path::PathBuf;
use std::process::exit;

extern crate afed_objects;

pub mod bltns;
pub mod docmt;
pub mod expr;
mod linking;

use self::bltns::build_bltns;
use self::linking::load_from_pkgs;
use afed_objects::pkg::Pkg;

#[derive(Debug, Clone)]
enum Stream {
    Void, // Same as /dev/null
    Stdin,
    Stdout,
    Stderr,
    Path(PathBuf),
}

impl Stream {
    fn new(path: String, is_input: bool) -> Stream {
        if &path == "-" {
            if is_input {
                Stream::Stdin
            } else {
                Stream::Stdout
            }
        } else {
            Stream::Path(PathBuf::from(path))
        }
    }

    // Try to convert Stream into a PathBuf
    fn get_path(&self) -> Option<PathBuf> {
        match self {
            Stream::Void | Stream::Stdin | Stream::Stdout | Stream::Stderr => None,
            Stream::Path(buf) => Some(buf.clone()),
        }
    }

    // Create Reader using appropriate interface for each type of pipe
    fn to_reader(&self) -> Box<dyn Read> {
        match self {
            Stream::Void => Box::new(empty()),
            Stream::Stdin => Box::new(stdin()),
            Stream::Path(p) => Box::new(File::open(p).unwrap_or_else(|err| {
                eprintln!("IO Error while opening reader for {}: {}", p.display(), err);
                exit(1);
            })),

            Stream::Stdout => panic!("Cannot read from STDOUT"),
            Stream::Stderr => panic!("Cannot read from STDERR"),
        }
    }

    // Create Writer using appropriate interface for each type of pipe
    fn to_writer(&self) -> Box<dyn Write> {
        match self {
            Stream::Void => Box::new(sink()),
            Stream::Stdout => Box::new(stdout()),
            Stream::Stderr => Box::new(stderr()),
            Stream::Path(p) => Box::new(File::create(p).unwrap_or_else(|err| {
                eprintln!("IO Error while opening writer for {}: {}", p.display(), err);
                exit(1);
            })),

            Stream::Stdin => panic!("Cannot write to STDIN"),
        }
    }
}

impl PartialEq for Stream {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Stream::Void, Stream::Void) => true,
            (Stream::Stdin, Stream::Stdin) => true,
            (Stream::Stdout, Stream::Stdout) => true,
            (Stream::Stderr, Stream::Stderr) => true,
            (Stream::Path(p1), Stream::Path(p2)) => canonicalize(p1)
                .and_then(|abs1| canonicalize(p2).map(|abs2| abs1 == abs2))
                .unwrap_or(false),
            _ => false,
        }
    }
}

struct Params {
    clear: bool,
    input_path: Option<PathBuf>,
    input: Stream,
    output: Stream,
    errors: Stream,
}

const USAGE_MSG: &str = concat!(
    "Usage: afed [OPTION...] [-i] INPUT [[-o] OUTPUT]\n",
    "Try 'afed -h' or 'afed --help' for more information",
);
const HELP_MSG: &str = concat!(
    "Usage: afed [OPTION...] [-i] INPUT [[-o] OUTPUT]\n",
    "\n",
    "Evaluate expressions in place\n",
    "\n",
    "Options:\n",
    "  -i, --input INPUT       Input file to evaluate\n",
    "  -o, --output OUTPUT     Output file to store result to\n",
    "  -C, --check             Don't output file only check for errors\n",
    "  -d, --clear             Clear the content of every substitution (eg. = ``)\n",
    "  -n, --no-clobber        Make sure INFILE is not used as the output\n",
    "  -e, --errors ERRORS     File to send errors to. Defaults to STDERR\n",
    "  -E, --no-errors         Don't print any error messages\n",
    "  -h, -?, --help          Print this help message\n",
    "\n",
    "'-' may be used with -o, -i, or -e to indicate STDOUT, STDIN, or STDOUT, respectively\n",
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
);

// Die and print usage message
macro_rules! usage {
    ($($arg:tt),*) => {{
        eprintln!($($arg),*);
        eprintln!("{}", USAGE_MSG);
        exit(1);
    }};
}

// Parse command line arguments
impl Params {
    fn parse() -> Params {
        let mut input_path = None;
        let (mut input, mut output, mut errors) = (None, None, None);
        let (mut check, mut clear, mut no_clobber, mut no_errors) = (false, false, false, false);

        let mut args = std::env::args();
        _ = args.next();

        while let Some(opt) = args.next() {
            match opt.as_str() {
                "-i" | "--input" => {
                    if input.is_some() {
                        usage!("Input file already provided");
                    } else if let Some(path) = args.next() {
                        input = Some(Stream::new(path, true));
                    } else {
                        usage!("No input file provided to -i");
                    }
                }
                "-o" | "--ouput" => {
                    if output.is_some() {
                        usage!("Output file already provided");
                    } else if let Some(path) = args.next() {
                        output = Some(Stream::new(path, false));
                    } else {
                        usage!("No output file provided to -o");
                    }
                }

                "-f" | "--filename" => {
                    if input_path.is_some() {
                        usage!("Input filename already provided");
                    } else if let Some(path) = args.next() {
                        input_path = Some(PathBuf::from(path));
                    } else {
                        usage!("No input name provided to -f");
                    }
                }

                "-C" | "--check" => {
                    check = true;
                }
                "-d" | "--clear" => {
                    clear = true;
                }
                "-n" | "--no-clobber" => {
                    no_clobber = true;
                }

                "-E" | "--no-errors" => {
                    no_errors = true;
                }
                "-e" | "--errors" => {
                    if errors.is_some() {
                        usage!("Error output already provided");
                    } else if let Some(path) = args.next() {
                        errors = Some(Stream::new(path, false));
                    } else {
                        usage!("No error file provided");
                    }
                }

                "-h" | "-?" | "--help" => {
                    println!("{}", HELP_MSG);
                    exit(0);
                }

                _ => {
                    if input.is_none() {
                        input = Some(Stream::new(opt, true));
                    } else if output.is_none() {
                        output = Some(Stream::new(opt, false));
                    } else {
                        usage!("Extra positional argument: {}", opt);
                    }
                }
            }
        }

        // Use STDIN as default
        let input = input.unwrap_or(Stream::Stdin);
        // Use `input` as default
        let output = if check {
            Stream::Void
        } else if let Some(out) = output {
            out
        } else if let Stream::Stdin = input {
            Stream::Stdout
        } else {
            input.clone()
        };
        // Use STDERR as default
        let errors = if no_errors {
            Stream::Void
        } else if let Some(err) = errors {
            err
        } else {
            Stream::Stderr
        };

        // Use path of `input` as default
        let input_path = input_path.or_else(|| input.get_path());

        // Make sure the program doesn't accidently overwrite the file
        if no_clobber && input == output {
            usage!("Input and output files match, but --no-clobber is on");
        }

        Params {
            clear,
            input_path,
            input,
            output,
            errors,
        }
    }
}

fn parse_and_eval(prms: Params, libs: &mut Vec<Library>) -> Result<(), Error> {
    let mut prog = String::new();
    prms.input.to_reader().read_to_string(&mut prog)?;

    let mut doc = docmt::Docmt::new(prog, prms.input_path);
    doc.only_clear = prms.clear;
    let mut any_errors = false;
    let mut errout = prms.errors.to_writer();

    // Generate all of the builtins
    let mut pkgs = build_bltns();

    // Load the packages from pkg folder
    load_from_pkgs(&mut errout, &mut pkgs, libs).expect("IO Error while writing loading error");
    let pkgs = Pkg::from_map(pkgs);

    // Parse program and print parse errors to `errout`
    if let Err(count) = doc.parse(&mut errout, pkgs) {
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
    if let Err(err) = parse_and_eval(Params::parse(), &mut libs) {
        eprintln!("IO Error: {}", err);
        exit(1);
    }
}
