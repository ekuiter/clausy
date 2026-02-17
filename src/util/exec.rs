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
    let mut exe_path = env::current_exe()
        .expect("failed to determine path of clausy executable");
    exe_path.pop();
    exe_path.push(file_name);
    exe_path
        .to_str()
        .expect("executable path contains invalid UTF-8")
        .to_owned()
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
    let kissat_path = path(&options().tool_paths.kissat);
    let process = Command::new(&kissat_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect(&format!(
            "failed to run SAT solver '{kissat_path}'. \
             Make sure kissat is installed, or specify a custom solver with --sat-path"
        ));
    process
        .stdin
        .expect("kissat stdin unavailable")
        .write_all(cnf.as_bytes())
        .expect("failed to write CNF to kissat");
    let mut output = String::new();
    process
        .stdout
        .expect("kissat stdout unavailable")
        .read_to_string(&mut output)
        .expect("failed to read kissat output");
    assert!(!output.is_empty(), "kissat produced no output");
    let solution: Vec<VarId> = output
        .lines()
        .filter(|line| line.starts_with("v "))
        .flat_map(|line| line[2..].split(' '))
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().expect(&format!("invalid literal in kissat output: '{s}'")))
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
    let mut tmp = NamedTempFile::new().expect("failed to create temporary file");
    write!(tmp, "{}", cnf).expect("failed to write CNF to temporary file");
    let resolved_path = path(solver_path);
    let output = Command::new(&resolved_path)
        .arg(tmp.path())
        .output()
        .expect(&format!(
            "failed to run SAT solver '{resolved_path}'. \
             Check that --sat-path is correct and the solver is executable"
        ));
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
    let mut tmp = NamedTempFile::new().expect("failed to create temporary file");
    write!(tmp, "{}", cnf).expect("failed to write CNF to temporary file");
    let d4_path = path(&options().tool_paths.d4);
    let output = Command::new(&d4_path)
        .arg("-i")
        .arg(tmp.path())
        .arg("-m")
        .arg("counting")
        .output()
        .expect(&format!(
            "failed to run model counter '{d4_path}'. \
             Make sure d4 is installed, or specify a custom solver with --sharp-sat-path"
        ));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count_line = stdout
        .lines()
        .find(|line| line.starts_with("s "))
        .expect(&format!("d4 output missing solution count (expected 's <count>'):\n{stdout}"));
    let count_str = count_line.split_at(2).1;
    BigInt::from_str(count_str)
        .expect(&format!("d4 output contains invalid model count: '{count_str}'"))
}

/// Counts solutions using an arbitrary #SAT solver.
///
/// The solver is invoked with the CNF file path as the first argument.
/// Output must contain a line starting with "s " followed by the model count.
fn arbitrary_sharp_sat(cnf: &str, solver_path: &str) -> BigInt {
    let mut tmp = NamedTempFile::new().expect("failed to create temporary file");
    write!(tmp, "{}", cnf).expect("failed to write CNF to temporary file");
    let resolved_path = path(solver_path);
    let output = Command::new(&resolved_path)
        .arg(tmp.path())
        .output()
        .expect(&format!(
            "failed to run #SAT solver '{resolved_path}'. \
             Check that --sharp-sat-path is correct and the solver is executable"
        ));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count_line = stdout
        .lines()
        .find(|line| line.starts_with("s "))
        .expect(&format!("#SAT solver output missing count (expected 's <count>'):\n{stdout}"));
    let count_str = count_line.split_at(2).1;
    BigInt::from_str(count_str)
        .expect(&format!("#SAT solver output contains invalid model count: '{count_str}'"))
}

/// Enumerates all solutions of some CNF in DIMACS format.
///
/// Runs an external AllSAT solver, which is only suitable for formulas with few solutions.
/// This does not currently output solutions for fully indeterminate (i.e., unconstrained) variables.
pub(crate) fn bc_minisat_all(cnf: &str) -> (impl Iterator<Item = Vec<VarId>>, NamedTempFile) {
    let mut tmp_in = NamedTempFile::new().expect("failed to create temporary file");
    write!(tmp_in, "{}", cnf).expect("failed to write CNF to temporary file");
    let solver_path = path(&options().tool_paths.bc_minisat_all);
    let process = Command::new(&solver_path)
        .arg(tmp_in.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect(&format!(
            "failed to run AllSAT solver '{solver_path}'. \
             Make sure bc_minisat_all is installed and accessible"
        ));
    let iter = BufReader::new(process.stderr.expect("bc_minisat_all stderr unavailable"))
        .lines()
        .map(|line| {
            let line = line.expect("failed to read bc_minisat_all output");
            line.split(' ')
                .filter(|s| !s.is_empty())
                .map(|s| s.parse().expect(&format!("invalid literal in bc_minisat_all output: '{s}'")))
                .filter(|literal| *literal != 0)
                .collect::<Vec<VarId>>()
        });
    (iter, tmp_in)
}

/// Converts a given feature-model file from one format into another.
///
/// Runs the tool FeatureIDE using the Java runtime environment, which is assumed to be available on the PATH variable.
pub(crate) fn io(file: &File, output_format: &str, variables: &[&str]) -> File {
    let io_path = path(&options().tool_paths.io);
    let process = Command::new("java")
        .arg("-jar")
        .arg(&io_path)
        .arg(&file.name)
        .arg(output_format)
        .arg(variables.join(","))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect(&format!(
            "failed to run FeatureIDE I/O interface '{io_path}'. \
             Make sure Java is installed and on PATH"
        ));
    if file.name.starts_with("-.") {
        process
            .stdin
            .expect("FeatureIDE stdin unavailable")
            .write_all(file.contents.as_bytes())
            .expect("failed to write to FeatureIDE");
    }
    let mut output = String::new();
    let mut error = String::new();
    process
        .stdout
        .expect("FeatureIDE stdout unavailable")
        .read_to_string(&mut output)
        .expect("failed to read FeatureIDE output");
    process
        .stderr
        .expect("FeatureIDE stderr unavailable")
        .read_to_string(&mut error)
        .expect("failed to read FeatureIDE errors");
    if error.trim() == "No path set for model. Can't load imported models." {
        error = String::new();
    }
    if !error.is_empty() {
        println!("{}", error);
    }
    assert!(
        error.is_empty() && !output.is_empty(),
        "FeatureIDE conversion failed for '{}' -> '{}': {}",
        file.name,
        output_format,
        if error.is_empty() { "no output produced" } else { &error }
    );
    File::new(format!("-.{}", output_format), output)
}