use std::ffi::OsStr;
use std::fs::{read_to_string, File};
use std::io::Read;
use std::path::Path;
use std::process::Command;

const BINARY_PATH: &str = "./target/debug/afed";
const TEST_FOLDER: &str = "./tests/examples";

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

fn run_file<I, S>(filename: &Path, args: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut path = Path::new(TEST_FOLDER).join(filename);
    path.set_extension("af");
    println!("Testing {}", path.file_name().unwrap().to_str().unwrap());

    let output = Command::new(BINARY_PATH)
        .arg(path.as_os_str())
        .args(args)
        .output()
        .expect("Failed to execute process");
    if let Some(code) = output.status.code() {
        assert!(code <= 1, "Program errored with code {}", code);
    } else {
        panic!("Program was killed by signal")
    }

    let stdout = String::from_utf8(output.stdout).expect("Failed to parse STDOUT as Unicode");
    let stderr = String::from_utf8(output.stderr).expect("Failed to parse STDERR as Unicode");

    println!("Stdout: \n{}", stdout);
    println!("Stderr: \n{}", stderr);

    path.set_extension("out");
    println!(
        "Checking stdout against {}",
        path.file_name().unwrap().to_str().unwrap()
    );
    let expected_stdout = read_to_string(&path).expect("Failed to read .out file");
    line_by_line(&stdout, &expected_stdout);

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

macro_rules! test_file {
    ($testname:ident, $filename:literal) => {
        test_file! {$testname, $filename, ["-"]}
    };
    ($testname:ident, $filename:literal, $args:expr) => {
        #[test]
        fn $testname() {
            run_file(Path::new($filename), $args);
        }
    };
}

test_file! {parse, "parse.af"}
test_file! {parse_errors, "parse_errors.af"}
test_file! {help, "help.af"}
test_file! {func, "func.af"}

test_file! {use_stmt, "parent_use.af"}
test_file! {clear, "clear.af", ["-d", "-"]}

test_file! {object_bool, "object/bool.af"}
test_file! {object_number, "object/number.af"}
test_file! {object_string, "object/string.af"}
test_file! {object_array, "object/array.af"}
test_file! {object_map, "object/map.af"}

test_file! {bltns_num, "bltns/num.af"}
test_file! {bltns_arr, "bltns/arr.af"}
test_file! {bltns_prs, "bltns/prs.af"}
test_file! {bltns_mod, "bltns/mod.af"}
test_file! {bltns_vec, "bltns/vec.af"}
test_file! {bltns_mat, "bltns/mat.af"}
test_file! {bltns_calc, "bltns/calc.af"}
test_file! {bltns_plt, "bltns/plt.af"}
