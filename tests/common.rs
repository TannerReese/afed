use std::io::Read;
use std::fs::{File, read_dir, metadata};
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

#[test]
fn parse_test() {
    for entry in read_dir(Path::new(TEST_FOLDER)).unwrap() {
        let mut path = entry.expect("Failed to get file entry").path();
        if !metadata(&path).expect("Failed to get metadata")
            .file_type().is_file()
        || "af" != path.extension().expect("Failed to get extension") {
            continue;
        }

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
        println!("Checking stderr against {}", path.file_name().unwrap().to_str().unwrap());
        let mut expected_stderr = String::new();
        File::open(&path).expect("Failed to open err file")
        .read_to_string(&mut expected_stderr).expect("Failed to read err file");
        line_by_line(&stderr, &expected_stderr);
    }
}

