//! Utilities for executing external programs.

use std::{
    env, fs,
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
};

use tempfile::NamedTempFile;

use crate::core::formula::VarId;

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
pub(crate) fn kissat(dimacs: &str) -> Option<Vec<VarId>> {
    let process = Command::new(path("kissat_MAB-HyWalk"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process.stdin.unwrap().write_all(dimacs.as_bytes()).ok();
    let mut output = String::new();
    process.stdout.unwrap().read_to_string(&mut output).ok();
    debug_assert!(!output.is_empty());
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
pub(crate) fn d4(dimacs: &str) -> String {
    let mut tmp = NamedTempFile::new().unwrap();
    write!(tmp, "{}", dimacs).ok();
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

/// Enumerates all solutions of some CNF in DIMACS format.
///
/// Runs an external AllSAT solver, which is only suitable for formulas with few solutions.
pub(crate) fn bc_minisat_all(dimacs: &str) -> Vec<Vec<VarId>> {
    let mut tmp_in = NamedTempFile::new().unwrap();
    let tmp_out = NamedTempFile::new().unwrap();
    write!(tmp_in, "{}", dimacs).ok();
    Command::new(path("bc_minisat_all_static"))
        .arg(tmp_in.path())
        .arg(tmp_out.path())
        .output()
        .ok();
    fs::read_to_string(tmp_out.path())
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split(' ').collect::<Vec<&str>>())
        .map(|literals| {
            literals
                .iter()
                .map(|literals| literals.parse().unwrap())
                .filter(|literal| *literal != 0)
                .collect()
        })
        .collect()
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
    process.stdin.unwrap().write_all(input.as_bytes()).ok();
    let mut output = String::new();
    process.stdout.unwrap().read_to_string(&mut output).ok();
    debug_assert!(!output.is_empty());
    output
}
