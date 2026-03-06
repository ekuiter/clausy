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
    serialize: bool,
    clauses_path: Option<&str>,
) -> (BigInt, Option<String>, Option<String>) {
    let minus_one = -1.to_bigint().unwrap();
    if !count && !serialize && clauses_path.is_none() {
        (minus_one, None, None)
    } else {
        let clauses;
        if tseitin_transform {
            let mut clone = formula.clone();
            let mut arena = arena.clone();
            clone.to_cnf_tseitin(true, &mut arena);
            clauses = clone.to_clauses(&arena);
        } else {
            clauses = formula.to_clauses(arena);
        }
        if let Some(path) = clauses_path {
            std::fs::write(path, clauses.to_string())
                .unwrap_or_else(|e| panic!("failed to write DIMACS to '{path}': {e}"));
        }
        (
            count.then(|| clauses.count()).unwrap_or(minus_one),
            serialize.then(|| io::to_uvl_string(&clauses)),
            serialize.then(|| io::to_xml_string(&clauses)),
        )
    }
}

/// Prints or serializes a description of the difference between two feature-model formulas.
///
/// Assumes that common variables are considered equal (e.g., equal features have equal names),
/// that both input formulas contain no auxiliary variables, and that both are in proto-CNF.
///
/// When `output` is given, diff artifacts are written to files using `output` as the path prefix
/// (creating intermediate directories as needed). Without a prefix, only a CSV line is printed.
///
/// When `verbose` is true (requires `output`), intermediate clause representations for each
/// counting step are also written alongside the other artifacts.
pub(crate) fn diff(
    a: &Formula,
    b: &Formula,
    a_diff_kind: DiffKind,
    b_diff_kind: DiffKind,
    output: Option<&str>,
    verbose: bool,
    arena: &mut Arena,
) {
    a.assert_proto_cnf(arena);
    b.assert_proto_cnf(arena);

    let write_files = output.is_some();
    if write_files {
        ensure_prefix_dir(output.unwrap());
    }
    let file_name = |name: &str| format!("{}{}", output.unwrap_or(""), name);
    let do_count = !write_files || verbose;
    let do_serialize = write_files;
    let dimacs_path = |suffix: &str| -> Option<String> {
        verbose.then(|| file_name(&format!("{suffix}.dimacs")))
    };

    let (common_var_ids, a_var_ids, b_var_ids) = a.diff_vars(b);
    let (common_constraint_ids, a_constraint_ids, b_constraint_ids) = a.diff_constraints(b, arena);
    let common_vars: u32 = common_var_ids.len().try_into().unwrap();
    let a_vars: u32 = a_var_ids.len().try_into().unwrap();
    let b_vars: u32 = b_var_ids.len().try_into().unwrap();
    let common_constraints: u32 = common_constraint_ids.len().try_into().unwrap();
    let a_constraints: u32 = a_constraint_ids.len().try_into().unwrap();
    let b_constraints: u32 = b_constraint_ids.len().try_into().unwrap();

    if write_files {
        io::write_vars(file_name(".common.features"), arena, &common_var_ids);
        io::write_vars(file_name(".removed.features"), arena, &a_var_ids);
        io::write_vars(file_name(".added.features"), arena, &b_var_ids);
        io::write_constraints(
            file_name(".common.constraints"),
            arena,
            &common_constraint_ids,
        );
        io::write_constraints(file_name(".removed.constraints"), arena, &a_constraint_ids);
        io::write_constraints(file_name(".added.constraints"), arena, &b_constraint_ids);
    } else {
        print!(
            "{common_vars},{a_vars},{b_vars},{common_constraints},{a_constraints},{b_constraints}"
        );
        std::io::stdout().flush().unwrap();
    }

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

    let mut a2 = a.clone();
    let mut b2 = b.clone();
    let mut a2_file = a2.file.clone();
    let mut b2_file = b2.file.clone();

    if let DiffKind::Slice = a_diff_kind {
        (a2, a2_file) = measure_time!(a2_file.as_ref().unwrap().slice_with_featureide(
            &common_var_ids,
            arena,
            write_files
        ));
    } else {
        no_duration!();
    }
    if let DiffKind::Slice = b_diff_kind {
        (b2, b2_file) = measure_time!(b2_file.as_ref().unwrap().slice_with_featureide(
            &common_var_ids,
            arena,
            write_files
        ));
    } else {
        no_duration!();
    }
    if let DiffKind::Fixed {
        default,
        ref core_vars,
        ref dead_vars,
    } = a_diff_kind
    {
        b2 = b2.force_foreign_vars(default, core_vars, dead_vars, &b_var_ids, arena);
        if write_files {
            let mut file = b2_file.as_ref().unwrap().convert_with_featureide("uvl");
            io::uvl_file_add_vars(&mut file, "Removed Features", &a_var_ids, arena);
            b2_file = Some(file);
        }
    }
    if let DiffKind::Fixed {
        default,
        ref core_vars,
        ref dead_vars,
    } = b_diff_kind
    {
        a2 = a2.force_foreign_vars(default, core_vars, dead_vars, &a_var_ids, arena);
        if write_files {
            let mut file = a2_file.as_ref().unwrap().convert_with_featureide("uvl");
            io::uvl_file_add_vars(&mut file, "Added Features", &b_var_ids, arena);
            a2_file = Some(file);
        }
    }
    if write_files {
        a2_file = Some(a2_file.as_ref().unwrap().convert_with_featureide("uvl"));
        b2_file = Some(b2_file.as_ref().unwrap().convert_with_featureide("uvl"));
    }

    let minus_one = -1.to_bigint().unwrap();
    let mut cnt_a = minus_one.clone();
    let mut cnt_a2 = minus_one.clone();
    let mut cnt_b = minus_one.clone();
    let mut cnt_b2 = minus_one.clone();
    let mut lost_ratio = -1f64;
    let mut gained_ratio = -1f64;
    // This currently only supports deleting/adding up to 1000 features due to f64 precision.
    let ratio =
        |a, b, vars: u32| BigRational::new(a, b).to_f64().unwrap().log2() / vars.to_f64().unwrap();

    if let DiffKind::Slice = a_diff_kind {
        cnt_a = measure_time!(
            diff_helper(
                a,
                arena,
                true,
                do_count,
                do_serialize,
                dimacs_path(".a").as_deref()
            )
            .0
        );
        cnt_a2 = measure_time!(
            diff_helper(
                &a2,
                arena,
                false,
                do_count,
                do_serialize,
                dimacs_path(".a2").as_deref()
            )
            .0
        );
        if a_vars > 0 {
            lost_ratio = ratio(cnt_a.clone(), cnt_a2.clone(), a_vars);
        }
    } else {
        no_duration!();
        no_duration!();
    }
    if let DiffKind::Slice = b_diff_kind {
        cnt_b = measure_time!(
            diff_helper(
                b,
                arena,
                true,
                do_count,
                do_serialize,
                dimacs_path(".b").as_deref()
            )
            .0
        );
        cnt_b2 = measure_time!(
            diff_helper(
                &b2,
                arena,
                false,
                do_count,
                do_serialize,
                dimacs_path(".b2").as_deref()
            )
            .0
        );
        if b_vars > 0 {
            gained_ratio = ratio(cnt_b.clone(), cnt_b2.clone(), b_vars);
        }
    } else {
        no_duration!();
        no_duration!();
    }

    let mut diff = a2.and(&b2, arena);
    measure_time!(diff.to_cnf_tseitin(false, arena));

    let (cnt_common, uvl_common, xml_common) = measure_time!(diff_helper(
        &diff.assume(arena.expr(And(vec![a2.root_id, b2.root_id])), arena),
        arena,
        false,
        do_count,
        do_serialize,
        dimacs_path(".common").as_deref()
    ));
    let (cnt_removed, uvl_removed, xml_removed) = measure_time!(diff_helper(
        &diff.assume(a2.and_not_expr(&b2, arena), arena),
        arena,
        false,
        do_count,
        do_serialize,
        dimacs_path(".removed").as_deref()
    ));
    let (cnt_added, uvl_added, xml_added) = measure_time!(diff_helper(
        &diff.assume(b2.and_not_expr(&a2, arena), arena),
        arena,
        false,
        do_count,
        do_serialize,
        dimacs_path(".added").as_deref()
    ));

    let cnt_union = &cnt_common + &cnt_removed + &cnt_added;
    let common_ratio = BigRational::new(cnt_common.clone(), cnt_union.clone())
        .to_f64()
        .unwrap();
    let removed_ratio = BigRational::new(cnt_removed.clone(), cnt_union.clone())
        .to_f64()
        .unwrap();
    let added_ratio = BigRational::new(cnt_added.clone(), cnt_union.clone())
        .to_f64()
        .unwrap();

    if write_files {
        io::write_uvl_and_xml(
            file_name(".common.left"),
            &a2_file.as_ref().unwrap().contents,
            &uvl_common.as_ref().unwrap(),
            &xml_common.as_ref().unwrap(),
        );
        io::write_uvl_and_xml(
            file_name(".common.right"),
            &b2_file.as_ref().unwrap().contents,
            &uvl_common.unwrap(),
            &xml_common.as_ref().unwrap(),
        );
        io::write_uvl_and_xml(
            file_name(".removed"),
            &a2_file.as_ref().unwrap().contents,
            &uvl_removed.as_ref().unwrap(),
            &xml_removed.as_ref().unwrap(),
        );
        io::write_uvl_and_xml(
            file_name(".added"),
            &b2_file.as_ref().unwrap().contents,
            &uvl_added.as_ref().unwrap(),
            &xml_added.as_ref().unwrap(),
        );
        if verbose {
            let durations: Vec<String> = durations
                .iter()
                .map(|duration| duration.as_nanos().to_string())
                .collect();
            let durations = durations.join(",");
            println!("{common_vars},{a_vars},{b_vars},{common_constraints},{a_constraints},{b_constraints},{lost_ratio},{removed_ratio},{common_ratio},{added_ratio},{gained_ratio},{cnt_a},{cnt_a2},{cnt_b},{cnt_b2},{cnt_common},{cnt_removed},{cnt_added},{durations}");
        }
    } else {
        let durations: Vec<String> = durations
            .iter()
            .map(|duration| duration.as_nanos().to_string())
            .collect();
        let durations = durations.join(",");
        println!(",{lost_ratio},{removed_ratio},{common_ratio},{added_ratio},{gained_ratio},{cnt_a},{cnt_a2},{cnt_b},{cnt_b2},{cnt_common},{cnt_removed},{cnt_added},{durations}");
    }
}
