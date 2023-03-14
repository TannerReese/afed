use std::ffi::OsStr;
use std::fs::{read_to_string, File};
use std::io::Read;
use std::path::Path;
use std::process::Command;

const TEST_FOLDER: &str = "./tests";

// Panics on first pair of lines that differ between `s1` and `s2`
fn line_by_line(s1: &str, s2: &str) {
    let (mut lns1, mut lns2) = (s1.lines(), s2.lines());
    loop {
        let (line1, line2) = (lns1.next(), lns2.next());
        assert_eq!(line1, line2);
        if let None = line1 {
            return;
        }
    }
}

// Execute .af file and check results
pub fn run_file<I, S>(binary: S, filename: &Path, args: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut path = Path::new(TEST_FOLDER).join(filename);
    path.set_extension("af");
    // Make sure test file exists before proceeding
    match path.try_exists() {
        Ok(true) => {},
        _ => panic!("Test file {:?} does not exist", path.display()),
    }
    println!("Testing {:?}", path.display());

    // Execute Afed on test file with arguments
    let output = Command::new(binary)
        .arg(path.as_os_str())
        .args(args)
        .output()
        .expect("Failed to execute process");
    // Check that program wasn't killed or segfaulted
    if let Some(code) = output.status.code() {
        assert!(code <= 1, "Program errored with code {}", code);
    } else {
        panic!("Program was killed by signal")
    }

    let stdout = String::from_utf8(output.stdout).expect("Failed to parse STDOUT as Unicode");
    let stderr = String::from_utf8(output.stderr).expect("Failed to parse STDERR as Unicode");

    println!("Stdout: \n{}", stdout);
    println!("Stderr: \n{}", stderr);

    // Load expected output and check against result
    path.set_extension("out");
    println!(
        "Checking stdout against {}",
        path.file_name().unwrap().to_str().unwrap()
    );
    let expected_stdout = read_to_string(&path).expect("Failed to read .out file");
    line_by_line(&stdout, &expected_stdout);

    // Load expected error and check against result
    path.set_extension("err");
    let mut expected_stderr = String::new();
    if let Ok(mut fl) = File::open(&path) {
        println!(
            "Checking stderr against {}",
            path.file_name().unwrap().to_str().unwrap()
        );
        fl.read_to_string(&mut expected_stderr)
            .expect("Failed to read err file");
    } else {
        expected_stderr += "No Errors encountered";
    }
    line_by_line(&stderr, &expected_stderr);
}

// Creates functions to run test files
#[macro_export]
macro_rules! test_file {
    ($testname:ident, $filename:literal) => {
        test_file! {$testname, $filename, "./target/debug", ["-", "--no-local-pkgs"]}
    };
    ($testname:ident, $filename:literal, [$($arg:expr),*]) => {
        test_file! {$testname, $filename, "./target/debug", [$($arg),*]}
    };

    // --no-local-pkgs is necessary to prevent the local configuration from affecting tests
    ($testname:ident, $filename:literal, $pkg_folder:literal) => {
        test_file! {$testname, $filename, $pkg_folder, ["-", "--no-local-pkgs"]}
    };
    ($testname:ident, $filename:literal, $pkg_folder:literal, [$($arg:expr),*]) => {
        #[test]
        fn $testname() {
            afed_objects::testing::run_file(
                "afed", ::std::path::Path::new($filename),
                [$($arg),* , "--no-local-pkgs", "-L", $pkg_folder]
            );
        }
    };
}

