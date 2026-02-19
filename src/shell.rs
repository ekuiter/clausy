//! Imperative shell for operating on feature-model formulas.

use crate::core::file::File;
use crate::core::formula::DiffKind;
use crate::parser::sat_inline::SatInlineFormulaParser;
use crate::{
    core::{arena::Arena, formula::Formula},
    parser::{parser, FormulaParsee},
};
use clap::{Args, Parser};
use std::env;
use std::process;
use std::sync::OnceLock;

/// Transforms feature-model formulas into CNF.
#[derive(Parser)]
#[command(name = "clausy")]
#[command(version)]
#[command(after_help = r#"A few notes on these options:
- External tools are only used by certain commands. CNF transformation does not require any external tools on built-in formats.
- The only exception are non-standard formats, which require a Java runtime environment in the PATH and a correct --io-path.
- All tool paths can be absolute or relative. Relative paths are resolved against the working directory and the clausy executable directory.
- For each solver class, we support one tool out of the box (e.g., kissat for SAT solving).
  You can override this with an arbitrary tool (e.g., using --sat-path), which has to conform to the specified I/O conventions.
- The config file clausy.conf next to the clausy executable can be used to set recurring default options."#)]
struct CliOptions {
    /// Input file and commands to run on the formula.
    /// Use "-" for stdin, or provide a file followed by commands like "to_cnf_dist", "print", etc.
    #[arg(trailing_var_arg = true)]
    commands: Vec<String>,

    #[command(flatten)]
    tool_paths: ToolPathOptions,

    #[command(flatten)]
    output_options: OutputOptions,
}

/// Paths to external tools used for SAT solving, model counting, etc.
///
/// Paths are looked up based on these strings. The supplied paths can be absolute or relative.
/// Relative paths are first resolved against the working directory, and then against the directory of the clausy executable.
#[derive(Args, Default, Debug)]
#[command(next_help_heading = "Tool Path Options")]
pub struct ToolPathOptions {
    /// Path to the satisfiability solver kissat
    #[arg(long = "kissat-path", default_value = "kissat")]
    pub kissat: String,

    /// Path to a satisfiability solver that takes a .cnf file and outputs "s [UN]SATISFIABLE"
    #[arg(long = "sat-path")]
    pub sat: Option<String>,

    /// Path to the model counter d4
    #[arg(long = "d4-path", default_value = "d4")]
    pub d4: String,

    /// Path to a model counter that takes a .cnf file and outputs "s <model count>"
    #[arg(long = "sharp-sat-path")]
    pub sharp_sat: Option<String>,

    /// Path to the AllSAT solver bc_minisat_all
    #[arg(long = "bc-minisat-all-path", default_value = "bc_minisat_all")]
    pub bc_minisat_all: String,

    /// Path to the FeatureIDE I/O interface
    #[arg(long = "io-path", default_value = "io.jar")]
    pub io: String,
}

/// Output formatting options.
#[derive(Args, Default, Debug)]
#[command(next_help_heading = "Output Options")]
pub struct OutputOptions {
    /// Print expression identifiers, useful when debugging
    #[arg(long, default_value_t = false)]
    pub print_ids: bool,

    /// Prefix for auxiliary variables introduced by Tseitin transformation
    #[arg(long, default_value = "_aux_")]
    pub aux_prefix: String,
}

/// All configuration options.
#[derive(Default, Debug)]
pub struct Options {
    pub tool_paths: ToolPathOptions,
    pub output: OutputOptions,
}

/// Global storage for options.
static OPTIONS: OnceLock<Options> = OnceLock::new();

/// Get options, using defaults if not explicitly initialized.
pub fn options() -> &'static Options {
    OPTIONS.get_or_init(Options::default)
}

/// Name of the config file that can be placed next to the executable.
/// 
/// This is hardcoded and cannot be configured, because at the point of loading the config file,
/// we already need to know its name.
const CONFIG_FILE: &str = "clausy.conf";

/// Exit code for unsatisfiable formulas.
/// 
/// This allows distinguishing between unsatisfiability and other errors.
const UNSAT_EXIT_CODE: i32 = 20;

/// Loads default arguments from a config file next to the executable.
///
/// The config file (`clausy.conf`) contains whitespace-separated arguments,
/// which are inserted before user-provided arguments (so CLI overrides config).
/// Lines beginning with `#` are treated as comments and ignored.
/// This is useful for providing platform-specific defaults via the Makefile.
fn load_config_file_args() -> Vec<String> {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(CONFIG_FILE)))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| {
            s.lines()
                .filter(|line| !line.trim_start().starts_with('#'))
                .flat_map(|line| line.split_whitespace())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Returns the most recently parsed formula.
macro_rules! formula {
    ($formulas:expr) => {
        $formulas.last_mut().unwrap()
    };
}

/// Converts a formula into its clause representation, if not done yet.
macro_rules! clauses {
    ($clauses:expr, $arena:expr, $formulas:expr) => {{
        if $clauses.is_none() {
            $clauses = Some(formula!($formulas).to_clauses(&$arena));
        }
        $clauses.as_ref().unwrap()
    }};
}

/// Main entry point.
///
/// Parses CLI arguments and runs each command in order.
pub fn main() {
    let mut args: Vec<String> = env::args().collect();
    let config_file_args = load_config_file_args();
    args.splice(1..1, config_file_args);
    let cli = CliOptions::parse_from(args);
    OPTIONS.set(Options {
        tool_paths: cli.tool_paths,
        output: cli.output_options,
    }).unwrap();

    let mut commands = cli.commands;
    let mut arena = Arena::new();
    let mut formulas = Vec::<Formula>::new();
    let mut clauses = None;
    if commands.is_empty() {
        commands.push("-".to_string());
    }
    if commands.len() == 1 && File::exists(&commands[0]) {
        commands.push("to_cnf_dist".to_string());
        commands.push("to_clauses".to_string());
        commands.push("print".to_string());
    }
    for command in &commands {
        let mut arguments: Vec<&str> = command.split_whitespace().collect();
        let action = arguments[0];
        arguments.remove(0);
        match action {
            "print" => {
                if clauses.is_some() {
                    print!("{}", clauses.as_ref().unwrap());
                } else {
                    println!("{}", formula!(formulas).as_ref(&arena));
                };
            }
            "print_sub_exprs" => {
                for id in formula!(formulas).sub_exprs(&mut arena) {
                    println!("{}", arena.as_formula(id).as_ref(&arena));
                }
            }
            "to_canon" => formula!(formulas).to_canon(&mut arena),
            "to_nnf" => formula!(formulas).to_nnf(&mut arena),
            "to_cnf_dist" => formula!(formulas).to_cnf_dist(&mut arena),
            "to_cnf_tseitin" => {
                formula!(formulas).to_cnf_tseitin(true, &mut arena);
            }
            "to_clauses" => clauses = Some(formula!(formulas).to_clauses(&mut arena)),
            "satisfy" => {
                if let Some(solution) = clauses!(clauses, arena, formulas).satisfy() {
                    eprintln!("s SATISFIABLE");
                    println!("c {solution}");
                } else {
                    eprintln!("s UNSATISFIABLE");
                    process::exit(UNSAT_EXIT_CODE);
                }
            }
            "count" => println!("{}", clauses!(clauses, arena, formulas).count()),
            "assert_count" => {
                let clauses = clauses!(clauses, arena, formulas);
                formula!(formulas)
                    .file
                    .as_ref()
                    .unwrap()
                    .assert_count(clauses);
            }
            "enumerate" => clauses!(clauses, arena, formulas).enumerate(),
            "count_inc" => {
                let [a, b] = &formulas[..] else { panic!() };
                println!("{}", a.count_inc(b, arguments.into_iter().next(), &mut arena));
            }
            "diff" => {
                let [a, b] = &formulas[..] else { panic!() };
                let mut arguments = arguments.into_iter();
                let mut parse_argument = || match arguments.next() {
                    Some("top-strong") => DiffKind::Strong(true),
                    Some("bottom-strong") | Some("strong") => DiffKind::Strong(false),
                    Some("weak") | None => DiffKind::Weak,
                    _ => panic!()
                };
                a.diff(b, parse_argument(), parse_argument(), arguments.next(), &mut arena);
            }
            _ => {
                if File::exists(action) {
                    let file = File::read(action);
                    let extension = file.extension();
                    formulas.push(arena.parse(file, parser(extension)));
                } else if SatInlineFormulaParser::can_parse(command) {
                    formulas.push(
                        SatInlineFormulaParser::new(&formulas, Some(false))
                            .parse_into(&command, &mut arena),
                    );
                } else {
                    unreachable!();
                }
                clauses = None;
            }
        }
        #[cfg(debug_assertions)]
        {
            if formulas.last().is_some() {
                formulas.last_mut().unwrap().assert_canon(&mut arena);
            }
        }
    }
}
