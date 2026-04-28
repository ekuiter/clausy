//! Utilities for executing external tools.

use crate::core::{file::File, var::VarId};
use crate::shell::options;
use crate::util::log::{log, scope};
use clap::ValueEnum;
use num::BigInt;
use std::{
    env,
    ffi::OsString,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
    str::FromStr,
    sync::OnceLock,
    thread,
    time::Duration,
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
    let mut exe_path = env::current_exe().expect("failed to determine path of clausy executable");
    exe_path.pop();
    exe_path.push(file_name);
    exe_path
        .to_str()
        .expect("executable path contains invalid UTF-8")
        .to_owned()
}

/// Logs the exact command line about to be invoked for an external tool.
fn log_invoked_command(program: &str, args: &[OsString]) {
    fn shell_escape_if_needed(s: &str) -> String {
        let safe = !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c));
        if safe {
            s.to_owned()
        } else {
            format!("'{}'", s.replace('\'', "'\\''"))
        }
    }

    let mut command = shell_escape_if_needed(program);
    for arg in args {
        command.push(' ');
        command.push_str(&shell_escape_if_needed(&arg.to_string_lossy()));
    }
    let display = if command.len() > 180 {
        format!("{}…", &command[..180])
    } else {
        command
    };
    log(&format!("[EXEC] invoking command: {display}"));
}

/// Checks satisfiability of a CNF formula.
///
/// Uses the user-specified SAT solver (--sat-path) if provided, otherwise falls back to kissat.
/// When using an arbitrary solver, the solution will be empty even if satisfiable.
/// This is because arbitrary solvers (e.g., tinisat) do not generally support extracting solutions.
pub(crate) fn sat(cnf: &str) -> Option<Vec<VarId>> {
    static LOGGED: OnceLock<()> = OnceLock::new();
    let tool = &options().tool;
    if let Some(sat_path) = &tool.sat_path {
        LOGGED.get_or_init(|| {
            log(&format!(
                "[EXEC] SAT solving will use the custom solver configured at {sat_path}"
            ))
        });
        arbitrary_sat(cnf, sat_path)
    } else {
        LOGGED.get_or_init(|| log("[EXEC] SAT solving will use the default kissat solver"));
        kissat(cnf)
    }
}

/// Attempts to find a solution of some CNF in DIMACS format.
///
/// Runs the external satisfiability solver kissat, which performs well on all known feature-model formulas.
/// Returns Some with the satisfying assignment if satisfiable, None otherwise.
fn kissat(cnf: &str) -> Option<Vec<VarId>> {
    let kissat_path = path(&options().tool.kissat_path);
    log_invoked_command(&kissat_path, &[]);
    log(&format!(
        "[EXEC] starting SAT solver process using executable {}",
        kissat_path
    ));
    let _timing = scope("EXEC", "SAT solve via kissat");
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
        .map(|s| {
            s.parse()
                .expect(&format!("invalid literal in kissat output: '{s}'"))
        })
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
/// Extraction of solutions is currently not supported for an arbitrary SAT solver.
fn arbitrary_sat(cnf: &str, solver_path: &str) -> Option<Vec<VarId>> {
    let mut tmp = NamedTempFile::new().expect("failed to create temporary file");
    write!(tmp, "{}", cnf).expect("failed to write CNF to temporary file");
    let resolved_path = path(solver_path);
    let args = vec![tmp.path().as_os_str().to_owned()];
    log_invoked_command(&resolved_path, &args);
    log(&format!(
        "[EXEC] starting custom SAT solver process using executable {}",
        resolved_path
    ));
    let _timing = scope("EXEC", "SAT solve via custom solver");
    let output = Command::new(&resolved_path)
        .args(&args)
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
pub(crate) fn sharp_sat(cnf: &str, projected: bool) -> BigInt {
    static LOGGED: OnceLock<()> = OnceLock::new();
    if let Some(sharp_sat_path) = &options().tool.sharp_sat_path {
        LOGGED.get_or_init(|| {
            log(&format!(
            "[EXEC] model counting will use the custom #SAT solver configured at {sharp_sat_path}"
        ))
        });
        arbitrary_sharp_sat(cnf, sharp_sat_path, projected)
    } else {
        LOGGED.get_or_init(|| log("[EXEC] model counting will use the default d4 solver"));
        d4(cnf, projected)
    }
}

/// Waits for a spawned child process, collecting its stdout.
///
/// Reads stdout on a background thread to avoid pipe-buffer deadlock.
/// If `--sharp-sat-timeout` is non-zero and the process exceeds it, the process is killed
/// and `None` is returned. Otherwise returns the captured stdout.
fn wait_with_timeout(mut child: std::process::Child, name: String) -> Option<String> {
    let mut stdout_handle = child
        .stdout
        .take()
        .expect(&format!("{name} stdout unavailable"));
    let name_for_thread = name.clone();
    let stdout_thread = thread::spawn(move || {
        let mut s = String::new();
        stdout_handle
            .read_to_string(&mut s)
            .expect(&format!("failed to read {name_for_thread} output"));
        s
    });
    let timeout = options().tool.sharp_sat_timeout;
    let timed_out = if timeout > 0 {
        use wait_timeout::ChildExt;
        child
            .wait_timeout(Duration::from_secs(timeout))
            .expect(&format!("failed to wait for {name}"))
            .is_none()
    } else {
        child.wait().expect(&format!("failed to wait for {name}"));
        false
    };
    if timed_out {
        child.kill().ok();
        child.wait().ok();
        drop(stdout_thread); // detach — orphaned child processes may still hold the pipe open
        log(&format!(
            "[EXEC] {name} timed out after {timeout}s, returning -1"
        ));
        None
    } else {
        Some(
            stdout_thread
                .join()
                .expect(&format!("{name} stdout thread panicked")),
        )
    }
}

/// Counts the number of solutions of some CNF in DIMACS format.
///
/// Runs the external model counter d4, which performs well on most small to medium size inputs.
/// Returns -1 if the configured timeout (--sharp-sat-timeout) is exceeded.
fn d4(cnf: &str, projected: bool) -> BigInt {
    let mut tmp = NamedTempFile::new().expect("failed to create temporary file");
    write!(tmp, "{}", cnf).expect("failed to write CNF to temporary file");
    let d4_path = path(&options().tool.d4_path);
    let d4_mode = if projected {
        options()
            .tool
            .d4_projection_mode
            .to_possible_value()
            .expect("invalid d4 projection mode")
            .get_name()
            .to_owned()
    } else {
        "counting".to_owned()
    };
    let args = vec![
        OsString::from("-i"),
        tmp.path().as_os_str().to_owned(),
        OsString::from("-m"),
        OsString::from(&d4_mode),
    ];
    log_invoked_command(&d4_path, &args);
    log(&format!(
        "[EXEC] starting #SAT solver process using executable {}",
        d4_path
    ));
    let _timing = scope("EXEC", "#SAT solve via d4");
    let child = Command::new(&d4_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect(&format!(
            "failed to run model counter '{d4_path}'. \
             Make sure d4 is installed, or specify a custom solver with --sharp-sat-path"
        ));
    let Some(stdout) = wait_with_timeout(child, "d4".to_owned()) else {
        return BigInt::from(-1i32);
    };
    let count_line = stdout
        .lines()
        .find(|line| line.starts_with("s "))
        .expect(&format!(
            "d4 output missing solution count (expected 's <count>'):\n{stdout}"
        ));
    let count_str = count_line.split_at(2).1;
    BigInt::from_str(count_str).expect(&format!(
        "d4 output contains invalid model count: '{count_str}'"
    ))
}

/// Counts solutions using an arbitrary #SAT solver.
///
/// The solver is invoked with the CNF file path as the first argument and "projected" or
/// "counting" as the second argument, indicating whether projected model counting is requested.
/// When projected, the CNF is annotated with "c t pmc" and "c p show" lines.
/// Output must contain a line starting with "s " followed by the model count.
/// It is not strictly necessary that the solver supports projected model counting, but it will then yield inaccurate results whenever projection is requested.
/// Returns -1 if the configured timeout (--sharp-sat-timeout) is exceeded.
fn arbitrary_sharp_sat(cnf: &str, solver_path: &str, projected: bool) -> BigInt {
    let mut tmp = NamedTempFile::new().expect("failed to create temporary file");
    write!(tmp, "{}", cnf).expect("failed to write CNF to temporary file");
    let resolved_path = path(solver_path);
    let args = vec![
        tmp.path().as_os_str().to_owned(),
        OsString::from(if projected { "projected" } else { "counting" }),
    ];
    log_invoked_command(&resolved_path, &args);
    log(&format!(
        "[EXEC] starting custom #SAT solver process using executable {}",
        resolved_path
    ));
    let _timing = scope("EXEC", "#SAT solve via custom solver");
    let child = Command::new(&resolved_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect(&format!(
            "failed to run #SAT solver '{resolved_path}'. \
             Check that --sharp-sat-path is correct and the solver is executable"
        ));
    let Some(stdout) = wait_with_timeout(child, "#SAT solver".to_owned()) else {
        return BigInt::from(-1i32);
    };
    let count_line = stdout
        .lines()
        .find(|line| line.starts_with("s "))
        .expect(&format!(
            "#SAT solver output missing count (expected 's <count>'):\n{stdout}"
        ));
    let count_str = count_line.split_at(2).1;
    BigInt::from_str(count_str).expect(&format!(
        "#SAT solver output contains invalid model count: '{count_str}'"
    ))
}

/// Enumerates all solutions of some CNF in DIMACS format.
///
/// Runs an external AllSAT solver, which is only suitable for formulas with few solutions.
/// This does not currently output solutions for fully indeterminate (i.e., unconstrained) variables.
pub(crate) fn bc_minisat_all(cnf: &str) -> (impl Iterator<Item = Vec<VarId>>, NamedTempFile) {
    let mut tmp_in = NamedTempFile::new().expect("failed to create temporary file");
    write!(tmp_in, "{}", cnf).expect("failed to write CNF to temporary file");
    let solver_path = path(&options().tool.bc_minisat_all_path);
    let args = vec![tmp_in.path().as_os_str().to_owned()];
    log_invoked_command(&solver_path, &args);
    log(&format!(
        "[EXEC] starting AllSAT solver process using executable {}",
        solver_path
    ));
    let _timing = scope("EXEC", "AllSAT solve");
    let process = Command::new(&solver_path)
        .args(&args)
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
                .map(|s| {
                    s.parse()
                        .expect(&format!("invalid literal in bc_minisat_all output: '{s}'"))
                })
                .filter(|literal| *literal != 0)
                .collect::<Vec<VarId>>()
        });
    (iter, tmp_in)
}

/// Converts a given feature-model file from one format into another.
///
/// Runs the tool FeatureIDE using the Java runtime environment, which is assumed to be available on the PATH variable.
pub(crate) fn io(file: &File, output_format: &str, variables: &[&str]) -> File {
    let io_path = path(&options().tool.io_path);
    let args = vec![
        OsString::from("-jar"),
        OsString::from(&io_path),
        OsString::from(&file.name),
        OsString::from(output_format),
        OsString::from(variables.join(",")),
    ];
    log_invoked_command("java", &args);
    log(&format!(
        "[EXEC] starting FeatureIDE conversion from {} to {} using {}",
        file.name, output_format, io_path
    ));
    if variables.len() > 0 {
        log(&format!(
            "[EXEC] projecting onto {} variables",
            variables.len()
        ));
    }
    let _timing = scope("EXEC", "FeatureIDE conversion");
    let process = Command::new("java")
        .args(&args)
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
        if error.is_empty() {
            "no output produced"
        } else {
            &error
        }
    );
    File::new(format!("-.{}", output_format), output)
}

/// Classifies the edit between two feature-model files using FeatureIDE's ModelComparator.
///
/// Returns one of "Refactoring", "Specialization", "Generalization", or "ArbitraryEdit".
/// Both files are written to temporary files so FeatureIDE can load them by path.
pub(crate) fn io_compare(file_a: &File, file_b: &File) -> String {
    let io_path = path(&options().tool.io_path);
    let ext = |f: &File| {
        std::path::Path::new(&f.name)
            .extension()
            .and_then(|e| e.to_str())
            .expect("file passed to io_compare has no extension")
            .to_owned()
    };
    let mut tmp_a = tempfile::Builder::new()
        .suffix(&format!(".{}", ext(file_a)))
        .tempfile()
        .expect("failed to create temporary file for FeatureIDE comparison");
    let mut tmp_b = tempfile::Builder::new()
        .suffix(&format!(".{}", ext(file_b)))
        .tempfile()
        .expect("failed to create temporary file for FeatureIDE comparison");
    write!(tmp_a, "{}", file_a.contents).expect("failed to write to temporary file");
    write!(tmp_b, "{}", file_b.contents).expect("failed to write to temporary file");
    let args = vec![
        OsString::from("-jar"),
        OsString::from(&io_path),
        tmp_a.path().as_os_str().to_owned(),
        OsString::from("compare"),
        tmp_b.path().as_os_str().to_owned(),
    ];
    log_invoked_command("java", &args);
    log(&format!(
        "[EXEC] starting FeatureIDE ModelComparator using {}",
        io_path
    ));
    let _timing = scope("EXEC", "FeatureIDE comparison");
    let process = Command::new("java")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect(&format!(
            "failed to run FeatureIDE I/O interface '{io_path}'. \
             Make sure Java is installed and on PATH"
        ));
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
    assert!(error.is_empty(), "FeatureIDE comparison failed: {}", error);
    output.trim().to_owned()
}
