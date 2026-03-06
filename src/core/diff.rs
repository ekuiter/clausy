//! Differencing of feature-model formulas.

use crate::util::io;
use num::{bigint::ToBigInt, BigInt, BigRational, ToPrimitive};
use std::{
    collections::HashSet,
    io::Write,
    time::{Duration, Instant},
};

use super::{arena::Arena, expr::Expr::And, formula::Formula, var::VarId};

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
pub(crate) fn diff_helper(
    formula: &Formula,
    arena: &Arena,
    tseitin_transform: bool,
    count: bool,
    uvl: bool,
    xml: bool,
    cnf: Option<&str>,
) -> (BigInt, Option<String>, Option<String>) {
    let minus_one = -1.to_bigint().unwrap();
    if !count && !uvl && !xml && cnf.is_none() {
        (minus_one, None, None)
    } else {
        if let Some(path) = cnf {
            io::write_formula(
                &format!("{}.txt", path.strip_suffix(".cnf").unwrap()),
                formula,
                arena,
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
            std::fs::write(path, clauses.to_string())
                .unwrap_or_else(|e| panic!("failed to write clauses to '{path}': {e}"));
        }
        (
            count.then(|| clauses.count()).unwrap_or(minus_one),
            uvl.then(|| io::to_uvl_string(&clauses)),
            xml.then(|| io::to_xml_string(&clauses)),
        )
    }
}

/// Prints or serializes a description of the difference between two feature-model formulas.
///
/// Assumes that common variables are considered equal (e.g., equal features have equal names),
/// that both input formulas contain no auxiliary variables, and that both are in proto-CNF.
/// When `output` is given, diff artifacts are written to files using `output` as the path prefix
/// (creating intermediate directories as needed). Without a prefix, only a CSV line is printed.
/// `count` controls whether model counting is performed (expensive; results appear in CSV).
/// `uvl` and `xml` control whether UVL/XML output files are written (require `output`).
/// `cnf` controls whether intermediate clause files are written for each counting step (requires `output`).
pub(crate) fn diff(
    a: &Formula,
    b: &Formula,
    a_diff_kind: DiffKind,
    b_diff_kind: DiffKind,
    output: Option<&str>,
    no_count: bool,
    vars: bool,
    constraints: bool,
    uvl: bool,
    xml: bool,
    cnf: bool,
    no_header: bool,
    arena: &mut Arena,
) {
    // Ensure both formulas are in proto-CNF form.
    a.assert_proto_cnf(arena);
    b.assert_proto_cnf(arena);

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
    let count = !no_count;

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
        println!("common_vars,removed_vars,added_vars,common_constraints,removed_constraints,added_constraints,lost_products,removed_products,common_products,added_products,gained_products,left_count,left_sliced_count,right_count,right_sliced_count,common_products_count,removed_products_count,added_products_count,left_sliced_duration,right_sliced_duration,left_count_duration,left_sliced_count_duration,right_count_duration,right_sliced_count_duration,tseitin_duration,common_products_count_duration,removed_products_count_duration,added_products_count_duration,total_duration");
    }
    print!("{common_vars},{a_vars},{b_vars},{common_constraints},{a_constraints},{b_constraints}");
    std::io::stdout().flush().unwrap();

    // We now start with the actual semantic differencing.
    // Not that `a_sliced` can both refer to the original formula `a` or a sliced version of it, depending on whether slicing is requested.
    let mut a_sliced = a.clone();
    let mut b_sliced = b.clone();
    let mut a_sliced_file = a_sliced.file.clone();
    let mut b_sliced_file = b_sliced.file.clone();

    // Slice one or both formulas down to their common variables if requested.
    // This works directly on the input file and consequently requires it to be parsable by FeatureIDE.
    if let DiffKind::Slice = a_diff_kind {
        (a_sliced, a_sliced_file) = measure_time!(a_sliced_file
            .as_ref()
            .unwrap()
            .slice_with_featureide(&common_var_ids, arena, serialize));
    } else {
        no_duration!();
    }
    if let DiffKind::Slice = b_diff_kind {
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
    if let DiffKind::Fixed {
        default,
        ref core_vars,
        ref dead_vars,
    } = a_diff_kind
    {
        // `b_sliced` now has as `sub_vars` the common variables (if sliced) or possibly also those exclusive to `b` (if not sliced).
        b_sliced = b_sliced.force_foreign_vars(default, core_vars, dead_vars, &b_var_ids, arena);
        // `b_sliced` now has as `sub_vars` the common variables, as well as those exclusive to `a`
        // (which are fully determinate and don't affect the model count).
        // There is a very subtle invariant violation triggered only when slicing _none_ of the formulas:
        // In that case, `b_sliced` is temporarily in an "inconsistent" state, because it still mentions variables exclusive to `b`.
        // We would never want to actually work with such a formula, because it mentions variables in its syntax tree that are not recorded in its `sub_vars`.
        // However, the twist here is that whenever we work with `b_sliced` below, it _will_ be in a consistent state (as we discuss below).
        // So while this code can temporarily violate the invariant that every mentioned variable must be in `sub_vars`, it isn't an issue.
        // Why do we violate the invariant though, in the first place?
        // Because it will be convenient to exclude `b_var_ids` when we measure the impact of slicing further below.
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
        a_sliced = a_sliced.force_foreign_vars(default, core_vars, dead_vars, &a_var_ids, arena);
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
    if cnf {
        io::write_formula(&file_path("a_sliced.txt"), &a_sliced, arena);
        io::write_formula(&file_path("b_sliced.txt"), &b_sliced, arena);
    }

    // Convert both sliced formulas to UVL so we can serialize hierarchies later, if not already done above.
    // This also requires the input file to be parsable by FeatureIDE.
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

    // If we performed slicing above, we count the original and sliced formulas here to measure the impact of slicing.
    if count {
        if let DiffKind::Slice = a_diff_kind {
            // Here we count the original and sliced formulas for `a`.
            // Here the subtlety mentioned above comes into play:
            // `a` has as `sub_vars` the common variables and those exclusive to `a`.
            // `a_sliced` has as `sub_vars` the common variables and possibly determinate variables exclusive to `b`.
            // In particular, we know here that `a_sliced` has actually been sliced (due to DiffKind::Slice),
            // and it does not refer anymore to variables exclusive to `a`, hence the invariant is not violated here and we can count `a_sliced`.
            // Also, we need not worry about the variables exclusive to `b`, as they are determinate and they don't affect the model count.
            // Thus, we get a fair comparison between `a` and `a_sliced`, whose model counts only differ in the variables exclusive to `a`.
            // We can map that easily onto a number between 0 and 1 with the `ratio` function from above.
            cnt_a = measure_time!(
                diff_helper(a, arena, true, count, uvl, xml, cnf_path("a").as_deref()).0
            );
            cnt_a_sliced = measure_time!(
                diff_helper(
                    &a_sliced,
                    arena,
                    false,
                    count,
                    uvl,
                    xml,
                    cnf_path("a_sliced").as_deref()
                )
                .0
            );
            // Because we cannot divide by zero, we do not report a ratio if nothing was sliced.
            if a_vars > 0 {
                lost_ratio = ratio(cnt_a.clone(), cnt_a_sliced.clone(), a_vars);
            }
        } else {
            no_duration!();
            no_duration!();
        }
        if let DiffKind::Slice = b_diff_kind {
            // This logic is symmetric to the logic above.
            cnt_b = measure_time!(
                diff_helper(b, arena, true, count, uvl, xml, cnf_path("b").as_deref()).0
            );
            cnt_b_sliced = measure_time!(
                diff_helper(
                    &b_sliced,
                    arena,
                    false,
                    count,
                    uvl,
                    xml,
                    cnf_path("b_sliced").as_deref()
                )
                .0
            );
            if b_vars > 0 {
                gained_ratio = ratio(cnt_b.clone(), cnt_b_sliced.clone(), b_vars);
            }
        } else {
            no_duration!();
            no_duration!();
        }
    } else {
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
    // If both formulas have been sliced, they both only refer to the common variables, and the union does nothing.
    // If exactly one formula has been sliced, we have constructed it above to refer to the common
    // variables and to determinate variables exclusive to the other formula, and the other formula
    // refers to the common variables and its own exclusive variables, and the union does nothing.
    // The interesting case is if no formulas have been sliced, which corresponds to our invariant violation above.
    // In this case, the union will include all variables from both formulas, which restores the invariant.
    // So starting from here we are good to go in terms of finally having a unified variable set.
    let mut diff = a_sliced.and(&b_sliced, arena);
    // This formula is not in CNF yet (when we counted above we made a clone of the arena and a separate CNF transformation).
    // We will transform it once using the total Tseitin transformation.
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
        count,
        uvl,
        xml,
        cnf_path("common").as_deref()
    ));

    // By reusing the formula and switching at the assumption, we can count or serialize removed satisfying assignments.
    let (cnt_removed, uvl_removed, xml_removed) = measure_time!(diff_helper(
        &diff.assume(a_sliced.and_not_expr(&b_sliced, arena), arena),
        arena,
        false,
        count,
        uvl,
        xml,
        cnf_path("removed").as_deref()
    ));

    // Analogously, we can count or serialize added satisfying assignments.
    let (cnt_added, uvl_added, xml_added) = measure_time!(diff_helper(
        &diff.assume(b_sliced.and_not_expr(&a_sliced, arena), arena),
        arena,
        false,
        count,
        uvl,
        xml,
        cnf_path("added").as_deref()
    ));

    // Finally, we derive what fraction of the union of solutions is common, removed, or added.
    let (common_ratio, removed_ratio, added_ratio) = if count {
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

    // Print the remaining details about the semantic differences.
    let durations: Vec<String> = durations.iter().map(|d| d.as_nanos().to_string()).collect();
    let durations = durations.join(",");
    println!(",{lost_ratio},{removed_ratio},{common_ratio},{added_ratio},{gained_ratio},{cnt_a},{cnt_a_sliced},{cnt_b},{cnt_b_sliced},{cnt_common},{cnt_removed},{cnt_added},{durations}");
}
