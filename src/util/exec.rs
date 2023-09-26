//! Utilities for executing external programs.

use std::{process::{Command, Stdio},io::{Write, Read}, env, path::Path};

use tempfile::NamedTempFile;

/// Returns the path of a bundled external program.
/// 
/// Looks up the program (a) as a sibling of the currently running executable, (b) in the working directory,
/// and (c) in the `bin` directory in the working directory, if that exists.
fn path(file_name: &str) -> String {
    let mut path = env::current_exe().unwrap();
    path.pop();
    path.push(file_name);
    if path.exists() {
        return path.to_str().unwrap().to_owned()
    }
    let path = Path::new(file_name).to_path_buf();
    if path.exists() {
        return format!("./{}", file_name)
    }
    let path = Path::new(&format!("bin/{}", file_name)).to_path_buf();
    if path.exists() {
        return path.to_str().unwrap().to_owned()
    }
    panic!("could not locate file {}", file_name);
}

/// Counts the number of satisfying assignments of some CNF in DIMACS format.
/// 
/// Runs the efficient external model counter d4, which performs well on most small to medium size inputs.
/// Returns the number as a string, as it will typically overflow otherwise.
pub(crate) fn d4(dimacs: &str) -> String {
    let mut tmp = NamedTempFile::new().unwrap();
    write!(tmp, "{}", dimacs).unwrap();
    let output = Command::new(path("d4"))
        .arg("-i")
        .arg(tmp.path())
        .arg("-m")
        .arg("counting")
        .arg("-p")
        .arg("sharp-equiv")
        .output()
        .unwrap();
    String::from(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find(|line| line.starts_with("s "))
            .unwrap()
            .split_at(2)
            .1,
    )
}

/// Converts a given feature-model file from one format into another.
/// 
/// Runs the tool FeatureIDE using the Java runtime environment.
pub(crate) fn io(input: &str, input_format: &str, output_format: &str) -> String {
    let process = Command::new("java")
        .arg("-jar")
        .arg(path("io.jar"))
        .arg(format!("-.{}", input_format))
        .arg(output_format)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process.stdin.unwrap().write_all(input.as_bytes()).unwrap();
    let mut output = String::new();
    process.stdout.unwrap().read_to_string(&mut output).unwrap();
    assert!(!output.is_empty());
    output
}