//! Differencing of feature-model formulas.

use crate::util::io;
use crate::util::log::log;
use num::{BigInt, BigRational, Signed, ToPrimitive, bigint::ToBigInt};
use std::{
    collections::HashSet,
    io::Write,
    time::{Duration, Instant},
};

use super::{arena::Arena, expr::Expr::And, formula::Formula, var::VarId};

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
pub(crate) fn apply_var_maps(a: &mut Formula, b: &mut Formula, var_maps: &[VarMap], arena: &mut Arena) {
    for var_map in var_maps {
        assert!(
            var_map.left.len() == 1 || var_map.right.len() == 1,
            "variable mapping '{}={}' must have exactly one variable on at least one side",
            var_map.left.join(","),
            var_map.right.join(",")
        );
        // Assert exclusivity: left-side variables must belong to A only, right-side to B only.
        // Otherwise applying this mapping doesn't make much sense.
        let left_ids: Vec<VarId> = var_map.left.iter().map(|n| resolve_mapped_var(n, arena)).collect();
        let right_ids: Vec<VarId> = var_map.right.iter().map(|n| resolve_mapped_var(n, arena)).collect();
        for (name, id) in var_map.left.iter().zip(left_ids.iter()) {
            assert!(a.sub_var_ids.contains(id) && !b.sub_var_ids.contains(id),
                "variable '{}' (left side) must occur exclusively in formula A", name);
        }
        for (name, id) in var_map.right.iter().zip(right_ids.iter()) {
            assert!(!a.sub_var_ids.contains(id) && b.sub_var_ids.contains(id),
                "variable '{}' (right side) must occur exclusively in formula B", name);
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

/// Returns the number of solutions of a given formula or serializes it.
///
/// Optionally uses a Tseitin transformation into CNF on a cloned [Formula] and [Arena].
/// Does not modify the given [Formula] or [Arena].
/// If `clauses_path` is given, the clauses representation fed to the model counter is written to that file.
/// This is an ugly helper function that muddles responsibilities, but it is necessary to keep the code below DRY.
pub(crate) fn diff_helper(
    formula: &Formula,
    arena: &Arena,
    tseitin_transform: bool,
    any_count: bool,
    uvl: bool,
    xml: bool,
    cnf: Option<&str>,
    proj_vars: Option<&HashSet<VarId>>,
) -> (BigInt, Option<String>, Option<String>) {
    let minus_one = -1.to_bigint().unwrap();
    if !any_count && !uvl && !xml && cnf.is_none() {
        (minus_one, None, None)
    } else {
        if let Some(path) = cnf {
            io::write_formula(
                &format!("{}.txt", path.strip_suffix(".cnf").unwrap()),
                formula,
                proj_vars,
                arena
            );
        }
        let clauses;
        if tseitin_transform {
            let mut clone = formula.clone();
            let mut arena = arena.clone();
            clone.to_cnf_tseitin(true, &mut arena);
            clauses = clone.to_clauses(&arena);
        } else {
            clauses = formula.to_clauses(arena);
        }
        if let Some(path) = cnf {
            if let Some(proj_vars) = proj_vars {
                std::fs::write(path, clauses.to_projected_string(proj_vars))
                    .unwrap_or_else(|e| panic!("failed to write projected clauses to '{path}': {e}"));
            } else {
                std::fs::write(path, clauses.to_string())
                    .unwrap_or_else(|e| panic!("failed to write clauses to '{path}': {e}"));
            }
        }
        let count = any_count
                .then(|| match proj_vars {
                    Some(proj_vars) => clauses.proj_count(proj_vars),
                    None => clauses.count(),
                })
                .inspect(|count| {
                    if *count == minus_one {
                        log(&format!("[DIFF] timeout while counting number of solutions for partial result"));
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

/// Prints or serializes a description of the difference between two feature-model formulas.
///
/// Assumes that common variables are considered equal (e.g., equal features have equal names),
/// and that both input formulas contain no auxiliary variables.
/// Clean renames, splits, and merges of variables can be handled by passing a [VarMap].
/// When `output` is given, diff artifacts are written to files using `output` as the path prefix
/// (creating intermediate directories as needed). Without `output`, only a CSV line is printed.
/// `count` controls whether model counting is performed (expensive; results appear in CSV).
/// `projected_count` uses a projected model counter instead of a regular one.
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
    vars: bool,
    constraints: bool,
    uvl: bool,
    xml: bool,
    cnf: bool,
    no_header: bool,
    arena: &mut Arena,
) {
    // Ensure both formulas are in proto-CNF form.
    a.ensure_proto_cnf(arena);
    b.ensure_proto_cnf(arena);

    // Ensure output directory exists and prepare filename helpers.
    output.map(ensure_prefix_dir);
    let file_path = |name: &str| format!("{}{}", output.unwrap_or(""), name);
    let cnf_path =
        |suffix: &str| -> Option<String> { cnf.then(|| file_path(&format!("{suffix}.cnf"))) };
    let serialize = uvl || xml;

    // Prepare helpers for measuring time durations.
    let mut durations: Vec<Duration> = vec![];
    macro_rules! measure_time {
        ($expr:expr) => {{
            let start = Instant::now();
            let result = $expr;
            durations.push(start.elapsed());
            result
        }};
    }
    macro_rules! no_duration {
        () => {{
            durations.push(Duration::ZERO)
        }};
    }
    if count && projected_count {
        panic!("--no-count and --projected-count are mutually exclusive");
    }
    if projected_count && (uvl || xml) {
        panic!("--projected-count does not support --uvl or --xml serialization");
    }

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
        println!("common_vars,removed_vars,added_vars,common_constraints,removed_constraints,added_constraints,lost_solutions,removed_solutions,common_solutions,added_solutions,gained_solutions,left_count,left_sliced_count,right_count,right_sliced_count,common_solutions_count,removed_solutions_count,added_solutions_count,left_sliced_duration,right_sliced_duration,left_count_duration,left_sliced_count_duration,right_count_duration,right_sliced_count_duration,tseitin_duration,common_solutions_count_duration,removed_solutions_count_duration,added_solutions_count_duration,total_duration");
    }
    print!("{common_vars},{a_vars},{b_vars},{common_constraints},{a_constraints},{b_constraints}");
    std::io::stdout().flush().unwrap();

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
            .slice_with_featureide(&common_var_ids, arena, serialize));
    } else {
        no_duration!();
    }
    if matches!(b_diff_kind, DiffKind::Slice) && !projected_count {
        (b_sliced, b_sliced_file) = measure_time!(b_sliced_file
            .as_ref()
            .unwrap()
            .slice_with_featureide(&common_var_ids, arena, serialize));
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
        b_sliced = b_sliced.force_foreign_vars(default, core_vars, dead_vars, if projected_count { &empty } else { &b_var_ids }, arena);
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
        if serialize {
            // The whole invariant discussion does not apply here because we work on the file representation, which is not affected by `force_foreign_vars`.
            let mut file = b_sliced_file
                .as_ref()
                .unwrap()
                .convert_with_featureide("uvl");
            io::uvl_file_add_vars(&mut file, "Removed Features", &a_var_ids, arena);
            b_sliced_file = Some(file);
        }
    }
    if let DiffKind::Fixed {
        default,
        ref core_vars,
        ref dead_vars,
    } = b_diff_kind
    {
        // This logic is symmetric to the logic above.
        a_sliced = a_sliced.force_foreign_vars(default, core_vars, dead_vars, if projected_count { &empty } else { &a_var_ids }, arena);
        if serialize {
            let mut file = a_sliced_file
                .as_ref()
                .unwrap()
                .convert_with_featureide("uvl");
            io::uvl_file_add_vars(&mut file, "Added Features", &b_var_ids, arena);
            a_sliced_file = Some(file);
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
    // This currently only supports deleting/adding up to 1000 features due to f64 precision.
    let ratio =
        |a, b, vars: u32| BigRational::new(a, b).to_f64().unwrap().log2() / vars.to_f64().unwrap();

    // If we perform slicing, we count the original and sliced formulas here to measure the impact of slicing.
    if count || projected_count {
        if let DiffKind::Slice = a_diff_kind {
            // Here we count the original and sliced formulas for `a`.
            // First, we can easily count `a`, which has as `sub_vars` the common variables and those exclusive to `a`.
            cnt_a = measure_time!(
                diff_helper(a, arena, true, true, false, false, cnf_path("a").as_deref(), None).0
            );
            log(&format!("[DIFF] #a = {}", cnt_a));
            // Let's assume for now we don't do projected model counting, so any slicing is performed above with FeatureIDE.
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
            let proj_vars = a_sliced.sub_var_ids.difference(&a_var_ids).cloned().collect();
            cnt_a_sliced = measure_time!(
                diff_helper(
                    &a_sliced,
                    arena,
                    projected_count,
                    true,
                    false,
                    false,
                    cnf_path("a_sliced").as_deref(),
                    if projected_count { Some(&proj_vars) } else { None }
                )
                .0
            );
            log(&format!("[DIFF] #a_sliced = {}", cnt_a_sliced));
            // Because we cannot divide by zero, we do not report a ratio if nothing was sliced.
            if a_vars > 0 && cnt_a.is_positive() && cnt_a_sliced.is_positive() {
                lost_ratio = ratio(cnt_a.clone(), cnt_a_sliced.clone(), a_vars);
            } else {
                log(&format!("[DIFF] cannot compute lost solutions, omitting lost solutions from output"));
            }
        } else {
            log(&format!("[DIFF] no slicing requested on the left, omitting lost solutions from output"));
            no_duration!();
            no_duration!();
        }
        if let DiffKind::Slice = b_diff_kind {
            // This logic is symmetric to the logic above.
            cnt_b = measure_time!(
                diff_helper(b, arena, true, true, false, false, cnf_path("b").as_deref(), None).0
            );
            log(&format!("[DIFF] #b = {}", cnt_b));
            let proj_vars = b_sliced.sub_var_ids.difference(&b_var_ids).cloned().collect();
            cnt_b_sliced = measure_time!(
                diff_helper(
                    &b_sliced,
                    arena,
                    projected_count,
                    true,
                    false,
                    false,
                    cnf.then(|| cnf_path("b_sliced")).flatten().as_deref(),
                    if projected_count { Some(&proj_vars) } else { None }
                )
                .0
            );
            log(&format!("[DIFF] #b_sliced = {}", cnt_b_sliced));
            if b_vars > 0 && cnt_b.is_positive() && cnt_b_sliced.is_positive() {
                gained_ratio = ratio(cnt_b.clone(), cnt_b_sliced.clone(), b_vars);
            } else {
                log(&format!("[DIFF] cannot compute gained solutions, omitting gained solutions from output"));
            }
        } else {
            log(&format!("[DIFF] no slicing requested on the right, omitting gained solutions from output"));
            no_duration!();
            no_duration!();
        }
    } else {
        log(&format!("[DIFF] no counting requested, omitting all model counts from output"));
        no_duration!();
        no_duration!();
        no_duration!();
        no_duration!();
    }

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
    let mut diff = a_sliced.and(&b_sliced, arena);

    // If we do projected model counting, we still need to figure out which variables to slice,
    // as we haven't done this already above with FeatureIDE.
    // Fortunately, this is straightforward and consistent with FeatureIDE's slicing applied above.
    // todo: currently we slice all auxiliary variables, but it is unclear whether this is correct and whether it impacts performance
    let proj_vars: Option<&HashSet<i32>>;
    if projected_count {
        proj_vars = if matches!(a_diff_kind, DiffKind::Slice) && matches!(b_diff_kind, DiffKind::Slice) {
            Some(&common_var_ids)
        } else if matches!(a_diff_kind, DiffKind::Slice) {
            Some(&b_sliced.sub_var_ids)
        } else if matches!(b_diff_kind, DiffKind::Slice) {
            Some(&a_sliced.sub_var_ids)
        } else {
            None
        };
        if cnf {
            io::write_formula(&file_path("a_sliced.txt"), &a_sliced, proj_vars, arena);
            io::write_formula(&file_path("b_sliced.txt"), &b_sliced, proj_vars, arena);
        }
    } else {
        proj_vars = None;
    }

    // This commonality formula is not in CNF yet (when we counted above, we cloned the arena for a separate CNF transformation).
    // We transform it here once using the total Tseitin transformation.
    // However, we do not assume a root literal yet, so we can reuse this formula for determining both commonalities and differences.
    measure_time!(diff.to_cnf_tseitin(false, arena));

    // We are now ready to count or serialize the common satisfying assignments of both formulas.
    // This is done by assuming the conjunction of both formulas' root literals.
    let (cnt_common, uvl_common, xml_common) = measure_time!(diff_helper(
        &diff.assume(
            arena.expr(And(vec![a_sliced.root_id, b_sliced.root_id])),
            arena
        ),
        arena,
        false,
        count || projected_count,
        uvl,
        xml,
        cnf_path("common").as_deref(),
        proj_vars
    ));
    log(&format!("[DIFF] #common = {}", cnt_common));

    // By reusing the formula and switching at the assumption, we can count or serialize removed satisfying assignments.
    let (cnt_removed, uvl_removed, xml_removed) = measure_time!(diff_helper(
        &diff.assume(a_sliced.and_not_expr(&b_sliced, arena), arena),
        arena,
        false,
        count || projected_count,
        uvl,
        xml,
        cnf_path("removed").as_deref(),
        proj_vars
    ));
    log(&format!("[DIFF] #removed = {}", cnt_removed));

    // Analogously, we can count or serialize added satisfying assignments.
    let (cnt_added, uvl_added, xml_added) = measure_time!(diff_helper(
        &diff.assume(b_sliced.and_not_expr(&a_sliced, arena), arena),
        arena,
        false,
        count || projected_count,
        uvl,
        xml,
        cnf_path("added").as_deref(),
        proj_vars
    ));
    log(&format!("[DIFF] #added = {}", cnt_added));

    // Finally, we derive what fraction of the union of solutions is common, removed, or added.
    let (common_ratio, removed_ratio, added_ratio) = if !cnt_common.is_negative() && !cnt_removed.is_negative() && !cnt_added.is_negative() {
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
        log(&format!("[DIFF] cannot compute ratios due to missing data, omitting ratios from output"));
        (-1f64, -1f64, -1f64)
    };

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

    // Print the remaining details about the semantic differences, omitting results that are -1.
    let durations: Vec<String> = durations.iter().map(|d| d.as_nanos().to_string()).collect();
    let durations = durations.join(",");
    let ff = |v: f64| if v < 0.0 { String::new() } else { v.to_string() };
    let fb = |v: &BigInt| if v.is_negative() { String::new() } else { v.to_string() };
    println!(",{},{},{},{},{},{},{},{},{},{},{},{},{durations}",
        ff(lost_ratio), ff(removed_ratio), ff(common_ratio), ff(added_ratio), ff(gained_ratio),
        fb(&cnt_a), fb(&cnt_a_sliced), fb(&cnt_b), fb(&cnt_b_sliced),
        fb(&cnt_common), fb(&cnt_removed), fb(&cnt_added));
}
