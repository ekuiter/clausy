//! Defines a feature-model formula.

use num::{bigint::ToBigInt, BigInt, BigRational, ToPrimitive};

use crate::util::io;

use super::{
    arena::Arena,
    clauses::Clauses,
    expr::{Expr::*, ExprId},
    file::File,
    formula_ref::FormulaRef,
    var::{Var, VarId},
};
use std::{
    collections::HashSet,
    str::FromStr,
    time::{Duration, Instant},
};

/// Commands for computing differences of feature-model formulas.
pub(crate) enum DiffCommand {
    /// Computes the difference of both formulas, considering a solution as common if it satisfies both formulas.
    Strong,

    /// Computes the difference of both formulas, considering a solution as common if it satisfies the slices
    /// of both formulas down to their common variables.
    Weak,
}

/// A feature-model formula.
///
/// A [Formula] is a view onto part of an [Arena], which contains the actual implementation of most algorithms on formulas.
/// As such, a formula is given by the set of variables (defining its universe of solutions) as well as
/// the root expression of its syntax tree (constraining said variables).
/// We must store the variables to ensure correct results for certain feature-model analyses (e.g., model counting and slicing).
/// A formula is only a view and always implicitly tied to an [Arena].
#[derive(Clone)]
pub(crate) struct Formula {
    /// Specifies the sub-variables of this formula.
    ///
    /// Each identifiers serves as an index into [Arena::vars].
    /// All variables in the syntax tree of this formula must occur in this set, but it may contain more (unconstrained) variables.
    pub(crate) sub_var_ids: HashSet<VarId>,

    /// Specifies the root expression of this formula.
    ///
    /// Serves as an index into [Arena::exprs].
    /// The corresponding expression is the root of this formula's syntax tree and thus the starting point for most algorithms.
    /// We consider all expressions below this expression (including itself) to be sub-expressions.
    /// There might be other (non-sub-)expressions that are currently not relevant to this formula.
    pub(crate) root_id: ExprId,

    /// The file this formula was originally parsed from, if any.
    pub(crate) file: Option<File>,
}

impl Formula {
    /// Creates a new formula.
    ///
    /// The sub-variable and root expression identifiers must be valid in the context of some given [Arena].
    pub(crate) fn new(sub_var_ids: HashSet<VarId>, root_id: ExprId, file: Option<File>) -> Self {
        Self {
            sub_var_ids,
            root_id,
            file,
        }
    }

    /// Returns a shared reference to this formula in the context of its arena.
    pub(crate) fn as_ref<'a>(&'a self, arena: &'a Arena) -> FormulaRef {
        FormulaRef {
            arena,
            formula: self,
        }
    }

    /// Returns the clause representation of this formula.
    pub(crate) fn to_clauses(&self, arena: &Arena) -> Clauses {
        Clauses::from(self.as_ref(arena))
    }

    /// Resets the root expression of this formula, if necessary.
    ///
    /// If the root expression is mutated with [Arena::set_expr], structural sharing might be violated.
    /// Because [Arena::set_expr] can only address this issue for children,
    /// we need to explicitly address the only expression that is not a child itself - the root expression.
    pub(super) fn reset_root_expr(arena: &Arena, root_id: &mut ExprId) {
        *root_id = arena.get_expr(&arena.exprs[*root_id]).unwrap();
    }

    /// Returns all sub-variables of this formula and their identifiers.
    pub(crate) fn sub_vars(&self, arena: &Arena) -> Vec<(VarId, Var)> {
        arena.vars(|var_id, _| self.sub_var_ids.contains(&var_id))
    }

    /// Returns all sub-variable identifiers of this formula also in another formula.
    pub(crate) fn common_vars(&self, other: &Formula) -> HashSet<VarId> {
        self.sub_var_ids
            .intersection(&other.sub_var_ids)
            .map(|id| *id)
            .collect::<HashSet<VarId>>()
    }

    /// Returns all sub-variable identifiers of this formula or in another formula.
    pub(crate) fn all_vars(&self, other: &Formula) -> HashSet<VarId> {
        self.sub_var_ids
            .union(&other.sub_var_ids)
            .map(|id| *id)
            .collect::<HashSet<VarId>>()
    }

    /// Returns all sub-variable identifiers of this formula not in another formula.
    pub(crate) fn except_vars(&self, other: &Formula) -> HashSet<VarId> {
        self.sub_var_ids
            .difference(&other.sub_var_ids)
            .map(|id| *id)
            .collect::<HashSet<VarId>>()
    }

    /// Returns the identifiers of all sub-expressions of this formula.
    ///
    /// If in canonical form, each identifier is guaranteed to appear only once.
    pub(crate) fn sub_exprs(&mut self, arena: &mut Arena) -> Vec<ExprId> {
        let mut sub_exprs = Vec::<ExprId>::new();
        arena.preorder_rev(&mut self.root_id, |_, id| sub_exprs.push(id));
        sub_exprs
    }

    /// Panics if structural sharing is violated in this formula.
    ///
    /// That is, we assert that every sub-expression's identifier is indeed the canonical one.
    /// Does not currently check for other properties of canonicity (see [Formula::to_canon]).
    #[cfg(debug_assertions)]
    pub(crate) fn assert_canon(&mut self, arena: &mut Arena) {
        arena.preorder_rev(&mut self.root_id, |arena, id| {
            debug_assert_eq!(arena.get_expr(&arena.exprs[id]).unwrap(), id)
        });
    }

    /// Transforms this formula into canonical form (see [Arena::canon_visitor]).
    ///
    /// The resulting formula is logically equivalent to the original formula.
    /// This function is useful when an algorithm assumes or profits from canonical form, or for simplifying a formula after parsing.
    /// In canonical form, several useful guarantees hold:
    /// First, no sub-expression occurs twice in the syntax tree with different identifiers (structural sharing).
    /// Second, equality of sub-expressions is up to commutativity, idempotency, and unary expressions, and those expressions are simplified.
    /// Third, no `And` expression is below an `And` expression (and analogously for `Or`).
    /// Fourth, no `Not` expression is below a `Not` expression.
    /// To ensure these guarantees, this visitor must be called in a postorder traversal, preorder does not work.
    pub(crate) fn to_canon(&mut self, arena: &mut Arena) {
        arena.postorder_rev(&mut self.root_id, Arena::canon_visitor);
    }

    /// Transforms this formula into canonical negation normal form by applying De Morgan's laws (see [Arena::nnf_visitor]).
    ///
    /// The resulting formula is logically equivalent to the original formula.
    /// We do this by traversing the formula top-down, meanwhile, we push negations towards the leaves (i.e., [Var] expressions).
    /// Double negations cannot be encountered, as they have already been removed by [Arena::simp_expr].
    pub(crate) fn to_nnf(&mut self, arena: &mut Arena) {
        arena.prepostorder_rev(&mut self.root_id, Arena::nnf_visitor, Arena::canon_visitor);
    }

    /// Transforms this formula into canonical conjunctive normal form by applying distributivity laws (see [Arena::cnf_dist_visitor]).
    ///
    /// The resulting formula is logically equivalent to the original formula.
    /// We do this by traversing the formula bottom-up and pushing [Or] expressions below [And] expressions via multiplication.
    /// This algorithm has exponential worst-case complexity, but ensures logical equivalence to the original formula.
    pub(crate) fn to_cnf_dist(&mut self, arena: &mut Arena) {
        arena.prepostorder_rev(
            &mut self.root_id,
            Arena::nnf_visitor,
            Arena::cnf_dist_visitor,
        );
    }

    /// Transforms this formula into canonical conjunctive normal form by introducing auxiliary variables (see [Arena::cnf_tseitin_visitor]).
    ///
    /// The resulting formula is equivalent to the original formula projected onto its named variables (i.e., satisfiability and model count are preserved).
    /// If this formula is in canonical form (see [Formula::to_canon]), we introduce exactly one auxiliary variable per (complex) sub-expression.
    /// Thus, every sub-expression will be "abbreviated" with an auxiliary variable, including the root expression, which facilitates algebraic operations.
    /// Also, no sub-expression will be abbreviated twice, so the number of auxiliary variables is equal to the number of sub-expressions.
    /// If this formula is not in canonical form, more auxiliary variables might be introduced.
    /// Note that we only abbreviate complex sub-expressions (i.e., [And] and [Or] expressions), as [Var] and [Not] expressions do not profit from abbrevation.
    pub(crate) fn to_cnf_tseitin(&mut self, assume_root: bool, arena: &mut Arena) {
        arena.new_vars = Some(vec![]);
        arena.new_exprs = Some(vec![]);
        arena.postorder_rev(&mut self.root_id, Arena::cnf_tseitin_visitor);
        self.sub_var_ids.extend(arena.new_vars.take().unwrap());
        if assume_root {
            arena.new_exprs.as_mut().unwrap().push(self.root_id);
        }
        let new_expr = And(arena.new_exprs.take().unwrap());
        let root_id = arena.expr(new_expr);
        self.root_id = root_id;
    }

    /// Returns a formula that only contains constraints of this formula that do not contain any given variable.
    ///
    /// Assumes that this formula is in proto-CNF; that is, it is a conjunction of constraints.
    /// Does not modify this formula.
    pub(crate) fn remove_constraints(&self, ids: &HashSet<VarId>, arena: &mut Arena) -> Formula {
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

    /// Asserts that this formula is in proto-CNF; that is, it is a non-empty conjunction of constraints.
    pub(crate) fn assert_proto_cnf(&self, arena: &Arena) {
        if let And(child_ids) = &arena.exprs[self.root_id] {
            if child_ids.is_empty() {
                panic!()
            }
        } else {
            panic!()
        }
    }

    /// Returns the identifiers of all variables common and unique to this and a given formula.
    pub(crate) fn diff_vars(
        &self,
        other: &Formula,
    ) -> (HashSet<VarId>, HashSet<VarId>, HashSet<VarId>) {
        (
            self.common_vars(other),
            self.except_vars(other),
            other.except_vars(self),
        )
    }

    /// Returns the identifiers of all constraints common and unique to this and a given formula.
    ///
    /// Assumes that this formula is in proto-CNF; that is, it is a conjunction of constraints.
    pub(crate) fn diff_constraints(
        &self,
        other: &Formula,
        arena: &mut Arena,
    ) -> (HashSet<ExprId>, HashSet<ExprId>, HashSet<ExprId>) {
        if let And(child_ids) = &arena.exprs[self.root_id] {
            if let And(other_child_ids) = &arena.exprs[other.root_id] {
                let child_ids: HashSet<ExprId> = child_ids.clone().into_iter().collect();
                let other_child_ids: HashSet<ExprId> =
                    other_child_ids.clone().into_iter().collect();
                let common_constraint_ids = child_ids
                    .intersection(&other_child_ids)
                    .into_iter()
                    .map(|id| *id)
                    .collect();
                let a_constraint_ids = child_ids
                    .difference(&other_child_ids)
                    .into_iter()
                    .map(|id| *id)
                    .collect();
                let b_constraint_ids = other_child_ids
                    .difference(&child_ids)
                    .into_iter()
                    .map(|id| *id)
                    .collect();
                (common_constraint_ids, a_constraint_ids, b_constraint_ids)
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        }
    }

    /// Returns a formula that assumes an additional constraint.
    pub(crate) fn assume(&mut self, id: ExprId, arena: &mut Arena) -> Formula {
        let mut expr = And(vec![self.root_id, id]);
        arena.flatten_expr(&mut expr);
        Formula::new(self.sub_var_ids.clone(), arena.expr(expr), None)
    }

    /// Returns an expression that encodes the common solutions of this and another formula.
    pub(crate) fn and_expr(&self, other: &Formula, arena: &mut Arena) -> ExprId {
        arena.expr(And(vec![self.root_id, other.root_id]))
    }

    /// Returns a formula that encodes the common solutions of this and another formula.
    ///
    /// Does not modify this formula.
    pub(crate) fn and(&self, other: &Formula, arena: &mut Arena) -> Formula {
        Formula::new(self.all_vars(other), self.and_expr(other, arena), None)
    }

    /// Returns an expression that encodes whether this formula implies another formula.
    ///
    /// Also encodes solutions gone in the other formula, if any.
    pub(crate) fn implies_expr(&self, other: &Formula, arena: &mut Arena) -> ExprId {
        let not_other = arena.expr(Not(other.root_id));
        arena.expr(And(vec![self.root_id, not_other]))
    }

    /// Returns a formula that encodes whether this formula implies another formula.
    ///
    /// Does not modify this formula.
    pub(crate) fn implies(&self, other: &Formula, arena: &mut Arena) -> Formula {
        Formula::new(self.all_vars(other), self.implies_expr(other, arena), None)
    }

    /// Returns the number of solutions of this formula.
    ///
    /// Optionally uses a Tseitin transformation into CNF.
    /// Does not modify this formula or the given arena.
    pub(crate) fn count(
        &self,
        arena: &Arena,
        use_tseitin: bool,
        serialize: bool,
    ) -> (BigInt, Option<String>, Option<String>) {
        let mut clone = self.clone();
        let mut arena = arena.clone();
        if use_tseitin {
            clone.to_cnf_tseitin(true, &mut arena);
        }
        clone.to_clauses(&arena).count(serialize)
    }

    /// Returns a mathematical term that, given the number of solutions of this formula, calculates the number of solutions of another formula.
    pub(crate) fn count_inc(
        &self,
        b: &Formula,
        left_model_count: Option<&str>,
        arena: &mut Arena,
    ) -> String {
        let a = self;
        a.assert_proto_cnf(arena);
        b.assert_proto_cnf(arena);
        let left_model_count = left_model_count.map(|argument| BigInt::from_str(argument).unwrap());
        let (_, a_var_ids, b_var_ids) = a.diff_vars(b);
        let a_vars: u32 = a_var_ids.len().try_into().unwrap();
        let b_vars: u32 = b_var_ids.len().try_into().unwrap();
        let a2 = a.remove_constraints(&a_var_ids, arena);
        let b2 = b.remove_constraints(&b_var_ids, arena);
        let cnt_a2_to_a = a2.implies(a, arena).count(arena, true, false).0;
        let cnt_b2_to_b = b2.implies(b, arena).count(arena, true, false).0;
        let mut diff = a2.and(&b2, arena);
        diff.to_cnf_tseitin(false, arena);
        let (cnt_removed, _, _) = diff
            .assume(a2.implies_expr(&b2, arena), arena)
            .to_clauses(&arena)
            .count(false);
        let (cnt_added, _, _) = diff
            .assume(b2.implies_expr(&a2, arena), arena)
            .to_clauses(&arena)
            .count(false);
        if left_model_count.is_some() {
            let two = 2.to_bigint().unwrap();
            format!(
                "{}",
                (((&left_model_count.unwrap() + &cnt_a2_to_a) / two.pow(a_vars)) - &cnt_removed
                    + &cnt_added)
                    * two.pow(b_vars)
                    - &cnt_b2_to_b
            )
        } else {
            format!("(((#+{cnt_a2_to_a})/2^{a_vars})-{cnt_removed}+{cnt_added})*2^{b_vars}-{cnt_b2_to_b}# | sed 's/#/<left model count>/' | bc")
        }
    }

    /// Returns a description of the difference between this formula and another.
    pub(crate) fn diff(
        &self,
        b: &Formula,
        command: DiffCommand,
        argument: Option<&str>,
        arena: &mut Arena,
    ) -> String {
        let a = self;
        a.assert_proto_cnf(arena);
        b.assert_proto_cnf(arena);
        let verbose = argument.is_some() && !argument.unwrap().is_empty();
        let file_name = |name: &str| format!("{}{}", argument.unwrap().to_string(), name);
        let (common_var_ids, a_var_ids, b_var_ids) = a.diff_vars(b);
        let (common_constraint_ids, a_constraint_ids, b_constraint_ids) =
            self.diff_constraints(b, arena);
        let common_vars: u32 = common_var_ids.len().try_into().unwrap();
        let a_vars: u32 = a_var_ids.len().try_into().unwrap();
        let b_vars: u32 = b_var_ids.len().try_into().unwrap();
        let common_constraints: u32 = common_constraint_ids.len().try_into().unwrap();
        let a_constraints: u32 = a_constraint_ids.len().try_into().unwrap();
        let b_constraints: u32 = b_constraint_ids.len().try_into().unwrap();
        if verbose {
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
        let a2;
        let b2;
        let mut a2_uvl = None;
        let mut b2_uvl = None;
        match command {
            DiffCommand::Strong => {
                a2 = a.clone(); // todo
                b2 = b.clone();
            }
            DiffCommand::Weak => {
                (a2, a2_uvl) = measure_time!(a.file.as_ref().unwrap().slice_featureide(
                    &common_var_ids,
                    arena,
                    verbose
                ));
                (b2, b2_uvl) = measure_time!(b.file.as_ref().unwrap().slice_featureide(
                    &common_var_ids,
                    arena,
                    verbose
                ));
            }
        }
        let mut cnt_a = -1.to_bigint().unwrap();
        let mut cnt_b = -1.to_bigint().unwrap();
        let mut cnt_a2 = -1.to_bigint().unwrap();
        let mut cnt_b2 = -1.to_bigint().unwrap();
        match command {
            DiffCommand::Strong => {}
            DiffCommand::Weak => {
                cnt_a = measure_time!(a.count(arena, true, false).0);
                cnt_a2 = measure_time!(a2.count(arena, false, false).0);
                cnt_b = measure_time!(b.count(arena, true, false).0);
                cnt_b2 = measure_time!(b2.count(arena, false, false).0);
            }
        }
        let mut diff = a2.and(&b2, arena);
        measure_time!(diff.to_cnf_tseitin(false, arena));
        let (cnt_common, uvl_common, xml_common) = measure_time!(diff
            .assume(arena.expr(And(vec![a2.root_id, b2.root_id])), arena)
            .to_clauses(&arena)
            .count(verbose));
        let (cnt_removed, uvl_removed, xml_removed) = measure_time!(diff
            .assume(a2.implies_expr(&b2, arena), arena)
            .to_clauses(&arena)
            .count(verbose));
        let (cnt_added, uvl_added, xml_added) = measure_time!(diff
            .assume(b2.implies_expr(&a2, arena), arena)
            .to_clauses(&arena)
            .count(verbose));
        let mut lost_ratio = -1f64;
        let mut gained_ratio = -1f64;
        // this currently only supports deleting/adding up to 1000 features due to f64 precision
        if a_vars > 0 {
            lost_ratio = BigRational::new(cnt_a.clone(), cnt_a2.clone())
                .to_f64()
                .unwrap()
                .log2()
                / a_vars.to_f64().unwrap();
        }
        if b_vars > 0 {
            gained_ratio = BigRational::new(cnt_b.clone(), cnt_b2.clone())
                .to_f64()
                .unwrap()
                .log2()
                / b_vars.to_f64().unwrap();
        }
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
        if verbose {
            io::write_uvl_and_xml(
                file_name(".common.left"),
                &a2_uvl.as_ref().unwrap().contents,
                &uvl_common.as_ref().unwrap(),
                &xml_common.as_ref().unwrap(),
            );
            io::write_uvl_and_xml(
                file_name(".common.right"),
                &b2_uvl.as_ref().unwrap().contents,
                &uvl_common.unwrap(),
                &xml_common.as_ref().unwrap(),
            );
            io::write_uvl_and_xml(
                file_name(".removed"),
                &a2_uvl.as_ref().unwrap().contents,
                &uvl_removed.as_ref().unwrap(),
                &xml_removed.as_ref().unwrap(),
            );
            io::write_uvl_and_xml(
                file_name(".added"),
                &b2_uvl.as_ref().unwrap().contents,
                &uvl_added.as_ref().unwrap(),
                &xml_added.as_ref().unwrap(),
            );
        }
        let durations: Vec<String> = durations
            .iter()
            .map(|duration| duration.as_nanos().to_string())
            .collect();
        let durations = durations.join(",");
        format!("{common_vars},{a_vars},{b_vars},{common_constraints},{a_constraints},{b_constraints},{lost_ratio},{removed_ratio},{common_ratio},{added_ratio},{gained_ratio},{cnt_a},{cnt_b},{cnt_a2},{cnt_b2},{cnt_common},{cnt_removed},{cnt_added},{durations}")
    }
}
