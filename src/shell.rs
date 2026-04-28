//! Imperative shell for operating on feature-model formulas.

use crate::core::count_inc::count_inc;
use crate::core::diff::{diff, DiffKind, VarMap};
use crate::core::file::File;
use crate::parser::sat_inline::SatInlineFormulaParser;
use crate::{
    core::{arena::Arena, formula::Formula, var::VarId},
    parser::{parser, FormulaParsee},
    util::log::{log, scope},
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process;
use std::str::FromStr;
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
pub struct CliOptions {
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
    /// Path to the satisfiability solver kissat.
    #[arg(long, default_value = "kissat")]
    pub kissat_path: String,

    /// Path to a satisfiability solver that takes a .cnf file and outputs "s (UN)SATISFIABLE".
    #[arg(long)]
    pub sat_path: Option<String>,

    /// Path to the model counter d4.
    #[arg(long, default_value = "d4")]
    pub d4_path: String,

    /// d4 mode used when performing projected model counting.
    ///
    /// Has no effect on plain (non-projected) model counting, which always uses counting mode.
    /// Has no effect if another model counter than d4 is being used.
    #[arg(long, default_value = "counting")]
    pub d4_projection_mode: D4ProjectionMode,

    /// Path to a model counter that takes a .cnf file and outputs "s 'model count'".
    ///
    /// The solver is invoked with the CNF file path as the first argument and "projected" or
    /// "counting" as the second argument, indicating whether projected model counting is requested.
    /// The CNF is annotated with "c t pmc" and "c p show" when projected.
    /// Output must contain a line starting with "s " followed by the model count.
    #[arg(long)]
    pub sharp_sat_path: Option<String>,

    /// Timeout in seconds for model counting (0 = no timeout).
    ///
    /// If the model counter (either d4 or one set with `--sharp_sat_path`) exceeds this time, it is killed and -1 is returned as the count.
    #[arg(long, default_value_t = 0)]
    pub sharp_sat_timeout: u64,

    /// Path to the AllSAT solver bc_minisat_all.
    #[arg(long, default_value = "bc_minisat_all")]
    pub bc_minisat_all_path: String,

    /// Path to the FeatureIDE I/O interface.
    #[arg(long, default_value = "io.jar")]
    pub io_path: String,

    /// Force parsing all input files through the FeatureIDE I/O interface.
    #[arg(long)]
    pub force_io: bool,
}

/// Supported d4 projection modes for projected model counting.
#[derive(Clone, Copy, Debug, ValueEnum, Default)]
pub enum D4ProjectionMode {
    /// Model counting (d4).
    ///
    /// Default implementation for model counting in d4
    /// implemented [here](https://github.com/SoftVarE-Group/d4v2/blob/main/src/methods/DpllStyleMethod.hpp).
    /// Also supports projected model counting.
    #[default]
    #[value(name = "counting")]
    Counting,

    /// Projected d-DNNF compilation (pd4).
    ///
    /// See "Efficient Slicing of Feature Models via Projected d-DNNF Compilation" by Sundermann et al.
    /// implemented [here](https://github.com/SoftVarE-Group/d4v2/blob/main/src/methods/ProjDpllStyleMethod.hpp).
    /// Performs particularly well for projected model counting.
    #[value(name = "proj-ddnnf-compiler")]
    ProjDdnnfCompiler,

    /// Projected model counting (projMC).
    ///
    /// See "A Recursive Algorithm for Projected Model Counting" by J.-M. Lagniez and P. Marquis
    /// implemented [here](https://github.com/SoftVarE-Group/d4v2/blob/main/src/methods/ProjMCMethod.hpp).
    /// Seems to perform overall less well for projected model counting than the other two modes.
    #[value(name = "projMC")]
    ProjMc,
}

/// Output formatting options.
#[derive(Args, Default, Debug)]
#[command(next_help_heading = "Output Options")]
pub struct OutputOptions {
    /// Print expression identifiers, useful when debugging
    #[arg(long)]
    pub print_ids: bool,

    /// Prefix for auxiliary variables introduced by Tseitin transformation
    #[arg(long, default_value = "_aux_")]
    pub aux_prefix: String,

    /// Disable optional SAT-style informational logs (`c ...`)
    #[arg(short = 'q', long)]
    pub quiet: bool,
}

/// Supported transformations.
#[derive(Clone, Debug, ValueEnum)]
pub enum Transform {
    Canon,
    Nnf,
    #[value(name = "cnf-dist", alias = "dist")]
    CnfDist,
    #[value(name = "cnf-tseitin", alias = "tseitin")]
    CnfTseitin,
}

/// Top-level actions.
#[derive(Subcommand, Debug)]
pub enum Action {
    /// Print the transformed formula as CNF clauses (default).
    #[command(alias = "print")]
    PrintClauses(ProjectionArgs),

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
    Count(ProjectionArgs),

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

/// Options for projection-aware actions (`print-clauses`, `count`).
#[derive(Args, Debug, Default)]
pub struct ProjectionArgs {
    /// Variables to project to (comma-separated).
    #[arg(
        long = "project",
        value_name = "VAR",
        num_args = 1..,
        value_delimiter = ',',
        conflicts_with_all = ["slice", "project_file", "slice_file"]
    )]
    project: Vec<String>,

    /// Variables to slice away (comma-separated).
    #[arg(
        long = "slice",
        value_name = "VAR",
        num_args = 1..,
        value_delimiter = ',',
        conflicts_with_all = ["project", "project_file", "slice_file"]
    )]
    slice: Vec<String>,

    /// Path to a newline-separated file of variable names to project to.
    #[arg(
        long = "project-file",
        value_name = "FILE",
        conflicts_with_all = ["project", "slice", "slice_file"]
    )]
    project_file: Option<String>,

    /// Path to a newline-separated file of variable names to slice away.
    #[arg(
        long = "slice-file",
        value_name = "FILE",
        conflicts_with_all = ["project", "slice", "project_file"]
    )]
    slice_file: Option<String>,
}

/// Options for `count-inc`.
#[derive(Args, Debug)]
pub struct CountIncArgs {
    /// Optional known model count for the left formula.
    left_model_count: Option<String>,
}

/// Diff mode for one side.
///
/// Accepted values:
/// - `slice` — slice both formulas to their common variables
/// - `true` — fix all foreign variables to true
/// - `false` — fix all foreign variables to false
/// - `true,<true-file>,<false-file>` — fix foreign variables to true by default, with per-variable
///   overrides from newline-separated files (empty path = no overrides for that set)
/// - `false,<true-file>,<false-file>` — fix foreign variables to false by default, with per-variable
///   overrides from newline-separated files (empty path = no overrides for that set)
/// In the literature, `false` is typically used.
#[derive(Clone, Debug)]
pub enum DiffMode {
    Slice,
    Fixed {
        default: bool,
        true_file: Option<String>,
        false_file: Option<String>,
    },
}

impl FromStr for DiffMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || {
            format!(
                "invalid diff mode '{}'; expected 'slice', 'true', 'false', \
             'true,<true-file>,<false-file>', or 'false,<true-file>,<false-file>'",
                s
            )
        };
        match s {
            "slice" => Ok(DiffMode::Slice),
            "true" => Ok(DiffMode::Fixed {
                default: true,
                true_file: None,
                false_file: None,
            }),
            "false" => Ok(DiffMode::Fixed {
                default: false,
                true_file: None,
                false_file: None,
            }),
            _ => {
                let parts: Vec<&str> = s.splitn(3, ',').collect();
                if parts.len() == 3 {
                    let default = match parts[0] {
                        "true" => true,
                        "false" => false,
                        _ => return Err(err()),
                    };
                    Ok(DiffMode::Fixed {
                        default,
                        true_file: if parts[1].is_empty() {
                            None
                        } else {
                            Some(parts[1].to_string())
                        },
                        false_file: if parts[2].is_empty() {
                            None
                        } else {
                            Some(parts[2].to_string())
                        },
                    })
                } else {
                    Err(err())
                }
            }
        }
    }
}

fn parse_diff_mode(s: &str) -> Result<DiffMode, String> {
    s.parse()
}

impl DiffMode {
    fn into_diff_kind(self, arena: &mut Arena) -> DiffKind {
        match self {
            DiffMode::Slice => DiffKind::Slice,
            DiffMode::Fixed {
                default,
                true_file,
                false_file,
            } => {
                let core_vars = true_file
                    .map(|f| resolve_named_var_ids(arena, &read_variable_name_file(&f)))
                    .unwrap_or_default();
                let dead_vars = false_file
                    .map(|f| resolve_named_var_ids(arena, &read_variable_name_file(&f)))
                    .unwrap_or_default();
                DiffKind::Fixed {
                    default,
                    core_vars,
                    dead_vars,
                }
            }
        }
    }
}

/// Options for `diff`.
#[derive(Args, Debug)]
pub struct DiffArgs {
    /// Left-side diff mode.
    ///
    /// By default, we use `slice` (i.e., project onto common variables).
    #[arg(long, default_value = "false", value_parser = parse_diff_mode)]
    left: DiffMode,

    /// Right-side diff mode.
    ///
    /// By default, we use `slice` (i.e., project onto common variables).
    #[arg(long, default_value = "false", value_parser = parse_diff_mode)]
    right: DiffMode,

    /// Perform model counting.
    ///
    /// With this flag, we quantify the differences between both formulas with a regular (non-projected) model counter.
    /// If the left or right [DiffMode] requests slicing, we perform it in a preliminary and separate step with FeatureIDE.
    /// FeatureIDE uses resolution-based slicing, which internally uses a distributive transformation to establish CNF,
    /// so it does not generally scale to complex formulas.
    /// Can be omitted, e.g., if only serialization of differences is requested.
    #[arg(long, conflicts_with_all = ["satisfy"])]
    count: bool,

    /// Perform projected model counting.
    ///
    /// With this flag, we quantify the differences between both formulas with a projected model counter.
    /// That means we perform slicing and counting in one combined step, skipping FeatureIDE.
    /// Incompatible with the `--uvl` and `--xml` serialization options,
    /// as our current architecture relies on counting and serialization being performed in separate steps.
    #[arg(long, conflicts_with_all = ["count", "satisfy", "uvl", "xml"])]
    projected_count: bool,

    /// Perform SAT-based classification.
    ///
    /// Classifies the difference between both formulas as Refactoring, Specialization,
    /// Generalization, or ArbitraryEdit using two SAT queries (one for removed solutions,
    /// one for added solutions) instead of full model counting, thus omitting a fine-grained quantification.
    /// This is the base algorithm proposed by Thüm et al. 2009 in "Reasoning about Edits to Feature Models".
    /// Requires negation-based reasoning (--negate), as only model counters allow for avoiding negation.
    /// Conflicts with quantification (--count and --projected-count).
    /// The utility of this algorithm is quite limited, because most edits are arbitrary in practice.
    /// However, SAT-based scales much better than counting-based methods, and is especially useful for tiny edits.
    #[arg(long, requires = "negate")]
    satisfy: bool,

    /// Use simplified reasoning for SAT-based classification.
    ///
    /// Instead of only two SAT queries, check each unique clause of the other individually
    /// and terminate early on the first satisfiable query.
    /// This is the improved algorithm proposed by Thüm et al. 2009 in "Reasoning about Edits to Feature Models".
    /// It was developed to mitigate the limited scalability of distributive transformation, and requires it.
    /// It is included here mostly for evaluation purposes.
    #[arg(long, requires = "satisfy", requires = "cnf_dist")]
    simplified: bool,

    /// Use distributive CNF transformation instead of Tseitin transformation.
    ///
    /// With this flag, all intermediate formulas are transformed using the distributive CNF transformation.
    /// Use with caution because these formulas will blow up exponentially if using negation-based reasoning.
    /// Even without negation-based reasoning, this can easily blow up on larger formulas.
    /// By default, we use the Tseitin transformation to avoid such blowup.
    /// This does not affect the correctness of the results, only the scalability.
    #[arg(long, alias = "dist")]
    cnf_dist: bool,

    /// Use negation-based reasoning.
    ///
    /// With this flag, negations will be used to construct intermediate difference-encoding formulas.
    /// That is, this will encode removed solutions by negating the right formula, and vice versa.
    /// By default, negations are avoided by computing the number of removed and added solutions from other counted formulas.
    /// This default only works with for pure quantification tasks that scale in terms of model counting.
    /// In contrast, to serialize difference models or apply SAT-based classification, negation-based reasoning is needed.
    #[arg(long)]
    negate: bool,

    /// Report results even if they may be incorrect.
    ///
    /// With this flag, results will be reported even when they are not guaranteed to be mathematically incorrect.
    /// This is known to happen in the following case:
    /// Projected model counting returns incorrect results for several computations when combined with negation-based reasoning.
    /// By default, such possibly incorrect results are omitted from the output.
    /// This flag is intended for evaluations that assess the deviation from the correct result.
    /// Not called `unsafe` here because it clashes with a Rust keyword.
    #[arg(long = "unsafe", requires = "projected_count", requires = "negate")]
    is_unsafe: bool,

    /// Output prefix for optional differencing artifacts.
    ///
    /// This prefix is prepended to all differencing artifact file names.
    /// When the prefix contains a `/`, the directory portion is created, if it does not exist.
    /// If no prefix is given, no differencing artifacts are written.
    #[arg(long)]
    output: Option<String>,

    /// Write variable list files.
    #[arg(long, requires = "output")]
    variables: bool,

    /// Write constraint list files.
    #[arg(long, requires = "output")]
    constraints: bool,

    /// Serialize formula differences as UVL.
    #[arg(long, requires = "output", requires = "negate")]
    uvl: bool,

    /// Serialize formula differences as XML.
    #[arg(long, requires = "output", requires = "negate")]
    xml: bool,

    /// Write intermediate formula and clause representation files.
    #[arg(long, requires = "output")]
    cnf: bool,

    /// Suppress the CSV header line in the output.
    #[arg(long)]
    no_header: bool,

    /// Path to a variable mapping file for handling renamed, split, or merged variables.
    ///
    /// Each non-empty, non-comment line must have the form `left=right`,
    /// where `left` and `right` are comma-separated variable name lists.
    /// Exactly one side must be a single variable name:
    /// - `a=b`: rename (`a` in the left formula corresponds to `b` in the right formula).
    /// - `a=b,c`: split (`a` in the left formula was split into `b` and `c` in the right formula).
    /// - `a,b=c`: merge (`a` and `b` in the left formula were merged into `c` in the right formula).
    /// Lines beginning with `#` are treated as comments and ignored.
    /// For the semantics of this mapping, see [crate::core::diff::apply_var_maps].
    #[arg(long)]
    variable_map: Option<String>,
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

/// Returns the most recently loaded formula.
///
/// This is the formula that subsequent transformations and most actions operate on.
fn current_formula(formulas: &mut [Formula]) -> &mut Formula {
    formulas
        .last_mut()
        .expect("no formula loaded; provide at least one --input item")
}

/// Parses all `--input` items into formulas within a shared arena.
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
            log("[SHELL] forcing foreign variables across all formulas to false, if any");
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

/// Returns whether a command ignores transform flags.
fn ignores_transforms(action: &Action) -> bool {
    matches!(action, Action::Diff(_) | Action::CountInc(_))
}

/// Applies the selected transformation pipeline to the current formula.
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

/// Reads newline-separated variable names from a text file.
fn read_variable_name_file(path: &str) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read variable-name file '{}': {}", path, e))
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

/// Parses a variable mapping file into a list of [VarMap] entries.
///
/// Each non-empty, non-comment line must be of the form `left=right`,
/// where `left` and `right` are comma-separated variable name lists.
/// Exactly one side must contain a single variable name; see [VarMap] for details.
fn parse_variable_map_file(path: &str) -> Vec<VarMap> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read variable-mapping file '{}': {}", path, e))
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (left_str, right_str) = line.split_once('=').unwrap_or_else(|| {
                panic!(
                    "invalid variable-mapping line '{}': expected 'left=right' format",
                    line
                )
            });
            let left: Vec<String> = left_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let right: Vec<String> = right_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            assert!(
                !left.is_empty() && !right.is_empty(),
                "variable-mapping line '{}' must have at least one variable on each side",
                line
            );
            assert!(
                left.len() == 1 || right.len() == 1,
                "variable-mapping line '{}' must have exactly one variable on at least one side",
                line
            );
            VarMap { left, right }
        })
        .collect()
}

/// Resolves variable names to arena variable identifiers.
fn resolve_named_var_ids(arena: &mut Arena, names: &[String]) -> HashSet<VarId> {
    names
        .iter()
        .map(|name| {
            arena
                .get_var_named(name.clone())
                .unwrap_or_else(|| panic!("unknown variable name in projection/slice: '{}'", name))
        })
        .collect()
}

/// Computes the projection variable set for projected model counting.
///
/// Returns `None` if projected model counting is not needed.
/// If the projection variable set is empty, the project model counter will decide satisfiability.
fn count_projection_vars(
    args: &ProjectionArgs,
    formula: &Formula,
    arena: &mut Arena,
) -> Option<HashSet<VarId>> {
    let source: Option<(Vec<String>, bool)> = if !args.project.is_empty() {
        Some((args.project.clone(), false))
    } else if !args.slice.is_empty() {
        Some((args.slice.clone(), true))
    } else if let Some(file) = &args.project_file {
        Some((read_variable_name_file(file), false))
    } else if let Some(file) = &args.slice_file {
        Some((read_variable_name_file(file), true))
    } else {
        None
    };

    let (variable_names, is_slice) = source?;
    let variable_ids = resolve_named_var_ids(arena, &variable_names);
    let projection = if is_slice {
        formula
            .sub_var_ids
            .difference(&variable_ids)
            .copied()
            .collect::<HashSet<VarId>>()
    } else {
        variable_ids
    };
    if projection == formula.sub_var_ids {
        None
    } else {
        Some(projection)
    }
}

/// Executes the selected top-level action on parsed/transformed formulas.
fn execute_action(action: Action, formulas: &mut [Formula], arena: &mut Arena) {
    let action_name = format!("{action:?}");
    let action_name = action_name.split('(').next().unwrap_or(&action_name);
    let _timing = scope("SHELL", &format!("action {action_name}"));
    match action {
        Action::PrintClauses(args) => {
            let formula = current_formula(formulas);
            let clauses = formula.to_clauses(arena);
            let projection = count_projection_vars(&args, formula, arena);
            if let Some(proj_vars) = projection {
                print!("{}", clauses.to_projected_string(&proj_vars));
            } else {
                print!("{}", clauses);
            }
        }
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
        Action::Count(args) => {
            let formula = current_formula(formulas);
            let clauses = formula.to_clauses(arena);
            let projection = count_projection_vars(&args, formula, arena);
            if let Some(proj_vars) = projection {
                println!("{}", clauses.proj_count(&proj_vars));
            } else {
                println!("{}", clauses.count());
            }
        }
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
            let [a, b] = formulas else {
                panic!("this command requires exactly two loaded formulas");
            };
            println!(
                "{}",
                count_inc(a, b, args.left_model_count.as_deref(), arena)
            );
        }
        Action::Diff(args) => {
            let var_maps = args
                .variable_map
                .as_deref()
                .map(parse_variable_map_file)
                .unwrap_or_default();
            let [a, b] = formulas else {
                panic!("this command requires exactly two loaded formulas");
            };
            diff(
                a,
                b,
                &var_maps,
                args.left.into_diff_kind(arena),
                args.right.into_diff_kind(arena),
                args.output.as_deref(),
                args.count,
                args.projected_count,
                args.satisfy,
                args.simplified,
                args.variables,
                args.constraints,
                args.uvl,
                args.xml,
                args.cnf,
                args.no_header,
                args.cnf_dist,
                args.is_unsafe,
                args.negate,
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
        Action::PrintClauses(ProjectionArgs::default())
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
