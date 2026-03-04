//! Imperative shell for operating on feature-model formulas.

use crate::core::file::File;
use crate::core::formula::DiffKind;
use crate::parser::sat_inline::SatInlineFormulaParser;
use crate::{
    core::{arena::Arena, formula::Formula},
    parser::{parser, FormulaParsee},
    util::log::{log, scope},
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::env;
use std::process;
use std::sync::OnceLock;

/// Transforms feature-model formulas into CNF.
#[derive(Parser)]
#[command(name = "clausy")]
#[command(version)]
#[command(after_help = r#"A few notes on these options:
- Inputs are supplied via repeated --input/-i options, and parsed in order.
- Transformations are supplied via repeated --transform/-t options, and applied in order.
- All --input options are processed before all --transform options, no matter how they are interleaved.
  The --transform options are applied to the final --input (if not overridden by the specified command).
- External tools are only used by certain commands. CNF transformation does not require any external tools on built-in formats.
- The only exception are non-standard formats, which require a Java runtime environment in the PATH and a correct --io-path.
- All tool paths can be absolute or relative. Relative paths are resolved against the working directory and the clausy executable directory.
- For each solver class, we support one tool out of the box (e.g., kissat for SAT solving).
  You can override this with an arbitrary tool (e.g., using --sat-path), which has to conform to the specified I/O conventions.
- The config file clausy.conf next to the clausy executable can be used to set recurring default options."#)]
struct CliOptions {
    /// Input item (file path, stdin with - or -.extension, or inline .sat expression)
    #[arg(
        short = 'i',
        long = "input",
        action = clap::ArgAction::Append,
        value_name = "INPUT",
        allow_hyphen_values = true
    )]
    inputs: Vec<String>,

    /// Transformation step
    #[arg(
        short = 't',
        long = "transform",
        action = clap::ArgAction::Append,
        value_name = "TRANSFORM"
    )]
    transforms: Vec<Transform>,

    #[command(subcommand)]
    action: Option<Action>,

    #[command(flatten)]
    tool_options: ToolOptions,

    #[command(flatten)]
    output_options: OutputOptions,
}

/// Options of external tools used for SAT solving, model counting, etc.
///
/// Most options here are paths that define where tools are looked up.
/// The supplied paths can be absolute or relative.
/// Relative paths are first resolved against the working directory, and then against the directory of the clausy executable.
/// Some options like `--force-io` control specific tool behavior.
#[derive(Args, Default, Debug)]
#[command(next_help_heading = "Tool Options")]
pub struct ToolOptions {
    /// Path to the satisfiability solver kissat
    #[arg(long = "kissat-path", default_value = "kissat")]
    pub kissat: String,

    /// Path to a satisfiability solver that takes a .cnf file and outputs "s (UN)SATISFIABLE"
    #[arg(long = "sat-path")]
    pub sat: Option<String>,

    /// Path to the model counter d4
    #[arg(long = "d4-path", default_value = "d4")]
    pub d4: String,

    /// Path to a model counter that takes a .cnf file and outputs "s 'model count'"
    #[arg(long = "sharp-sat-path")]
    pub sharp_sat: Option<String>,

    /// Path to the AllSAT solver bc_minisat_all
    #[arg(long = "bc-minisat-all-path", default_value = "bc_minisat_all")]
    pub bc_minisat_all: String,

    /// Path to the FeatureIDE I/O interface
    #[arg(long = "io-path", default_value = "io.jar")]
    pub io: String,

    /// Force parsing all input files through the FeatureIDE I/O interface
    #[arg(long, default_value_t = false)]
    pub force_io: bool,
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

    /// Disable optional SAT-style informational logs (`c ...`)
    #[arg(short = 'q', long, default_value_t = false)]
    pub quiet: bool,
}

/// Supported transformations.
#[derive(Clone, Debug, ValueEnum)]
enum Transform {
    Canon,
    Nnf,
    #[value(name = "cnf-dist", alias = "dist")]
    CnfDist,
    #[value(name = "cnf-tseitin", alias = "tseitin")]
    CnfTseitin,
}

/// Top-level actions.
#[derive(Subcommand, Debug)]
enum Action {
    /// Print the transformed formula as CNF clauses (default).
    #[command(alias = "print")]
    PrintClauses,

    /// Print the transformed formula expression.
    PrintFormula,

    /// Print all sub-expressions of the transformed formula.
    PrintSubExprs,

    /// Performs no action after parsing and transformation.
    ///
    /// This is useful for profiling, when no output is desired.
    Nop,

    /// Check satisfiability and print a satisfying assignment, if any.
    ///
    /// This calls an external SAT solver as specified with `--kissat-path` or `--sat-path`.
    /// If set with `--sat-path`, no satisfying assignment will be printed.
    Satisfy,

    /// Count satisfying assignments.
    ///
    /// This calls an external model counter as specified with `--d4-path` or `--sharp-sat-path`.
    /// If a model counter set with `--sharp-sat-path` does not support projected model counting,
    /// `--projection` and `--slice` will be ignored and the model count will be computed normally.
    Count,

    /// Check model count against FeatureIDE as baseline.
    ///
    /// This can be used to validate the correctness of the CNF transformation, given that the transformation in FeatureIDE is correct.
    /// For this, the formula must originate from a file.
    /// This will not terminate for large formulas due to exponential blowup in FeatureIDE.
    AssertCount,

    /// Enumerate satisfying assignments.
    ///
    /// This calls an external AllSAT solver as specified with `--bc-minisat-all-path`.
    Enumerate,

    /// Compute incremental counting expression between two formulas.
    ///
    /// This expects exactly two formulas to be loaded with `--input`.
    /// All `--transform` options will be ignored by this command, which implements its own transformation logic.
    /// This will print an expression that, when evaluated, counts the number of models in the right formula.
    /// The expression takes as input the model count of the left formula and can be evaluated in Bash using `bc`.
    /// If the optional `left_model_count` is specified, the model count of the right formula will be computed instead of printing an expression.
    ///
    /// This was originally intended to speed up model counting of the right formula if the model count of the left formula is known.
    /// Intuitively, the difference between both formulas should be rather small, so the incremental counting expression should be more efficient to determine than counting the right formula from scratch.
    /// However, for a time series of exponentially growing formulas, the difference grows exponentially as well, so the performance benefit of this method is rather small.
    /// Still, the resulting expression can be useful, as it splits up the differences between both formulas in different terms.
    /// This idea is further developed in the `diff` command, of which this command is an early predecessor.
    CountInc(CountIncArgs),

    /// Compute difference between two formulas.
    ///
    /// This expects exactly two formulas to be loaded with `--input`.
    /// All `--transform` options will be ignored by this command, which implements its own transformation logic.
    Diff(DiffArgs),
}

/// Options for `count-inc`.
#[derive(Args, Debug)]
struct CountIncArgs {
    /// Optional known model count for the left formula.
    left_model_count: Option<String>,
}

/// Diff mode for one side.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum DiffMode {
    Weak,
    #[value(name = "top-strong")]
    TopStrong,
    #[value(name = "bottom-strong", alias = "strong")]
    BottomStrong,
}

impl DiffMode {
    fn into_diff_kind(self) -> DiffKind {
        match self {
            DiffMode::Weak => DiffKind::Weak,
            DiffMode::TopStrong => DiffKind::Strong(true),
            DiffMode::BottomStrong => DiffKind::Strong(false),
        }
    }
}

/// Options for `diff`.
#[derive(Args, Debug)]
struct DiffArgs {
    /// Left-side diff mode.
    #[arg(long, default_value = "weak")]
    left: DiffMode,

    /// Right-side diff mode.
    #[arg(long, default_value = "weak")]
    right: DiffMode,

    /// Optional output prefix for serialized diff artifacts.
    #[arg(long)]
    output_prefix: Option<String>,
}

/// All configuration options.
#[derive(Default, Debug)]
pub struct Options {
    pub tool: ToolOptions,
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

fn current_formula(formulas: &mut [Formula]) -> &mut Formula {
    formulas
        .last_mut()
        .expect("no formula loaded; provide at least one --input item")
}

fn formulas_pair(formulas: &[Formula]) -> (&Formula, &Formula) {
    let [a, b] = formulas else {
        panic!("this command requires exactly two loaded formulas");
    };
    (a, b)
}

fn parse_inputs(inputs: &[String], arena: &mut Arena) -> Vec<Formula> {
    let mut formulas = Vec::<Formula>::new();
    for input in inputs {
        let _timing = scope("SHELL", "parse input");
        if File::exists(input) {
            let file = File::read(input);
            let extension = file.extension();
            log(&format!(
                "[SHELL] parsing input file {} (detected format: {})",
                file.name,
                extension.as_deref().unwrap_or("no extension")
            ));
            formulas.push(arena.parse(file, parser(extension)));
        } else if SatInlineFormulaParser::can_parse(input) {
            log("[SHELL] parsing inline SAT expression from --input item");
            formulas
                .push(SatInlineFormulaParser::new(&formulas, Some(false)).parse_into(input, arena));
        } else {
            panic!(
                "{} is not an existing file, stdin token (- or -.<extension>), or parsable inline expression",
                input
            );
        }
    }
    formulas
}

fn ignores_transforms(action: &Action) -> bool {
    matches!(action, Action::Diff(_) | Action::CountInc(_))
}

fn apply_transforms(formulas: &mut [Formula], transforms: &[Transform], arena: &mut Arena) {
    for transform in transforms {
        let _timing = scope("SHELL", &format!("transform {:?}", transform));
        match transform {
            Transform::Canon => {
                current_formula(formulas).to_canon(arena);
                #[cfg(debug_assertions)]
                {
                    current_formula(formulas).assert_canon(arena);
                }
            }
            Transform::Nnf => current_formula(formulas).to_nnf(arena),
            Transform::CnfDist => current_formula(formulas).to_cnf_dist(arena),
            Transform::CnfTseitin => current_formula(formulas).to_cnf_tseitin(true, arena),
        }
    }
}

fn execute_action(action: Action, formulas: &mut [Formula], arena: &mut Arena) {
    let action_name = format!("{action:?}");
    let action_name = action_name.split('(').next().unwrap_or(&action_name);
    let _timing = scope("SHELL", &format!("action {action_name}"));
    match action {
        Action::PrintClauses => print!("{}", current_formula(formulas).to_clauses(arena)),
        Action::PrintFormula => println!("{}", current_formula(formulas).as_ref(arena)),
        Action::PrintSubExprs => {
            let sub_expr_ids = current_formula(formulas).sub_exprs(arena);
            for id in sub_expr_ids {
                println!("{}", arena.as_formula(id).as_ref(arena));
            }
        }
        Action::Nop => {}
        Action::Satisfy => {
            if let Some(solution) = current_formula(formulas).to_clauses(arena).satisfy() {
                eprintln!("s SATISFIABLE");
                println!("c {solution}");
            } else {
                eprintln!("s UNSATISFIABLE");
                process::exit(UNSAT_EXIT_CODE);
            }
        }
        Action::Count => println!("{}", current_formula(formulas).to_clauses(arena).count()),
        Action::AssertCount => {
            let formula = current_formula(formulas);
            let clauses = formula.to_clauses(arena);
            formula
                .file
                .as_ref()
                .expect("assert-count requires a formula parsed from a file")
                .assert_count(&clauses);
            log("[SHELL] asserted model count matches the expected value");
        }
        Action::Enumerate => current_formula(formulas).to_clauses(arena).enumerate(),
        Action::CountInc(args) => {
            let (a, b) = formulas_pair(formulas);
            println!(
                "{}",
                a.count_inc(b, args.left_model_count.as_deref(), arena)
            );
        }
        Action::Diff(args) => {
            let (a, b) = formulas_pair(formulas);
            a.diff(
                b,
                args.left.into_diff_kind(),
                args.right.into_diff_kind(),
                args.output_prefix.as_deref(),
                arena,
            );
        }
    }
}

/// Main entry point.
///
/// Parses .conf/CLI arguments and executes the structured pipeline of inputs, transforms, and an action.
pub fn main() {
    let mut args: Vec<String> = env::args().collect();
    let config_file_args = load_config_file_args();
    args.splice(1..1, config_file_args);
    let mut cli = CliOptions::parse_from(args);

    OPTIONS
        .set(Options {
            tool: cli.tool_options,
            output: cli.output_options,
        })
        .expect("global options were initialized more than once");
    log(&format!(
        "[SHELL] {} {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    ));

    if cli.inputs.is_empty() {
        log("[SHELL] no input specified, defaulting to - (stdin)");
        cli.inputs.push("-".to_string());
    }
    let action = if let Some(action) = cli.action {
        action
    } else {
        log("[SHELL] no action specified, defaulting to print-clauses");
        Action::PrintClauses
    };
    if ignores_transforms(&action) {
        if !cli.transforms.is_empty() {
            log("[SHELL] ignoring --transform options for this action");
        }
    } else if cli.transforms.is_empty() {
        log("[SHELL] no transform specified, defaulting to cnf-dist");
        cli.transforms.push(Transform::CnfDist);
    }

    let _timing = scope("SHELL", &format!("clausy"));
    let mut arena = Arena::new();
    let mut formulas = parse_inputs(&cli.inputs, &mut arena);
    if !ignores_transforms(&action) {
        apply_transforms(&mut formulas, &cli.transforms, &mut arena);
    }
    execute_action(action, &mut formulas, &mut arena);
}
