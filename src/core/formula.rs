//! Defines a feature-model formula.

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
    pub(crate) fn as_ref<'a>(&'a self, arena: &'a Arena) -> FormulaRef<'a> {
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
    pub(super) fn reset_root_expr(root_id: &mut ExprId, arena: &Arena) {
        *root_id = arena.get_expr(&arena.exprs[*root_id]).unwrap();
    }

    /// Returns a formula that forces all variables only occurring in the given arena to a fixed value.
    ///
    /// Variables in `core_vars` are forced to true, variables in `dead_vars` are forced to false,
    /// and all remaining foreign variables not in `sub_var_ids` or `exclude_vars` are forced to `default`.
    /// Does not modify this formula.
    pub(crate) fn force_foreign_vars(
        &self,
        default: bool,
        core_vars: &HashSet<VarId>,
        dead_vars: &HashSet<VarId>,
        exclude_vars: &HashSet<VarId>,
        arena: &mut Arena,
    ) -> Formula {
        let mut ids;
        if let And(child_ids) = &arena.exprs[self.root_id] {
            ids = child_ids.clone();
        } else {
            ids = vec![self.root_id];
        }
        let mut remaining_core = core_vars.clone();
        let mut remaining_dead = dead_vars.clone();
        ids.extend(
            arena
                .vars(|var_id, _| {
                    !self.sub_var_ids.contains(&var_id) && !exclude_vars.contains(&var_id)
                })
                .into_iter()
                .map(|(var_id, _)| {
                    let top = if remaining_core.remove(&var_id) {
                        true
                    } else if remaining_dead.remove(&var_id) {
                        false
                    } else {
                        default
                    };
                    let expr = arena.expr(Var(var_id));
                    if top {
                        expr
                    } else {
                        arena.expr(Not(expr))
                    }
                }),
        );
        assert!(
            remaining_core.is_empty(),
            "core_vars contained variables that are not foreign to this formula: {}",
            arena.var_strs(&remaining_core).join(", ")
        );
        assert!(
            remaining_dead.is_empty(),
            "dead_vars contained variables that are not foreign to this formula: {}",
            arena.var_strs(&remaining_dead).join(", ")
        );
        let sub_var_ids = arena
            .var_ids()
            .difference(exclude_vars)
            .map(|var| *var)
            .collect();
        let root_id = arena.expr(And(ids));
        Self::new(sub_var_ids, root_id, None)
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
    /// To ensure these guarantees, [Arena::canon_visitor] must be called in a postorder traversal, preorder does not work.
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
    /// Usually, we want `assume_root` to express that the formula should be true.
    /// However, for negation or other algebraic operations, we might not want to assume the root
    /// and assume another expression instead.
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

    /// Asserts that this formula is in proto-CNF; that is, it is a non-empty conjunction of constraints.
    pub(crate) fn assert_proto_cnf(&self, arena: &Arena) {
        if let And(child_ids) = &arena.exprs[self.root_id] {
            if child_ids.is_empty() {
                panic!("formula is empty, thus not in proto-CNF");
            }
        } else {
            panic!("formula is not a conjunction, thus not in proto-CNF");
        }
    }

    /// Forces this formula to be in proto-CNF.
    ///
    /// [Arena::simp_expr] may reduce `And([x])` to `x`.
    /// This function forcibly wraps the root back into an `And`.
    /// This violates canonicity, as canonical form forbids the expression `And([x])`.
    pub(crate) fn ensure_proto_cnf(&mut self, arena: &mut Arena) {
        if !matches!(arena.exprs[self.root_id], And(_)) {
            self.root_id = arena.add_expr(And(vec![self.root_id]));
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
    /// Results are most accurate if both formulas are in canonical form.
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

    /// Renames a variable in this formula's syntax tree and variable set.
    ///
    /// Replaces all occurrences of `old_var_id` with `new_var_id` in the syntax tree,
    /// and updates `sub_var_ids` accordingly.
    pub(crate) fn rename_var(&mut self, old_var_id: VarId, new_var_id: VarId, arena: &mut Arena) {
        arena.rename_var(&mut self.root_id, old_var_id, new_var_id);
        self.sub_var_ids.remove(&old_var_id);
        self.sub_var_ids.insert(new_var_id);
    }

    /// Adds an equivalence constraint between two variables to this formula.
    ///
    /// Appends `And(Or(Not(v1), v2), Or(Not(v2), v1))` as two new clauses to the root conjunction.
    /// Both variables are added to [Formula::sub_var_ids] if not already present.
    pub(crate) fn and_equivalent(&mut self, var1_id: VarId, var2_id: VarId, arena: &mut Arena) {
        let v1 = arena.expr(Var(var1_id));
        let v2 = arena.expr(Var(var2_id));
        let not_v1 = arena.expr(Not(v1));
        let not_v2 = arena.expr(Not(v2));
        let clause1 = arena.expr(Or(vec![not_v1, v2]));
        let clause2 = arena.expr(Or(vec![not_v2, v1]));
        let mut root_children = match &arena.exprs[self.root_id] {
            And(children) => children.clone(),
            _ => vec![self.root_id],
        };
        root_children.push(clause1);
        root_children.push(clause2);
        self.root_id = arena.expr(And(root_children));
        self.sub_var_ids.insert(var1_id);
        self.sub_var_ids.insert(var2_id);
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

    /// Returns an expression that encodes whether this formula does not imply another formula.
    ///
    /// Also encodes solutions gone in the other formula, if any.
    /// If this expression is unsatisfiable, this formula implies the other formula.
    pub(crate) fn and_not_expr(&self, other: &Formula, arena: &mut Arena) -> ExprId {
        let not_other = arena.expr(Not(other.root_id));
        arena.expr(And(vec![self.root_id, not_other]))
    }

    /// Returns a formula that encodes whether this formula does not imply another formula.
    ///
    /// Does not modify this formula.
    pub(crate) fn and_not(&self, other: &Formula, arena: &mut Arena) -> Formula {
        Formula::new(self.all_vars(other), self.and_not_expr(other, arena), None)
    }
}
