use std::{process::{Command, Stdio},io::{Write, Read}, env, path::Path};

use tempfile::NamedTempFile;

fn path(file: &str) -> String {
    let mut path = env::current_exe().unwrap();
    path.pop();
    path.push(file);
    if path.exists() {
        return path.to_str().unwrap().to_owned()
    }
    let path = Path::new(file).to_path_buf();
    if path.exists() {
        return format!("./{}", file)
    }
    let path = Path::new(&format!("bin/{}", file)).to_path_buf();
    if path.exists() {
        return path.to_str().unwrap().to_owned()
    }
    panic!("could not locate file {}", file);
}

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
    let mut dimacs = String::new();
    process.stdout.unwrap().read_to_string(&mut dimacs).unwrap();
    dimacs
}