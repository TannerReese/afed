use std::io::Read;
use std::fs::File;
use std::path::Path;
use std::process::{Command, Output};

const BINARY_PATH: &str = "./target/debug/afed";
const TEST_FOLDER: &str = "./tests/examples";

fn line_by_line(s1: &str, s2: &str) {
    let (mut lns1, mut lns2) = (s1.lines(), s2.lines());
    loop {
        let (line1, line2) = (lns1.next(), lns2.next());
        assert_eq!(line1, line2);
        if let None = line1 { return; }
    }
}

fn run_file(filename: &Path) {
    let mut path = Path::new(TEST_FOLDER).join(filename);
    path.set_extension("af");
    println!("Testing {}", path.file_name().unwrap().to_str().unwrap());

    let output = Command::new(BINARY_PATH)
        .arg(path.as_os_str()).arg("-")
        .output().expect("Failed to execute process");
    let Output {stdout, stderr, ..} = output;
    let stdout = String::from_utf8(stdout).expect("Failed to parse STDOUT as Unicode");
    let stderr = String::from_utf8(stderr).expect("Failed to parse STDERR as Unicode");

    println!("Stdout: \n{}", stdout);
    println!("Stderr: \n{}", stderr);

    path.set_extension("out");
    println!("Checking stdout against {}", path.file_name().unwrap().to_str().unwrap());
    let mut expected_stdout = String::new();
    File::open(&path).expect("Failed to open out file")
    .read_to_string(&mut expected_stdout).expect("Failed to read out file");
    line_by_line(&stdout, &expected_stdout);

    path.set_extension("err");
    let mut expected_stderr = String::new();
    if let Ok(mut fl) = File::open(&path) {
        println!("Checking stderr against {}",
            path.file_name().unwrap().to_str().unwrap()
        );
        fl.read_to_string(&mut expected_stderr)
        .expect("Failed to read err file");
    } else { expected_stderr += "No Errors encountered"; }
    line_by_line(&stderr, &expected_stderr);
}

macro_rules! test_file {
    ($testname:ident, $filename:literal) => {
        #[test]
        fn $testname() { run_file(Path::new($filename)); }
    };
}

test_file!{parse, "parse.af"}
test_file!{parse_errors, "parse_errors.af"}
test_file!{func, "func.af"}

test_file!{object_bool, "object/bool.af"}
test_file!{object_number, "object/number.af"}
test_file!{object_string, "object/string.af"}
test_file!{object_array, "object/array.af"}
test_file!{object_map, "object/map.af"}

test_file!{libs_num, "libs/num.af"}
test_file!{libs_vec, "libs/vec.af"}
test_file!{libs_mat, "libs/mat.af"}

