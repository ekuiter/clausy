//! Utilities for executing external tools.

use crate::core::{file::File, var::VarId};
use crate::shell::options;
use num::BigInt;
use std::{
    env,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
    str::FromStr,
};
use tempfile::NamedTempFile;

/// Returns the path of a bundled external tool.
///
/// Looks up the tool (a) in its absolute path, if given, (b) in the working directory,
/// and (c) as a sibling of the currently running executable.
fn path(file_name: &str) -> String {
    let path = Path::new(file_name).to_path_buf();
    if path.is_absolute() && path.exists() {
        return file_name.to_owned();
    }
    if path.exists() {
        return format!("./{}", file_name);
    }
    let mut exe_path = env::current_exe().unwrap();
    exe_path.pop();
    exe_path.push(file_name);
    exe_path.to_str().unwrap().to_owned()
}

/// Checks satisfiability of a CNF formula.
///
/// Uses the user-specified SAT solver (--sat-path) if provided, otherwise falls back to kissat.
/// When using an arbitrary solver, the solution will be empty even if satisfiable.
/// This is because arbitrary solvers (e.g., tinisat) do not generally support extracting solutions.
pub(crate) fn sat(cnf: &str) -> Option<Vec<VarId>> {
    let tool_paths = &options().tool_paths;
    if let Some(sat_path) = &tool_paths.sat {
        arbitrary_sat(cnf, sat_path)
    } else {
        kissat(cnf)
    }
}

/// Attempts to find a solution of some CNF in DIMACS format.
///
/// Runs the external satisfiability solver kissat, which performs well on all known feature-model formulas.
/// Returns Some with the satisfying assignment if satisfiable, None otherwise.
fn kissat(cnf: &str) -> Option<Vec<VarId>> {
    let process = Command::new(path(&options().tool_paths.kissat))
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

/// Checks satisfiability using an arbitrary SAT solver.
///
/// The solver is invoked with the CNF file path as the first argument.
/// Output must contain "s SATISFIABLE" or "s UNSATISFIABLE".
/// Returns Some with an empty solution if satisfiable, None otherwise.
fn arbitrary_sat(cnf: &str, solver_path: &str) -> Option<Vec<VarId>> {
    let mut tmp = NamedTempFile::new().unwrap();
    write!(tmp, "{}", cnf).unwrap();
    let output = Command::new(path(solver_path))
        .arg(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.lines().any(|line| line.starts_with("s SATISFIABLE")) {
        Some(Vec::new())
    } else {
        None
    }
}

/// Counts the number of solutions of a CNF formula.
///
/// Uses the user-specified #SAT solver (--sharp-sat-path) if provided, otherwise falls back to d4.
pub(crate) fn sharp_sat(cnf: &str) -> BigInt {
    let tool_paths = &options().tool_paths;
    if let Some(sharp_sat_path) = &tool_paths.sharp_sat {
        arbitrary_sharp_sat(cnf, sharp_sat_path)
    } else {
        d4(cnf)
    }
}

/// Counts the number of solutions of some CNF in DIMACS format.
///
/// Runs the external model counter d4, which performs well on most small to medium size inputs.
fn d4(cnf: &str) -> BigInt {
    let mut tmp = NamedTempFile::new().unwrap();
    write!(tmp, "{}", cnf).unwrap();
    let output = Command::new(path(&options().tool_paths.d4))
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

/// Counts solutions using an arbitrary #SAT solver.
///
/// The solver is invoked with the CNF file path as the first argument.
/// Output must contain a line starting with "s " followed by the model count.
fn arbitrary_sharp_sat(cnf: &str, solver_path: &str) -> BigInt {
    let mut tmp = NamedTempFile::new().unwrap();
    write!(tmp, "{}", cnf).unwrap();
    let output = Command::new(path(solver_path))
        .arg(tmp.path())
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
    let process = Command::new(path(&options().tool_paths.bc_minisat_all))
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
/// Runs the tool FeatureIDE using the Java runtime environment, which is assumed to be available on the PATH variable.
pub(crate) fn io(
    file: &File,
    output_format: &str,
    variables: &[&str],
) -> File {
    let process = Command::new("java")
        .arg("-jar")
        .arg(path(&options().tool_paths.io))
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