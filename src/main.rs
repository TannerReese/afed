use std::io::{Write, Read, Error, empty, sink, stdin, stdout, stderr};
use std::process::exit;
use std::fs::{File, canonicalize};
use std::path::PathBuf;

#[macro_use]
pub mod object;
pub mod libs;
pub mod expr;
pub mod docmt;

#[derive(Debug, Clone)]
enum Stream {
    Void,
    Stdin,
    Stdout,
    Stderr,
    Path(PathBuf),
}

impl Stream {
    fn new(path: String, is_input: bool) -> Stream {
        if &path == "-" {
            if is_input { Stream::Stdin }
            else { Stream::Stdout }
        } else { Stream::Path(PathBuf::from(path)) }
    }
    
    fn to_reader(&self) -> Box<dyn Read> {
        match self {
            Stream::Void => Box::new(empty()),
            Stream::Stdin => Box::new(stdin()),
            Stream::Path(p) => Box::new(File::open(p).unwrap_or_else(|err| {
                eprintln!("IO Error while opening reader for {}: {}",
                    p.display(), err
                );
                exit(1);
            })),
            
            Stream::Stdout => panic!("Cannot read from STDOUT"),
            Stream::Stderr => panic!("Cannot read from STDERR"),
        }
    }
    
    fn to_writer(&self) -> Box<dyn Write> {
        match self {
            Stream::Void => Box::new(sink()),
            Stream::Stdout => Box::new(stdout()),
            Stream::Stderr => Box::new(stderr()),
            Stream::Path(p) => Box::new(File::create(p).unwrap_or_else(|err| {
                eprintln!("IO Error while opening writer for {}: {}",
                    p.display(), err
                );
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
            (Stream::Path(p1), Stream::Path(p2)) => canonicalize(p1).and_then(|abs1|
                canonicalize(p2).map(|abs2| abs1 == abs2)
            ).unwrap_or(false),
            _ => false,
        }
    }
}



struct Params {
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
    "  # Read and Eval file.af and output to output.af\n",
    "  afed file.af -o output.af\n",
);

impl Params {
    fn parse() -> Params {
        let (mut input, mut output, mut errors) = (None, None, None);
        let (mut check, mut no_clobber, mut no_errors) = (false, false, false);
        
        let mut args = std::env::args();
        _ = args.next();
        while let Some(opt) = args.next() {
            match opt.as_str() {
                "-i" | "--input" => if let Some(_) = input {
                    eprintln!("Input file already provided\n{}", USAGE_MSG);
                    exit(1);
                } else if let Some(path) = args.next() {
                    input = Some(Stream::new(path, true));
                } else {
                    eprintln!("No input file provided to -i\n{}", USAGE_MSG);
                    exit(1);
                },
                "-o" | "--ouput" => if let Some(_) = output {
                    eprintln!("Output file already provided\n{}", USAGE_MSG);
                    exit(1);
                } else if let Some(path) = args.next() {
                    output = Some(Stream::new(path, false));
                } else {
                    eprintln!("No output file provided to -o\n{}", USAGE_MSG);
                    exit(1);
                },
                
                "-C" | "--check" => { check = true; },
                "-n" | "--no-clobber" => { no_clobber = true; },
                
                "-e" | "--errors" => if let Some(_) = errors {
                    eprintln!("Error output already provided\n{}", USAGE_MSG);
                    exit(1);
                } else if let Some(path) = args.next() {
                    errors = Some(Stream::new(path, false));
                } else {
                    eprintln!("No error file provided to -e\n{}", USAGE_MSG);
                    exit(1);
                },
                "-E" | "--no-errors" => { no_errors = true; },
                
                "-h" | "-?" | "--help" => {
                    println!("{}", HELP_MSG);
                    exit(0);
                },
                
                _ => {
                    if let None = input {
                        input = Some(Stream::new(opt, true));
                    } else if let None = output {
                        output = Some(Stream::new(opt, false));
                    } else {
                        eprintln!("Extra positional argument: {}\n{}", opt, USAGE_MSG);
                        exit(1);
                    }
                },
            }
        }
        
        let input = input.unwrap_or(Stream::Stdin);
        let output = if check { Stream::Void }
            else if let Some(out) = output { out }
            else if let Stream::Stdin = input { Stream::Stdout }
            else { input.clone() }; 
        let errors = if no_errors { Stream::Void }
            else if let Some(err) = errors { err }
            else { Stream::Stderr };
        
        if no_clobber && input == output {
            eprintln!("Input and output files match, but --no-clobber is on\n{}", USAGE_MSG);
            exit(1);
        }
        
        Params { input, output, errors }
    }
}


fn parse_and_eval(prms: Params) -> Result<(), Error> { 
    let mut prog = String::new();
    prms.input.to_reader().read_to_string(&mut prog)?;
    
    let mut doc = docmt::Docmt::new(prog);
    let mut any_errors = false;
    
    let bltns = libs::make_bltns();
    
    let mut errout = prms.errors.to_writer();
    if let Err(count) = doc.parse(&mut errout, &bltns) {
        any_errors = true;
        if count == 1 {
            write!(&mut errout, "1 Parse Error encountered\n\n\n")?;
        } else {
            write!(&mut errout, "{} Parse Errors encountered\n\n\n", count)?;
        }
    }
    
    if let Err(count) = doc.eval(&mut errout) {
        any_errors = true;
        if count == 1 {
            write!(&mut errout, "1 Eval Error encountered\n")?;
        } else {
            write!(&mut errout, "{} Eval Errors encountered\n", count)?;
        }
    }
    
    if !any_errors { write!(&mut errout, "No Errors encountered\n")?; }
    
    write!(prms.output.to_writer(), "{}", doc)?;
    Ok(())
}

fn main(){
    if let Err(err) = parse_and_eval(Params::parse()) {
        eprintln!("IO Error: {}", err);
        exit(1);
    }
}

