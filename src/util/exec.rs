//! Utilities for executing external programs.

use crate::core::{var::VarId, file::File};
use num::BigInt;
use std::{
    env,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
    str::FromStr,
};
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
        return path.to_str().unwrap().to_owned();
    }
    let path = Path::new(file_name).to_path_buf();
    if path.exists() {
        return format!("./{}", file_name);
    }
    let path = Path::new(&format!("bin/{}", file_name)).to_path_buf();
    if path.exists() {
        return path.to_str().unwrap().to_owned();
    }
    unreachable!()
}

/// Attempts to find a solution of some CNF in DIMACS format.
///
/// Runs the external satisfiability solver counter kissat, which performs well on all known feature-model formulas.
pub(crate) fn kissat(cnf: &str) -> Option<Vec<VarId>> {
    let process = Command::new(path("kissat"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process.stdin.unwrap().write_all(cnf.as_bytes()).unwrap();
    let mut output = String::new();
    process.stdout.unwrap().read_to_string(&mut output).unwrap();
    assert!(!output.is_empty());
    let solution: Vec<VarId> = output
        .lines()
        .filter(|line| line.starts_with("v "))
        .map(|line| line[2..].split(' ').collect::<Vec<&str>>())
        .flatten()
        .map(|str| str.parse().unwrap())
        .filter(|literal| *literal != 0)
        .collect();
    if solution.len() > 0 {
        Some(solution)
    } else {
        None
    }
}

/// Counts the number of solutions of some CNF in DIMACS format.
///
/// Runs the external model counter d4, which performs well on most small to medium size inputs.
/// Returns the number as a string, as it will typically overflow otherwise.
pub(crate) fn d4(cnf: &str) -> BigInt {
    let mut tmp = NamedTempFile::new().unwrap();
    write!(tmp, "{}", cnf).unwrap();
    let output = Command::new(path("d4"))
        .arg("-i")
        .arg(tmp.path())
        .arg("-m")
        .arg("counting")
        .output()
        .unwrap();
    BigInt::from_str(&String::from(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find(|line| line.starts_with("s "))
            .unwrap()
            .split_at(2)
            .1,
    ))
    .unwrap()
}

/// Enumerates all solutions of some CNF in DIMACS format.
///
/// Runs an external AllSAT solver, which is only suitable for formulas with few solutions.
/// This does not currently output solutions for fully indeterminate (i.e., unconstrained) variables.
pub(crate) fn bc_minisat_all(cnf: &str) -> (impl Iterator<Item = Vec<VarId>>, NamedTempFile) {
    let mut tmp_in = NamedTempFile::new().unwrap();
    write!(tmp_in, "{}", cnf).unwrap();
    let process = Command::new(path("bc_minisat_all"))
        .arg(tmp_in.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let iter = BufReader::new(process.stderr.unwrap()).lines().map(|line| {
        line.unwrap()
            .split(' ')
            .map(|literal| literal.parse().unwrap())
            .filter(|literal| *literal != 0)
            .collect::<Vec<VarId>>()
    });
    (iter, tmp_in)
}

/// Converts a given feature-model file from one format into another.
///
/// Runs the tool FeatureIDE using the Java runtime environment.
pub(crate) fn io(
    file: &File,
    output_format: &str,
    variables: &[&str],
) -> File {
    let process = Command::new("java")
        .arg("-jar")
        .arg(path("io.jar"))
        .arg(&file.name)
        .arg(output_format)
        .arg(variables.join(","))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if file.name.starts_with("-.") {
        process.stdin.unwrap().write_all(file.contents.as_bytes()).unwrap();
    }
    let mut output = String::new();
    let mut error = String::new();
    process.stdout.unwrap().read_to_string(&mut output).unwrap();
    process.stderr.unwrap().read_to_string(&mut error).unwrap();
    if error.trim() == "No path set for model. Can't load imported models." {
        error = String::new();
    }
    if !error.is_empty() {
        println!("{}", error);
    }
    assert!(error.is_empty() && !output.is_empty());
    File::new(format!("-.{}", output_format), output)
}