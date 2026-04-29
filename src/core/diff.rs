//! Differencing of feature-model formulas.

use crate::util::log::log;
use crate::util::{exec, io};
use num::{bigint::ToBigInt, BigInt, BigRational, Signed, ToPrimitive};
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    time::{Duration, Instant},
};

use super::{
    arena::Arena, clauses::Clauses, expr::Expr::And, file::File, formula::Formula, var::VarId,
};

// Print and flush information to standard output.
macro_rules! print_flush {
    ($($arg:tt)*) => {{
        print!($($arg)*);
        std::io::stdout().flush().unwrap();
    }};
}

// Print a new CSV column.
macro_rules! print_column {
    ($($arg:tt)*) => {{
        print!(",");
        print_flush!($($arg)*);
    }};
}

// Measure the time taken by an expression and print it.
macro_rules! measure_time {
    ($expr:expr) => {{
        let start = Instant::now();
        let result = $expr;
        print_column!("{}", start.elapsed().as_nanos().to_string());
        result
    }};
}

// Print a column with duration of zero.
macro_rules! no_duration {
    () => {{
        print_column!("0");
    }};
}

/// A mapping between variable names in formula A and formula B.
///
/// Encodes a correspondence where variables on the left side belong to formula A
/// and variables on the right side belong to formula B.
/// Exactly one side must have a single variable:
/// - Both sides single (`a=b`): rename (`a` in A is the same feature as `b` in B).
/// - Left single, right multiple (`a=b,c`): split (`a` in A was split into `b` and `c` in B).
/// - Left multiple, right single (`a,b=c`): merge (`a` and `b` in A were merged into `c` in B).
pub(crate) struct VarMap {
    pub(crate) left: Vec<String>,
    pub(crate) right: Vec<String>,
}

/// Looks up a variable by name and panics with a clear message if not found.
fn resolve_mapped_var(name: &str, arena: &mut Arena) -> VarId {
    arena
        .get_var_named(name.to_string())
        .unwrap_or_else(|| panic!("unknown variable '{}' in --variable-mapping", name))
}

/// Applies variable mappings to the two formulas before differencing.
///
/// In practice, the variable sets of two formula versions are often not identical.
/// Beyond simple addition and removal (which [DiffKind] already handles), three structural
/// changes can occur between versions of a feature model:
/// - **Rename**: a variable was renamed without semantic change.
///   Here, we rename the variable in A so that both formulas agree on the name.
///   Any constraint differences then reflect genuine semantic changes.
/// - **Split**: one variable in A was split into two or more variables in B
///   (a bit related to the KConfig `transitional` keyword).
///   We pretend the variable was always two variables that were kept equal via an
///   atomic-set constraint (`v1 iff v2`), so both formulas operate on the same variable set.
///   Here, we rename the split variable in A to the first split part, and add equivalence
///   clauses for each additional part.
/// - **Merge**: two or more variables in A were merged into one variable in B.
///   Symmetric to split: We expand the single variable in B into the original set by renaming
///   and adding equivalence clauses, so both formulas again share the same variable set.
/// In all cases the goal is to maximize the common variable set seen by [diff],
/// so that semantic differences are not obscured by apparent syntactic differences.
pub(crate) fn apply_var_maps(
    a: &mut Formula,
    b: &mut Formula,
    var_maps: &[VarMap],
    arena: &mut Arena,
) {
    for var_map in var_maps {
        assert!(
            var_map.left.len() == 1 || var_map.right.len() == 1,
            "variable mapping '{}={}' must have exactly one variable on at least one side",
            var_map.left.join(","),
            var_map.right.join(",")
        );
        // Assert exclusivity: left-side variables must belong to A only, right-side to B only.
        // Otherwise applying this mapping doesn't make much sense.
        let left_ids: Vec<VarId> = var_map
            .left
            .iter()
            .map(|n| resolve_mapped_var(n, arena))
            .collect();
        let right_ids: Vec<VarId> = var_map
            .right
            .iter()
            .map(|n| resolve_mapped_var(n, arena))
            .collect();
        for (name, id) in var_map.left.iter().zip(left_ids.iter()) {
            assert!(
                a.sub_var_ids.contains(id) && !b.sub_var_ids.contains(id),
                "variable '{}' (left side) must occur exclusively in formula A",
                name
            );
        }
        for (name, id) in var_map.right.iter().zip(right_ids.iter()) {
            assert!(
                !a.sub_var_ids.contains(id) && b.sub_var_ids.contains(id),
                "variable '{}' (right side) must occur exclusively in formula B",
                name
            );
        }
        if var_map.left.len() == 1 {
            // Rename or split: transform A.
            // Rename left[0] to right[0] in A, then add equivalences for right[1..].
            a.rename_var(left_ids[0], right_ids[0], arena);
            for &extra_id in &right_ids[1..] {
                a.and_equivalent(right_ids[0], extra_id, arena);
            }
        } else {
            // Merge: transform B.
            // Rename right[0] to left[0] in B, then add equivalences for left[1..].
            b.rename_var(right_ids[0], left_ids[0], arena);
            for &extra_id in &left_ids[1..] {
                b.and_equivalent(left_ids[0], extra_id, arena);
            }
        }
    }
    // Applying these mappings may simplify the formula and violate proto-CNF.
    // We must restore proto-CNF for [diff] by wrapping the root in an `And`, if needed.
    a.ensure_proto_cnf(arena);
    b.ensure_proto_cnf(arena);
}

/// Which CNF transformation to apply to a formula in [diff_helper].
#[derive(Clone, Copy)]
pub(crate) enum DiffTransform {
    /// Apply Tseitin transformation (introduces auxiliary variables, scales well).
    Tseitin,
    /// Apply distributive transformation (no auxiliary variables, may blow up exponentially).
    Dist,
}

/// How foreign variables are handled when differencing two formulas.
pub(crate) enum DiffKind {
    /// Fixes all foreign variables to a default boolean value, with optional per-variable overrides.
    ///
    /// A variable is foreign in one formula if it is only mentioned in the other.
    /// Variables in `core_vars` are forced true, variables in `dead_vars` are forced false,
    /// and all remaining foreign variables are forced to `default`.
    Fixed {
        default: bool,
        core_vars: HashSet<VarId>,
        dead_vars: HashSet<VarId>,
    },

    /// Projects the chosen formula onto the common variables of both formulas via slicing.
    Slice,
}

/// Formats a f64, returning an empty string for negative sentinel values.
fn format_f64(v: f64) -> String {
    if v < 0.0 {
        String::new()
    } else {
        v.to_string()
    }
}

/// Formats a BigInt, returning an empty string for negative sentinel values.
fn format_bigint(v: &BigInt) -> String {
    if v.is_negative() {
        String::new()
    } else {
        v.to_string()
    }
}

/// Ensures the output directory implied by a prefix exists, creating it if needed.
///
/// When the prefix contains `/`, the portion before the last `/` is treated as a directory path
/// and created with all intermediate directories.
fn ensure_prefix_dir(prefix: &str) {
    if let Some(slash_pos) = prefix.rfind('/') {
        let dir = &prefix[..slash_pos];
        if !dir.is_empty() {
            std::fs::create_dir_all(dir)
                .unwrap_or_else(|e| panic!("failed to create output directory '{dir}': {e}"));
        }
    }
}

/// Processes a formula by computing its number of solutions (the typical use case),
/// its satisfiability, or by serializing it into a string.
///
/// Optionally applies a CNF transformation on a cloned [Formula] and [Arena].
/// Does not modify the given [Formula] or [Arena].
/// If `cnf` is given, the clause representation fed to the model counter is written to that file.
/// If `satisfy` is true, runs a SAT solver instead of a model counter and returns 0 (UNSAT) or 1 (SAT).
/// The result 1 only indicates that the formula is satisfiable, and is nonsensical if interpreted as a model count.
/// Obviously, this is an ugly helper function that muddles responsibilities.
/// However, it is needed to keep the code below somewhat DRY due to many different supported modes of operation.
pub(crate) fn diff_helper(
    formula: &Formula,
    arena: &Arena,
    cnf_transform: Option<DiffTransform>,
    count_or_satisfy: bool,
    satisfy: bool,
    uvl: bool,
    xml: bool,
    cnf: Option<&str>,
    proj_vars: Option<&HashSet<VarId>>,
) -> (BigInt, Option<String>, Option<String>) {
    let minus_one = -1.to_bigint().unwrap();
    if !count_or_satisfy && !uvl && !xml && cnf.is_none() {
        (minus_one, None, None)
    } else {
        if let Some(path) = cnf {
            io::write_formula(
                &format!("{}.txt", path.strip_suffix(".cnf").unwrap()),
                formula,
                proj_vars,
                arena,
            );
        }
        let clauses = match cnf_transform {
            Some(DiffTransform::Tseitin) => {
                let mut clone = formula.clone();
                let mut arena = arena.clone();
                clone.to_cnf_tseitin(true, &mut arena);
                clone.to_clauses(&arena)
            }
            Some(DiffTransform::Dist) => {
                let mut clone = formula.clone();
                let mut arena = arena.clone();
                clone.to_cnf_dist(&mut arena);
                clone.to_clauses(&arena)
            }
            None => formula.to_clauses(arena),
        };
        if let Some(path) = cnf {
            if let Some(proj_vars) = proj_vars {
                std::fs::write(path, clauses.to_projected_string(proj_vars)).unwrap_or_else(|e| {
                    panic!("failed to write projected clauses to '{path}': {e}")
                });
            } else {
                std::fs::write(path, clauses.to_string())
                    .unwrap_or_else(|e| panic!("failed to write clauses to '{path}': {e}"));
            }
        }
        let count = count_or_satisfy
            .then(|| {
                if satisfy {
                    BigInt::from(clauses.satisfy().is_some() as i32)
                } else {
                    match proj_vars {
                        Some(proj_vars) => clauses.proj_count(proj_vars),
                        None => clauses.count(),
                    }
                }
            })
            .inspect(|count| {
                if *count == minus_one {
                    log(&format!(
                        "[DIFF] timeout while counting number of solutions for partial result"
                    ));
                }
            })
            .unwrap_or(minus_one);
        (
            count,
            uvl.then(|| io::to_uvl_string(&clauses)),
            xml.then(|| io::to_xml_string(&clauses)),
        )
    }
}

/// Classifies a difference of two formulas using Thüm et al. 2009's simplified reasoning algorithm.
///
/// Both formulas must already be in proto-CNF form and refer to the same [Arena].
/// Requires distributive CNF because clause-level set operations need an explicit,
/// Tseitin-free clause representation so that the same semantic clause compares equal across formulas.
///
/// Rather than asking a SAT solver whether the whole formula `a&!b` is satisfiable,
/// we exploit the fact that `a` and `b` share most clauses.
/// We only need to consider the clauses that are in `b` but not in `a`:
/// if `a` together with the negation of any one such clause is satisfiable,
/// then there is a solution in `a` that is not in `b`.
/// Negating a single clause produces a small set of unit clauses,
/// so each individual SAT query is much cheaper than the full `a&!b` query.
/// We stop as soon as one query succeeds.
/// Returns `cnt_removed` = 1 if at least one solution has been removed and 0 otherwise (analogous for `cnt_added`).
fn satisfy_simplified(a: &Formula, b: &Formula, arena: &Arena) -> (BigInt, BigInt) {
    // Run distributive transformation for both formulas independently.
    let build_clauses = |f: &Formula| -> Clauses {
        let mut clone = f.clone();
        let mut arena_clone = arena.clone();
        clone.to_cnf_dist(&mut arena_clone);
        clone.to_clauses(&arena_clone)
    };
    let clauses_a = build_clauses(a);
    let clauses_b = build_clauses(b);

    // Invert var_remap to get a mapping from the clauses-local namespaces to the shared arena namespace.
    let inv_a = clauses_a.inv_var_remap();
    let inv_b = clauses_b.inv_var_remap();

    // Translate one clause from local identifiers to signed arena identifiers and sort the literals.
    // Sorting is required for equality comparison across two independently-built clause representations.
    let canonicalize = |clause: &[VarId], inv: &HashMap<VarId, VarId>| -> Vec<VarId> {
        let mut canonical: Vec<VarId> = clause
            .iter()
            .map(|&lit| {
                let &id = inv
                    .get(&lit.abs())
                    .expect("clause literal has no arena variable");
                if lit > 0 {
                    id
                } else {
                    -id
                }
            })
            .collect();
        canonical.sort_unstable();
        canonical
    };

    // Canonicalize all clauses to use arena identifiers.
    let canonical_a: Vec<Vec<VarId>> = clauses_a
        .clauses
        .iter()
        .map(|c| canonicalize(c, &inv_a))
        .collect();
    let canonical_b: Vec<Vec<VarId>> = clauses_b
        .clauses
        .iter()
        .map(|c| canonicalize(c, &inv_b))
        .collect();

    // Collect clauses unique to a (not in b) and vice versa.
    let set_a: HashSet<&Vec<VarId>> = canonical_a.iter().collect();
    let set_b: HashSet<&Vec<VarId>> = canonical_b.iter().collect();
    let unique_a: Vec<&Vec<VarId>> = canonical_a.iter().filter(|c| !set_b.contains(c)).collect();
    let unique_b: Vec<&Vec<VarId>> = canonical_b.iter().filter(|c| !set_a.contains(c)).collect();

    // Check one subset direction: for each unique clause, assume its negation in the other (base) formula.
    let check_direction =
        |clauses: &Clauses, unique: &[&Vec<VarId>], var_remap: &HashMap<VarId, VarId>| -> BigInt {
            let mut total = Duration::ZERO;
            for canonical_clause in unique {
                // Translate canonical (arena-ID) literals to base formula's local space, then negate.
                let negated: Vec<VarId> = canonical_clause
                    .iter()
                    .map(|&lit| {
                        let &id = var_remap
                            .get(&lit.abs())
                            .expect("unique clause references variable absent from base formula");
                        if lit > 0 {
                            -id
                        } else {
                            id
                        }
                    })
                    .collect();
                let start = Instant::now();
                let sat = clauses.assume(&negated).satisfy().is_some();
                total += start.elapsed();
                if sat {
                    return BigInt::from(1);
                }
            }
            print_column!("{}", total.as_nanos().to_string());
            BigInt::from(0)
        };

    let cnt_removed = check_direction(&clauses_a, &unique_b, &clauses_a.var_remap);
    let cnt_added = check_direction(&clauses_b, &unique_a, &clauses_b.var_remap);
    (cnt_removed, cnt_added)
}

/// Converts a formula to an in-memory CNF [File] by applying distributive CNF transformation.
fn to_cnf_file_dist(formula: &Formula, arena: &Arena) -> File {
    let mut clone = formula.clone();
    let mut arena_clone = arena.clone();
    clone.to_cnf_dist(&mut arena_clone);
    // FeatureIDE does not understand the extension .cnf, use .dimacs instead.
    File::new(
        "-.dimacs".to_string(),
        clone.to_clauses(&arena_clone).to_string(),
    )
}

/// Prints or serializes a description of the difference between two feature-model formulas.
///
/// Assumes that common variables are considered equal (e.g., equal features have equal names),
/// and that both input formulas contain no auxiliary variables.
/// Clean renames, splits, and merges of variables can be handled by passing a [VarMap].
/// When `output` is given, diff artifacts are written to files using `output` as the path prefix
/// (creating intermediate directories as needed). Without `output`, only a CSV line is printed.
/// `count` controls whether model counting is performed (expensive; results appear in CSV).
/// `projected_count` uses a projected model counter instead of a regular one.
/// `satisfy` runs Thüm et al. 2009's SAT-based classification instead of model counting.
/// `simplified` uses Thüm et al. 2009's simplified reasoning (clause iteration) instead of a single SAT call.
/// `featureide` uses FeatureIDE's ModelComparator for classification instead of our reimplementation.
/// `uvl` and `xml` control whether UVL/XML output files are written (require `output`).
/// `cnf` controls whether intermediate clause files are written for each counting step (requires `output`).
pub(crate) fn diff(
    a: &mut Formula,
    b: &mut Formula,
    var_maps: &[VarMap],
    a_diff_kind: DiffKind,
    b_diff_kind: DiffKind,
    output: Option<&str>,
    count: bool,
    projected_count: bool,
    satisfy: bool,
    simplified: bool,
    featureide: bool,
    vars: bool,
    constraints: bool,
    uvl: bool,
    xml: bool,
    cnf: bool,
    no_header: bool,
    cnf_dist: bool,
    is_unsafe: bool,
    negate: bool,
    arena: &mut Arena,
) {
    // Start total time measurement.
    let start = Instant::now();

    // Ensure both formulas are in proto-CNF form.
    a.ensure_proto_cnf(arena);
    b.ensure_proto_cnf(arena);

    // Ensure output directory exists and prepare filename helpers.
    output.map(ensure_prefix_dir);
    let file_path = |name: &str| format!("{}{}", output.unwrap_or(""), name);
    let cnf_path =
        |suffix: &str| -> Option<String> { cnf.then(|| file_path(&format!("{suffix}.cnf"))) };
    let serialize = uvl || xml;

    // Check options.
    if count && projected_count {
        panic!("--count and --projected-count are mutually exclusive");
    }
    if (count || projected_count) && satisfy {
        panic!("--satisfy is mutually exclusive with --count and --projected-count");
    }
    if satisfy && !negate {
        panic!("--satisfy requires --negate");
    }
    if simplified && featureide {
        panic!("--featureide is mutually exclusive with --simplified");
    }
    if (simplified || featureide) && (!satisfy || !cnf_dist) {
        panic!("--simplified and --featureide require --satisfy and --dist");
    }
    if projected_count && (uvl || xml) {
        panic!("--projected-count does not support --uvl or --xml serialization");
    }
    let cnf_transform = if cnf_dist {
        DiffTransform::Dist
    } else {
        DiffTransform::Tseitin
    };

    // Apply variable mappings before any other processing.
    // This is the first step towards unifying both variable sets.
    apply_var_maps(a, b, var_maps, arena);

    // Compute syntactic differences between variables and constraints.
    let (common_var_ids, a_var_ids, b_var_ids) = a.diff_vars(b);
    let (common_constraint_ids, a_constraint_ids, b_constraint_ids) = a.diff_constraints(b, arena);
    let common_vars: u32 = common_var_ids.len().try_into().unwrap();
    let a_vars: u32 = a_var_ids.len().try_into().unwrap();
    let b_vars: u32 = b_var_ids.len().try_into().unwrap();
    let common_constraints: u32 = common_constraint_ids.len().try_into().unwrap();
    let a_constraints: u32 = a_constraint_ids.len().try_into().unwrap();
    let b_constraints: u32 = b_constraint_ids.len().try_into().unwrap();

    // Write variable differences to files if requested.
    if vars {
        io::write_vars(file_path("vars_common.txt"), arena, &common_var_ids);
        io::write_vars(file_path("vars_removed.txt"), arena, &a_var_ids);
        io::write_vars(file_path("vars_added.txt"), arena, &b_var_ids);
    }

    // Write constraint differences to files if requested.
    if constraints {
        io::write_constraints(
            file_path("constraints_common.txt"),
            arena,
            &common_constraint_ids,
        );
        io::write_constraints(
            file_path("constraints_removed.txt"),
            arena,
            &a_constraint_ids,
        );
        io::write_constraints(file_path("constraints_added.txt"), arena, &b_constraint_ids);
    }

    // Already print details about the syntactic differences.
    // Later we will print details about the semantic differences as well, but this way we can
    // still write out the syntactic differences in case of a timeout.
    if !no_header {
        println!("common_vars,removed_vars,added_vars,common_constraints,removed_constraints,added_constraints,left_sliced_duration,right_sliced_duration,left_count_duration,left_sliced_count_duration,right_count_duration,right_sliced_count_duration,left_count,left_sliced_count,right_count,right_sliced_count,lost_solutions,gained_solutions,tseitin_or_featureide_duration,common_solutions_count_duration,common_solutions_count,removed_solutions_count_duration,added_solutions_count_duration,removed_solutions_count,added_solutions_count,removed_solutions,common_solutions,added_solutions,classification,total_duration");
    }
    print_flush!(
        "{common_vars},{a_vars},{b_vars},{common_constraints},{a_constraints},{b_constraints}"
    );

    // We now start with the actual semantic differencing.
    // Note that `a_sliced` can both refer to the original formula `a` or a sliced version of it, depending on whether slicing is requested.
    let mut a_sliced = a.clone();
    let mut b_sliced = b.clone();
    let mut a_sliced_file = a_sliced.file.clone();
    let mut b_sliced_file = b_sliced.file.clone();

    // Slice one or both formulas down to their common variables if requested (does not apply if we are doing projected model counting at the end).
    // This works directly on the input file and consequently requires it to be parsable by FeatureIDE.
    // This is the first step critical for scalability:
    // In FeatureIDE, this uses a resolution-based slicing algorithm that first transforms the formula into CNF with a distributive transformation.
    // Thus, this will fail for complex formulas where distribution explodes exponentially.
    // Even if the formula can be brought into CNF, the slicing itself it can be computationally demanding, especially if many variables are removed.
    if matches!(a_diff_kind, DiffKind::Slice) && !projected_count {
        (a_sliced, a_sliced_file) = measure_time!(a_sliced_file
            .as_ref()
            .unwrap()
            .slice_with_featureide(&common_var_ids, arena, serialize || featureide));
    } else {
        no_duration!();
    }
    if matches!(b_diff_kind, DiffKind::Slice) && !projected_count {
        (b_sliced, b_sliced_file) = measure_time!(b_sliced_file
            .as_ref()
            .unwrap()
            .slice_with_featureide(&common_var_ids, arena, serialize || featureide));
    } else {
        no_duration!();
    }

    // Now force all remaining foreign variables to a default value, if requested.
    // This is a bit intricate because we need to handle a variety of cases correctly.
    // In particular, `a|b_sliced` may at this point have been sliced to the common variables, or not.
    let empty = HashSet::new();
    if let DiffKind::Fixed {
        default,
        ref core_vars,
        ref dead_vars,
    } = a_diff_kind
    {
        // `b_sliced` currently has as `sub_vars` the common variables (if sliced) or possibly also those exclusive to `b` (if not sliced).
        b_sliced = b_sliced.force_foreign_vars(
            default,
            core_vars,
            dead_vars,
            if projected_count { &empty } else { &b_var_ids },
            arena,
        );
        // Let's assume for now we are not doing projected model counting.
        // Now, `b_sliced` has as `sub_vars` the common variables, as well as those exclusive to `a`
        // (which are fully determinate and don't affect the model count).
        // There is a very subtle invariant violation triggered only when slicing _none_ of the formulas above with FeatureIDE:
        // In that case, `b_sliced` is temporarily in an "inconsistent" state, because it still mentions variables exclusive to `b`.
        // We would never want to actually work with such a formula, because it mentions variables in its syntax tree that are not recorded in its `sub_vars`.
        // However, the twist here is that whenever we work with `b_sliced` below, it _will_ be in a consistent state (as we discuss below).
        // So while this code can temporarily violate the invariant that every mentioned variable must be in `sub_vars`, it isn't an issue.
        // Why do we violate the invariant though, in the first place?
        // Because if we do not exclude `b_var_ids`, we will get wrong results when we measure the impact of slicing further below.
        // Now let's assume that we do projected model counting, as the situation is different in this case:
        // We haven't done anything on `b_sliced` yet, and we need not remove its variables because this will be done by the projected model counter.
        // Hence, `b_sliced` now has as `sub_vars` the common variables,
        // and those exclusive to `b` (which will be sliced by the projected model counter),
        // and those exclusive to `a` (which are fully determinate and don't affect the model count).
        // Consequently, there is no invariant violation when we do projected model counting.
        if satisfy
            && matches!(a_diff_kind, DiffKind::Fixed { .. })
            && matches!(b_diff_kind, DiffKind::Fixed { .. })
        {
            // If we do SAT-based classification, the invariant violation is a problem, because we rely on it sooner.
            // However, we then also don't measure the impact of slicing, so we can manually restore the invariant here.
            // It is, however, really simple, because `b_sliced` now simply refers to all variables in the arena.
            // Why guard this to `satisfy`? Because it would mess up the slicing impact measurement as stated above.
            b_sliced.sub_var_ids = arena.var_ids();
        }
        if serialize {
            // The whole invariant discussion does not apply here because we work on the file representation, which is not affected by `force_foreign_vars`.
            let mut file = b_sliced_file
                .as_ref()
                .unwrap()
                .convert_with_featureide("uvl");
            io::uvl_file_add_vars(&mut file, "Removed Features", &a_var_ids, arena);
            b_sliced_file = Some(file);
        } else if featureide {
            // Serialize the variable-adjusted formula to CNF so FeatureIDE sees no foreign variables which could be forced to false during its classification.
            // As FeatureIDE would perform a distributive transformation internally anyway, this does not limit the scalability.
            // Here it is important that the invariant is not violated, which is why we restore it above.
            // For simplicity, we do not report the duration here.
            b_sliced_file = Some(to_cnf_file_dist(&b_sliced, arena));
        }
    }
    if let DiffKind::Fixed {
        default,
        ref core_vars,
        ref dead_vars,
    } = b_diff_kind
    {
        // This logic is symmetric to the logic above.
        a_sliced = a_sliced.force_foreign_vars(
            default,
            core_vars,
            dead_vars,
            if projected_count { &empty } else { &a_var_ids },
            arena,
        );
        if satisfy
            && matches!(a_diff_kind, DiffKind::Fixed { .. })
            && matches!(b_diff_kind, DiffKind::Fixed { .. })
        {
            a_sliced.sub_var_ids = arena.var_ids();
        }
        if serialize {
            let mut file = a_sliced_file
                .as_ref()
                .unwrap()
                .convert_with_featureide("uvl");
            io::uvl_file_add_vars(&mut file, "Added Features", &b_var_ids, arena);
            a_sliced_file = Some(file);
        } else if featureide {
            a_sliced_file = Some(to_cnf_file_dist(&a_sliced, arena));
        }
    }

    // At this point, `b_sliced` has as `sub_vars` the common variables and possibly determinate variables exclusive to `a`,
    // and `a_sliced` has as `sub_vars` the common variables and possibly determinate variables exclusive to `b`.
    // Write out both formulas for debugging purposes if requested.
    // Even though both of these formulas might inviolate the invariant that they only mention variables in their `sub_vars`,
    // this is not an issue here because we do not write a clause representation, but just the formula syntax trees themselves.
    if cnf && !projected_count {
        io::write_formula(&file_path("a_sliced.txt"), &a_sliced, None, arena);
        io::write_formula(&file_path("b_sliced.txt"), &b_sliced, None, arena);
    }

    // Convert both sliced formulas to UVL so we can serialize hierarchies later, if not already done above.
    // This also requires the input file to be parsable by FeatureIDE.
    // The invariant discussion does not apply here because we work on the file representation, similar to above.
    if serialize {
        a_sliced_file = Some(
            a_sliced_file
                .as_ref()
                .unwrap()
                .convert_with_featureide("uvl"),
        );
        b_sliced_file = Some(
            b_sliced_file
                .as_ref()
                .unwrap()
                .convert_with_featureide("uvl"),
        );
    }

    // Initialize counters for various counts.
    let minus_one = -1.to_bigint().unwrap();
    let mut cnt_a = minus_one.clone();
    let mut cnt_a_sliced = minus_one.clone();
    let mut cnt_b = minus_one.clone();
    let mut cnt_b_sliced = minus_one.clone();
    let mut lost_ratio = -1f64;
    let mut gained_ratio = -1f64;

    // Measures the impact of the slicing step from 0 (all sliced variables were determinate) to 1 (all sliced variables were unconstrained).
    // We compute log2(a/b) as log2(a) - log2(b) to avoid converting a huge BigRational directly to f64,
    // which overflows to inf when slicing more than ~1000 features.
    // For each operand, we right-shift to at most 53 significant bits before converting to f64.
    // f64 has a 53-bit significand, so this conversion is exact, and log2(n >> k) = log2(n) - k recovers the true value.
    let log2_bigint = |n: BigInt| -> f64 {
        let bits = n.bits();
        if bits <= 53 {
            n.to_f64().unwrap().log2()
        } else {
            let shift = bits - 53;
            (n >> shift).to_f64().unwrap().log2() + shift as f64
        }
    };
    let ratio = |a: BigInt, b: BigInt, vars: u32| (log2_bigint(a) - log2_bigint(b)) / vars as f64;

    // If we perform slicing, we count the original and sliced formulas here to measure the impact of slicing.
    if count || projected_count {
        if matches!(a_diff_kind, DiffKind::Slice) || !negate {
            // Here we count the original and sliced formulas for `a`.
            // First, we can easily count `a`, which has as `sub_vars` the common variables and those exclusive to `a`.
            cnt_a = measure_time!(
                diff_helper(
                    a,
                    arena,
                    Some(cnf_transform),
                    true,
                    false,
                    false,
                    false,
                    cnf_path("a").as_deref(),
                    None,
                )
                .0
            );
            log(&format!("[DIFF] #a = {}", cnt_a));
            if matches!(a_diff_kind, DiffKind::Slice) {
                // Let's assume for now we don't do projected model counting, so any slicing was performed above with FeatureIDE.
                // In that case, to count the sliced formula, the subtlety mentioned above comes into play:
                // `a_sliced` has as `sub_vars` the common variables and possibly determinate variables exclusive to `b`.
                // In particular, we know here that `a_sliced` has actually been sliced (due to the enclosing `if` statements),
                // and it does not refer anymore to variables exclusive to `a`, hence the invariant is not violated here and we can count `a_sliced`.
                // Also, we need not worry about the variables exclusive to `b`, as they are determinate and they don't affect the model count.
                // Thus, we get a fair comparison between `a` and `a_sliced`, whose model counts only differ in the variables exclusive to `a`.
                // We can map that easily onto a number between 0 and 1 with the `ratio` function from above.
                // Also, because `a_sliced` has been sliced by FeatureIDE, it is already in CNF and we need no further transformation.
                // Now let's assume that we do projected model counting, in which case `a_sliced` has not been sliced yet.
                // `a_sliced` then has as `sub_vars` the common variables, the variables exclusive to `a`,
                // and possibly determinate variables exclusive to `b`, and we simply slice away the `a`-exclusive variables now.
                // Slicing those variables away is equivalent to projecting down to any variables referred to by `b`.
                // In addition, we need to apply a Tseitin transformation to establish CNF.
                let proj_vars = a_sliced
                    .sub_var_ids
                    .difference(&a_var_ids)
                    .cloned()
                    .collect();
                cnt_a_sliced = measure_time!(
                    diff_helper(
                        &a_sliced,
                        arena,
                        projected_count.then_some(cnf_transform),
                        true,
                        false,
                        false,
                        false,
                        cnf_path("a_sliced").as_deref(),
                        if projected_count {
                            Some(&proj_vars)
                        } else {
                            None
                        },
                    )
                    .0
                );
                log(&format!("[DIFF] #a_sliced = {}", cnt_a_sliced));
                // Because we cannot divide by zero, we do not report a ratio if nothing was sliced.
                if a_vars > 0 && cnt_a.is_positive() && cnt_a_sliced.is_positive() {
                    lost_ratio = ratio(cnt_a.clone(), cnt_a_sliced.clone(), a_vars);
                } else {
                    log(&format!(
                        "[DIFF] cannot compute lost solutions, omitting lost solutions from output"
                    ));
                }
            } else {
                // Entering this case means that we want to avoid negation below.
                // To do so, we have to count `a` here, even though we are not interested in the lost ratio.
                // Because we did not slice, both counts are identical (`a` = `a_sliced`).
                cnt_a_sliced = cnt_a.clone();
                no_duration!();
            }
        }
        if !matches!(a_diff_kind, DiffKind::Slice) {
            log(&format!(
                "[DIFF] no slicing requested on the left, omitting lost solutions from output"
            ));
            if negate {
                no_duration!();
                no_duration!();
            }
        }
        if matches!(b_diff_kind, DiffKind::Slice) || !negate {
            // This logic is symmetric to the logic above.
            cnt_b = measure_time!(
                diff_helper(
                    b,
                    arena,
                    Some(cnf_transform),
                    true,
                    false,
                    false,
                    false,
                    cnf_path("b").as_deref(),
                    None,
                )
                .0
            );
            log(&format!("[DIFF] #b = {}", cnt_b));
            if matches!(b_diff_kind, DiffKind::Slice) {
                let proj_vars = b_sliced
                    .sub_var_ids
                    .difference(&b_var_ids)
                    .cloned()
                    .collect();
                cnt_b_sliced = measure_time!(
                    diff_helper(
                        &b_sliced,
                        arena,
                        projected_count.then_some(cnf_transform),
                        true,
                        false,
                        false,
                        false,
                        cnf.then(|| cnf_path("b_sliced")).flatten().as_deref(),
                        if projected_count {
                            Some(&proj_vars)
                        } else {
                            None
                        },
                    )
                    .0
                );
                log(&format!("[DIFF] #b_sliced = {}", cnt_b_sliced));
                if b_vars > 0 && cnt_b.is_positive() && cnt_b_sliced.is_positive() {
                    gained_ratio = ratio(cnt_b.clone(), cnt_b_sliced.clone(), b_vars);
                } else {
                    log(&format!(
                        "[DIFF] cannot compute gained solutions, omitting gained solutions from output"
                    ));
                }
            } else {
                cnt_b_sliced = cnt_b.clone();
                no_duration!();
            }
        }
        if !matches!(b_diff_kind, DiffKind::Slice) {
            log(&format!(
                "[DIFF] no slicing requested on the right, omitting gained solutions from output"
            ));
            if negate {
                no_duration!();
                no_duration!();
            }
        }
    } else {
        log(&format!(
            "[DIFF] no counting requested, omitting all model counts from output"
        ));
        no_duration!();
        no_duration!();
        no_duration!();
        no_duration!();
    }
    print_column!(
        "{},{},{},{},{},{}",
        format_bigint(&cnt_a),
        format_bigint(&cnt_a_sliced),
        format_bigint(&cnt_b),
        format_bigint(&cnt_b_sliced),
        format_f64(lost_ratio),
        format_f64(gained_ratio),
    );

    // At this point, we have a pretty clear understanding of the impact of slicing, if requested.
    // We now want to compare the actual formulas, and we can do it because they now refer to the same set of variables.
    // We start off by determining the commonalities between both formulas (i.e., satisfying assignments shared by both).
    // To do that, we form the conjunction of both formulas. What is subtle here is the variable sets:
    // `and` sets the `sub_vars` to the union of both formulas' `sub_vars`.
    // Let's assume for now we don't do projected model counting, so any slicing is performed above with FeatureIDE.
    // If both formulas have been sliced, they both only refer to the common variables, and the union does nothing.
    // If exactly one formula has been sliced, we have constructed it above to refer to the common
    // variables and to the determinate variables exclusive to the other formula, and the other formula
    // refers to the common variables and its own exclusive variables, and the union does nothing.
    // The interesting case is if no formulas have been sliced, which corresponds to our invariant violation above.
    // In this case, the union will include all variables from both formulas, which restores the invariant.
    // So starting from here we are good to go in terms of finally having a unified variable set.
    // Now let's assume that we do projected model counting, in which case the situation is slightly different:
    // Because we do not exclude any variables during [Formula::force_foreign_vars], one side of the union already includes all variables.
    // This is only not the case when both sides are to be sliced, in which case the union will return all variables anyway.
    // Thus, the conjunction of both formulas will have as `sub_vars` all variables of the arena,
    // which is fine, because we will slice those variables we do not want with the projected model counter.
    // All this does not apply to SAT-based classification, for which we already rectified the invariant violation above.
    let mut diff_base = a_sliced.and(&b_sliced, arena);
    let mut diff;

    let mut classification = String::new();
    if !cnf_dist {
        // Our commonality formula is not in CNF yet (when we counted above, we cloned the arena for a separate CNF transformation).
        // We transform it here once using the total Tseitin transformation.
        // However, we do not assume a root literal yet, so we can reuse this formula for determining both commonalities and differences.
        measure_time!(diff_base.to_cnf_tseitin(false, arena));
    } else if featureide {
        // In case we are classifying with FeatureIDE, we need to ensure the input files are available.
        // At this point, both files will already be in CNF (either due to FeatureIDE slicing or by calling `to_cnf_file_dist`).
        let a_file = a_sliced_file
            .as_ref()
            .expect("--featureide requires a file-backed input for the left formula");
        let b_file = b_sliced_file
            .as_ref()
            .expect("--featureide requires a file-backed input for the right formula");
        classification = measure_time!(exec::io_compare(a_file, b_file));
    } else {
        // If the distributive transformation is requested, we have to transform every formula individually below.
        // This is because there is no equivalent to the root literals introduced by the Tseitin transformation.
        // The distributed transformation is included in the counting duration measurement.
        no_duration!();
    }

    let proj_vars: Option<&HashSet<i32>>;
    let mut compute_removed = true;
    let mut compute_added = true;
    if projected_count {
        // If we do projected model counting, we still need to figure out which variables to slice,
        // as we haven't done this already above with FeatureIDE.
        // Fortunately, this is straightforward and consistent with FeatureIDE's slicing applied above.
        // Note that we do not project auxiliary variables, instead we slice them completely.
        // We must do this because we don't know which auxiliary variable is determined by which named variable.
        // Thus, if we slice `a` and have some auxiliary variable `x` with `x<->!a`, `x` must be sliced as well,
        // to avoid it becoming indeterminate and affecting the model count.
        proj_vars =
            if matches!(a_diff_kind, DiffKind::Slice) && matches!(b_diff_kind, DiffKind::Slice) {
                Some(&common_var_ids)
            } else if matches!(a_diff_kind, DiffKind::Slice) {
                Some(&b_sliced.sub_var_ids)
            } else if matches!(b_diff_kind, DiffKind::Slice) {
                Some(&a_sliced.sub_var_ids)
            } else {
                None
            };

        // Detect unsafe combinations: with projected model counting, negation-based reasoning is
        // unsound when one side has been sliced, because projection does not distribute over negation.
        // The common count is always safe (no negation involved).
        // The removed count is unsafe when the right side is sliced.
        // The added count is unsafe when the left side is sliced.
        if matches!(b_diff_kind, DiffKind::Slice) {
            log(&format!(
                "[DIFF] WARNING: removed count is unsound with projected model counting and right-side slicing{}",
                if is_unsafe { "; reporting anyway (--unsafe)" } else { "; omitting count" }
            ));
            compute_removed = is_unsafe;
        }
        if matches!(a_diff_kind, DiffKind::Slice) {
            log(&format!(
                "[DIFF] WARNING: added count is unsound with projected model counting and left-side slicing (negation over projected formula){}",
                if is_unsafe { "; reporting anyway (--unsafe)" } else { "; omitting count" }
            ));
            compute_added = is_unsafe;
        }

        if cnf {
            io::write_formula(&file_path("a_sliced.txt"), &a_sliced, proj_vars, arena);
            io::write_formula(&file_path("b_sliced.txt"), &b_sliced, proj_vars, arena);
        }
    } else {
        proj_vars = None;
    }

    // In case of distributive transformation, we need to re-transform the formula three times.
    // In case of Tseitin transformation, we can reuse `diff_base` for all three formulas as described above.
    diff = if cnf_dist {
        a_sliced.and(&b_sliced, arena)
    } else {
        diff_base.assume(
            arena.expr(And(vec![a_sliced.root_id, b_sliced.root_id])),
            arena,
        )
    };
    // We are now ready to count or serialize the common satisfying assignments of both formulas.
    // This is done by assuming the conjunction of both formulas' root literals.
    let (cnt_common, uvl_common, xml_common) = measure_time!(diff_helper(
        &diff,
        arena,
        cnf_dist.then_some(DiffTransform::Dist),
        count || projected_count,
        false,
        uvl,
        xml,
        cnf_path("common").as_deref(),
        proj_vars,
    ));
    log(&format!("[DIFF] #common = {}", cnt_common));
    print_column!("{}", format_bigint(&cnt_common));

    // Here we have to decide whether to use the big guns (negation-based reasoning) or avoid it.
    // The general idea is as follows:
    // We can obviously count the number of removed solutions by counting the solutions of the formula `a&!b`.
    // However, this requires negating `b`, which is very expensive with a distributive transformation.
    // With a (total) Tseitin transformation, it is very cheap (we can just flip the root literal).
    // However, even then we still have to count another formula.
    // We can fully circumvent negation by applying the simple counting identity `|a&!b| = |a| - |a&b|`.
    // The ratio of removed solutions is then `|a&!b| / |a| = 1 - |a&b| / |a|`.
    // This is a very elegant solution that avoids repeated, expensive distributive transformation and two calls to a model counter.
    // However, there are still applications for negation:
    // First, if we want to serialize the removed and added solutions, we need to use negation to actually reify the differences.
    // If we are only interested in serialization, we can thus fully avoid model counting and slicing as the only bottleneck.
    // Second, if we merely want to satisfy the difference with a SAT solver (no quantification), we need negation as well.
    // This is particularly interesting if model counting does not scale to a formula, but a SAT solver does.
    // Third, it could be the case that `a&!b` happens to scale better with a model counter than `a`,
    // in which case negation is preferred (this was the original idea of [count_inc]).
    let (mut cnt_removed, mut uvl_removed, mut xml_removed) = (minus_one.clone(), None, None);
    let (mut cnt_added, mut uvl_added, mut xml_added) = (minus_one.clone(), None, None);
    if negate && !featureide {
        if satisfy && simplified {
            // Simplified reasoning means iterating over clauses unique to each side instead of building a&!b.
            // Each SAT query is then tiny (assuming only a handful of unit clauses), and early termination is possible.
            // Here it is also important that the invariant is not violated, which is why we restore it above,
            // because this simplified reasoning algorithm relies on the formula being proper.
            (cnt_removed, cnt_added) = satisfy_simplified(&a_sliced, &b_sliced, arena);
        } else {
            // By reusing the formula and switching the assumption (if possible), we can count or serialize removed satisfying assignments.
            diff = if cnf_dist {
                a_sliced.and_not(&b_sliced, arena)
            } else {
                diff_base.assume(a_sliced.and_not_expr(&b_sliced, arena), arena)
            };
            if compute_removed {
                (cnt_removed, uvl_removed, xml_removed) = measure_time!(diff_helper(
                    &diff,
                    arena,
                    cnf_dist.then_some(DiffTransform::Dist),
                    count || projected_count || satisfy,
                    satisfy,
                    uvl,
                    xml,
                    cnf_path("removed").as_deref(),
                    proj_vars,
                ))
            } else {
                no_duration!();
            };

            // Analogously, we can count or serialize added satisfying assignments.
            diff = if cnf_dist {
                b_sliced.and_not(&a_sliced, arena)
            } else {
                diff_base.assume(b_sliced.and_not_expr(&a_sliced, arena), arena)
            };
            if compute_added {
                (cnt_added, uvl_added, xml_added) = measure_time!(diff_helper(
                    &diff,
                    arena,
                    cnf_dist.then_some(DiffTransform::Dist),
                    count || projected_count || satisfy,
                    satisfy,
                    uvl,
                    xml,
                    cnf_path("added").as_deref(),
                    proj_vars,
                ))
            } else {
                no_duration!();
            };
        }
    } else {
        if count || projected_count {
            cnt_removed = cnt_a_sliced.clone() - cnt_common.clone();
            cnt_added = cnt_b_sliced.clone() - cnt_common.clone();
        }
        no_duration!();
        no_duration!();
    }
    log(&format!("[DIFF] #removed = {}", cnt_removed));
    log(&format!("[DIFF] #added = {}", cnt_added));
    print_column!(
        "{},{}",
        format_bigint(&cnt_removed),
        format_bigint(&cnt_added)
    );

    // Finally, we derive what fraction of the union of solutions is common, removed, or added (if calculable).
    let (common_ratio, removed_ratio, added_ratio) =
        if !cnt_common.is_negative() && !cnt_removed.is_negative() && !cnt_added.is_negative() {
            let cnt_union = &cnt_common + &cnt_removed + &cnt_added;
            (
                BigRational::new(cnt_common.clone(), cnt_union.clone())
                    .to_f64()
                    .unwrap(),
                BigRational::new(cnt_removed.clone(), cnt_union.clone())
                    .to_f64()
                    .unwrap(),
                BigRational::new(cnt_added.clone(), cnt_union.clone())
                    .to_f64()
                    .unwrap(),
            )
        } else {
            log(&format!(
                "[DIFF] cannot compute ratios due to missing data, omitting ratios from output"
            ));
            (-1f64, -1f64, -1f64)
        };
    print_column!(
        "{},{},{}",
        format_f64(removed_ratio),
        format_f64(common_ratio),
        format_f64(added_ratio)
    );

    // Write UVL files for the semantic differences, if requested.
    // For the commonalities, we serialize two versions, one using the feature hierarchy of the left and right formula, respectively.
    if uvl {
        let a_sliced_contents = &a_sliced_file.as_ref().unwrap().contents;
        let b_sliced_contents = &b_sliced_file.as_ref().unwrap().contents;
        io::write_uvl(
            file_path("common_left"),
            a_sliced_contents,
            uvl_common.as_ref().unwrap(),
        );
        io::write_uvl(
            file_path("common_right"),
            b_sliced_contents,
            uvl_common.as_ref().unwrap(),
        );
        io::write_uvl(
            file_path("removed"),
            a_sliced_contents,
            uvl_removed.as_ref().unwrap(),
        );
        io::write_uvl(
            file_path("added"),
            b_sliced_contents,
            uvl_added.as_ref().unwrap(),
        );
    }

    // Write XML files for the semantic differences, if requested. Again, we serialize two versions for the commonalities.
    if xml {
        let a_sliced_contents = &a_sliced_file.as_ref().unwrap().contents;
        let b_sliced_contents = &b_sliced_file.as_ref().unwrap().contents;
        io::write_xml(
            file_path("common_left"),
            a_sliced_contents,
            xml_common.as_ref().unwrap(),
        );
        io::write_xml(
            file_path("common_right"),
            b_sliced_contents,
            xml_common.as_ref().unwrap(),
        );
        io::write_xml(
            file_path("removed"),
            a_sliced_contents,
            xml_removed.as_ref().unwrap(),
        );
        io::write_xml(
            file_path("added"),
            b_sliced_contents,
            xml_added.as_ref().unwrap(),
        );
    }

    // Derive Thüm et al. 2009's classification from removed/added solution data whenever available.
    // "available" means at least one of count, projected_count, or satisfy was active and successful,
    // giving us meaningful (non-negative) values for cnt_removed and cnt_added.
    if !cnt_removed.is_negative() && !cnt_added.is_negative() {
        classification = match (cnt_removed.is_positive(), cnt_added.is_positive()) {
            (false, false) => "Refactoring",
            (true, false) => "Specialization",
            (false, true) => "Generalization",
            (true, true) => "ArbitraryEdit",
        }
        .to_owned();
    }
    print_column!("{classification}");

    // Print total time measurement.
    print_column!("{}", start.elapsed().as_nanos().to_string());
}
