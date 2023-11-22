//! Defines a feature-model formula.

use num_bigint::BigUint;

use super::{
    arena::Arena,
    clauses::Clauses,
    expr::{Expr::*, ExprId},
    file::File,
    formula_ref::FormulaRef,
    var::{Var, VarId},
};
use std::collections::HashSet;

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

    /// Adds a constraint to this formula.
    pub(crate) fn add_constraint(&mut self, id: ExprId, arena: &mut Arena) {
        let mut expr = And(vec![self.root_id, id]);
        arena.flatten_expr(&mut expr);
        self.root_id = arena.expr(expr);
    }

    /// Removes a constraint from this formula.
    ///
    /// Assumes that this formula is in proto-CNF; that is, it is a conjunction of constraints.
    /// Useful for removing the root constraint of a Tseitin-transformed formula (see [Formula::to_cnf_tseitin]).
    pub(crate) fn remove_constraint(&mut self, id: ExprId, arena: &mut Arena) {
        if let And(child_ids) = &arena.exprs[self.root_id] {
            let expr = And(child_ids
                .clone()
                .into_iter()
                .filter(|_id| *_id != id)
                .collect());
            self.root_id = arena.expr(expr);
        } else {
            unreachable!()
        }
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
    pub(crate) fn to_cnf_tseitin(&mut self, arena: &mut Arena) -> ExprId {
        arena.new_vars = Some(vec![]);
        arena.new_exprs = Some(vec![]);
        arena.postorder_rev(&mut self.root_id, Arena::cnf_tseitin_visitor);
        self.sub_var_ids.extend(arena.new_vars.take().unwrap());
        let old_root_id = self.root_id;
        arena.new_exprs.as_mut().unwrap().push(old_root_id);
        let new_expr = And(arena.new_exprs.take().unwrap());
        let root_id = arena.expr(new_expr);
        self.root_id = root_id;
        old_root_id
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
            let root_id = arena.expr(And(new_child_ids));
            Self::new(self.sub_var_ids.clone(), root_id, None)
        } else {
            unreachable!()
        }
    }

    /// Returns an expression that encodes whether this formula implies another formula
    ///
    /// Also encodes solutions gone in the other formula, if any.
    pub(crate) fn implies_expr(&self, other: &Formula, arena: &mut Arena) -> ExprId {
        let not_other = arena.expr(Not(other.root_id));
        let root_id = arena.expr(And(vec![self.root_id, not_other]));
        root_id
    }

    /// Returns a formula that encodes whether this formula implies another formula.
    ///
    /// Does not modify this formula.
    pub(crate) fn implies(&self, other: &Formula, arena: &mut Arena) -> Formula {
        Formula::new(self.all_vars(other), self.implies_expr(other, arena), None)
    }

    /// Returns a description of the difference between this formula's count and another's by means of a pseudo-slice.
    pub(crate) fn count_diff_slice(
        &self,
        b: &Formula,
        arena: &mut Arena,
    ) -> (BigUint, u32, BigUint, BigUint, u32, BigUint) {
        let a = self;
        let a_var_ids = a.except_vars(b);
        let b_var_ids = b.except_vars(a);
        let common_var_ids = a.common_vars(b);
        let mut a2 = a
            .file
            .as_ref()
            .unwrap()
            .slice_featureide(&common_var_ids, arena);
        let mut b2 = b
            .file
            .as_ref()
            .unwrap()
            .slice_featureide(&common_var_ids, arena);
        a2.sub_var_ids = common_var_ids.clone();
        b2.sub_var_ids = common_var_ids.clone();
        let a2_to_a;
        let a_vars: u32 = a_var_ids.len().try_into().unwrap();
        let removed;
        let added;
        let b_vars: u32 = b_var_ids.len().try_into().unwrap();
        let b2_to_b;
        {
            let mut arena = arena.clone();
            let mut diff = a2.implies(a, &mut arena);
            diff.to_cnf_tseitin(&mut arena);
            a2_to_a = diff.to_clauses(&arena).count();
        }
        {
            let mut arena = arena.clone();
            let mut diff = a2.implies(&b2, &mut arena);
            diff.to_cnf_tseitin(&mut arena);
            removed = diff.to_clauses(&arena).count();
        }
        // todo: do not clone arena, modify root id instead
        {
            let mut arena = arena.clone();
            let mut diff = b2.implies(&a2, &mut arena);
            diff.to_cnf_tseitin(&mut arena);
            added = diff.to_clauses(&arena).count();
        }
        {
            let mut arena = arena.clone();
            let mut diff = b2.implies(b, &mut arena);
            diff.to_cnf_tseitin(&mut arena);
            b2_to_b = diff.to_clauses(&arena).count();
        }
        (a2_to_a, a_vars, removed, added, b_vars, b2_to_b)
    }

    /// Returns a description of the difference between this formula's count and another's by means of a pseudo-slice.
    pub(crate) fn count_diff_pseudo_slice(
        &self,
        b: &Formula,
        diff_only: bool,
        arena: &mut Arena,
    ) -> (BigUint, u32, BigUint, BigUint, u32, BigUint) {
        let a = self;
        let a_var_ids = a.except_vars(b);
        let b_var_ids = b.except_vars(a);
        let common_var_ids = a.common_vars(b);
        let mut a2 = a.remove_constraints(&a_var_ids, arena);
        let mut b2 = b.remove_constraints(&b_var_ids, arena);
        let a2_to_a;
        let a_vars: u32 = a_var_ids.len().try_into().unwrap();
        let removed;
        let added;
        let b_vars: u32 = b_var_ids.len().try_into().unwrap();
        let b2_to_b;
        if diff_only {
            {
                let mut arena = arena.clone();
                let mut diff = a2.implies(a, &mut arena);
                diff.to_cnf_tseitin(&mut arena);
                a2_to_a = diff.to_clauses(&arena).count();
            }
            {
                let mut arena = arena.clone();
                let mut diff = b2.implies(b, &mut arena);
                diff.to_cnf_tseitin(&mut arena);
                b2_to_b = diff.to_clauses(&arena).count();
            }
        } else {
            let mut tmp;
            {
                let mut arena = arena.clone();
                let mut a2 = a2.clone();
                a2.to_cnf_tseitin(&mut arena);
                tmp = a2.to_clauses(&arena).count();
            }
            {
                let mut arena = arena.clone();
                let mut a = a.clone();
                a.to_cnf_tseitin(&mut arena);
                a2_to_a = tmp - a.to_clauses(&arena).count();
            }
            {
                let mut arena = arena.clone();
                let mut b2 = b2.clone();
                b2.to_cnf_tseitin(&mut arena);
                tmp = b2.to_clauses(&arena).count();
            }
            {
                let mut arena = arena.clone();
                let mut b = b.clone();
                b.to_cnf_tseitin(&mut arena);
                b2_to_b = tmp - b.to_clauses(&arena).count();
            }
        }
        a2.sub_var_ids = common_var_ids.clone();
        b2.sub_var_ids = common_var_ids.clone();
        let mut diff = a2.implies(&b2, arena);
        let root_id = diff.to_cnf_tseitin(arena);
        removed = diff.to_clauses(&arena).count();
        diff.remove_constraint(root_id, arena);
        diff.add_constraint(b2.implies_expr(&a2, arena), arena);
        added = diff.to_clauses(&arena).count();
        (a2_to_a, a_vars, removed, added, b_vars, b2_to_b)
    }
}
