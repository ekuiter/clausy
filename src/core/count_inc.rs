//! Incremental model counting for feature-model formulas.

use num::{bigint::ToBigInt, BigInt, Signed};
use std::{collections::HashSet, str::FromStr};

use crate::util::log::log;

use super::{arena::Arena, expr::Expr::And, formula::Formula, var::VarId};

impl Formula {
    /// Returns a formula that only contains constraints of this formula that do not contain any given variable.
    ///
    /// Assumes that this formula is in proto-CNF; that is, it is a conjunction of constraints.
    /// This is not a slice, but similar in that the resulting formula no longer mentions the removed variables.
    /// Does not modify this formula.
    pub(crate) fn pseudo_slice(&self, ids: &HashSet<VarId>, arena: &mut Arena) -> Formula {
        if let And(child_ids) = &arena.exprs[self.root_id] {
            let new_child_ids = child_ids
                .clone()
                .into_iter()
                .filter(|child_id| !arena.contains_var(*child_id, ids))
                .collect();
            let sub_var_ids = self
                .sub_var_ids
                .difference(ids)
                .map(|id| *id)
                .collect::<HashSet<VarId>>();
            let root_id = arena.expr(And(new_child_ids));
            Self::new(sub_var_ids, root_id, None)
        } else {
            unreachable!()
        }
    }
}

/// Returns the number of solutions of a given formula.
///
/// Optionally uses a Tseitin transformation into CNF on a cloned [Formula] and [Arena].
/// Does not modify the given [Formula] or [Arena].
pub(crate) fn count_helper(formula: &Formula, arena: &Arena, tseitin_transform: bool) -> BigInt {
    let clauses;
    if tseitin_transform {
        let mut clone = formula.clone();
        let mut arena = arena.clone();
        clone.to_cnf_tseitin(true, &mut arena);
        clauses = clone.to_clauses(&arena);
    } else {
        clauses = formula.to_clauses(arena);
    }
    let count = clauses.count();
    if count.is_negative() {
        log(&format!(
            "[COUNT_INC] timeout while counting number of solutions for partial result"
        ));
    }
    count
}

/// Returns a mathematical term that, given the number of solutions of this formula, calculates the number of solutions of another formula.
///
/// See documentation in [crate::shell::Action] as well as inline comments.
/// The idea is nice, but it does not scale to a time series of formulas,
/// because the differences between formulas also grow exponentially if the formulas themselves grow exponentially.
pub(crate) fn count_inc(
    a: &Formula,
    b: &Formula,
    cnt_a: Option<&str>,
    arena: &mut Arena,
) -> String {
    a.assert_proto_cnf(arena);
    b.assert_proto_cnf(arena);

    // Parse the left model count if provided.
    let cnt_a = cnt_a.map(|argument| BigInt::from_str(argument).unwrap());

    // Compute the variables unique to each formula, and how many there are of them.
    let (_, a_var_ids, b_var_ids) = a.diff_vars(b);
    let a_vars: u32 = a_var_ids.len().try_into().unwrap();
    let b_vars: u32 = b_var_ids.len().try_into().unwrap();

    // Remove constraints from both formulas that mention their unique variables.
    // We do this so we can split up the counting into three steps, which we hypothesize could be a bit easier.
    // The trick is that all three counting steps are performed on compatible variable sets.
    // Alternatively, we could also use slices here, but they are expensive to compute,
    // and here we only care about the property that a and a2 must be sensible on the same variable set,
    // but a2 is not allowed to reference any variables that are only in a and not in b,
    // not the actual semantics of the slicing operator. Hence, we call it a pseudo slice.
    let a2 = a.pseudo_slice(&a_var_ids, arena);
    let b2 = b.pseudo_slice(&b_var_ids, arena);

    // Step 1: Count the difference between the left formula and its reduced version.
    // Both formulas are defined on the set of variables of the left formula.
    let cnt_a2_to_a = count_helper(&a2.and_not(a, arena), arena, true);

    // Step 2: Count the difference between the right formula and its reduced version.
    // Both formulas are defined on the set of variables of the right formula.
    let cnt_b2_to_b = count_helper(&b2.and_not(b, arena), arena, true);

    // Step 3: Count the difference between the reduced versions of both formulas.
    // Both formulas are defined on the set of variables common to both formulas.
    let mut diff = a2.and(&b2, arena);
    // We need to ensure the formula is in CNF form, but not assume that it is true.
    diff.to_cnf_tseitin(false, arena);

    // We split up this difference into the removed and added satisfying assignments.
    // Because the formula has already been Tseitin-transformed,
    // we can just assume two expressions on the root literals here.
    let cnt_removed = count_helper(
        &diff.assume(a2.and_not_expr(&b2, arena), arena),
        arena,
        false,
    );
    let cnt_added = count_helper(
        &diff.assume(b2.and_not_expr(&a2, arena), arena),
        arena,
        false,
    );

    // We have all the results now.
    // Depending on whether a left model count was given,
    // we output a formula for computing the right model count, or we apply the formula directly.
    if cnt_a.is_some() {
        let two = 2.to_bigint().unwrap();
        // a_vars is free in a2, so cnt_a2*2^a_vars = cnt_a + cnt_a2_to_a
        let cnt_a2 = (&cnt_a.unwrap() + &cnt_a2_to_a) / two.pow(a_vars);
        // this is just simple set arithmetic on common variables
        let cnt_b2 = cnt_a2 - &cnt_removed + &cnt_added;
        // inverse of step 1: cnt_b2*2^b_vars = cnt_b + cnt_b2_to_b
        let cnt_b = cnt_b2 * two.pow(b_vars) - &cnt_b2_to_b;
        format!("{}", cnt_b)
    } else {
        format!("(((#+{cnt_a2_to_a})/2^{a_vars})-{cnt_removed}+{cnt_added})*2^{b_vars}-{cnt_b2_to_b}# | sed 's/#/<left model count>/' | bc")
    }
}
