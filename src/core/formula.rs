//! Data structures and algorithms for feature-model formulas.

#![allow(unused_imports, rustdoc::private_intra_doc_links)]

use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    slice,
};
use Expr::*;

use crate::shell::{PRINT_ID, VAR_AUX_PREFIX};

use super::{arena::Arena, expr::{Expr, ExprId}, var::{Var, VarId}};

/// An expression that is explicitly paired with the formula it is tied to.
///
/// This struct is useful whenever we need to pass an expression around, but the containing formula is not available.
/// Using this might be necessary when there is no `self` of type [Formula], for example whenever we want to [fmt::Display] an expression.
pub(crate) struct Formula {
    /// Specifies the root expression of this formula.
    ///
    /// Serves as an index into [Formula::exprs].
    /// The corresponding expression is the root of this formula's syntax tree and thus the starting point for most algorithms.
    /// We consider all expressions below this expression (including itself) to be sub-expressions.
    /// There might be other (non-sub-)expressions that are currently not relevant to this formula.
    root_id: ExprId,

    own_vars: HashSet<VarId>,
}

impl Formula {
    pub(crate) fn new(root_id: ExprId, own_vars: HashSet<VarId>) -> Self {
        Self { root_id, own_vars }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn assert_valid(&mut self) {
        debug_assert!(self.root_id > 0 && !self.own_vars.is_empty());
    }

    pub(crate) fn in_arena<'a>(&'a self, arena: &'a Arena) -> FormulaContext {
        FormulaContext {
            formula: self,
            arena,
        }
    }

    /// Returns the root expression of this formula.
    pub(crate) fn get_root_expr(&self) -> ExprId {
        self.root_id
    }

    // todo: make this struct immutable (drop a formula to create a new one)
    /// Sets the root expression of this formula.
    ///
    /// For a formula to be valid, the root expression has to be set at least once.
    /// It may also be updated subsequently to focus on other expressions of the formula or build more complex expressions.
    fn set_root_expr(&mut self, root_id: ExprId) {
        self.root_id = root_id;
    }

    /// Resets the root expression, if necessary.
    ///
    /// If the root expression is mutated with [Formula::set_expr], structural sharing might be violated.
    /// Because [Formula::set_expr] can only address this issue for children,
    /// we need not explicitly address the only expression that is not a child itself - the root expression.
    pub(super) fn reset_root_expr(arena: &Arena, root_id: &mut ExprId) {
        *root_id = arena.get_expr(&arena.exprs[*root_id]).unwrap();
    }

    /// Returns the identifiers of all sub-expressions of this formula.
    ///
    /// If in canonical form, each identifier is guaranteed to appear only once.
    pub(crate) fn sub_exprs(&mut self, arena: &mut Arena) -> Vec<ExprId> {
        let mut sub_exprs = Vec::<ExprId>::new();
        arena.preorder_rev(&mut self.root_id, |_, id| sub_exprs.push(id));
        sub_exprs
    }

    pub(crate) fn vars(&self, arena: &Arena) -> Vec<(VarId, Var)> {
        arena.filter_vars(|var_id, _| self.own_vars.contains(&var_id))
    }

    /// Panics if structural sharing is violated in this formula.
    ///
    /// That is, we assert that every sub-expr0ession's identifier is indeed the canonical one.
    /// Does not currently check for commutativity, idempotency, or unary expressions.
    #[cfg(debug_assertions)]
    fn assert_canon(&mut self, arena: &mut Arena) {
        arena.preorder_rev(&mut self.root_id, |arena, id| {
            debug_assert_eq!(arena.get_expr(&arena.exprs[id]).unwrap(), id)
        });
    }

    /// Transforms this formula into canonical form (see [Formula::canon_visitor]).
    ///
    /// The resulting formula is logically equivalent to the original formula.
    /// This function is useful when an algorithm assumes or profits from canonical form, or for simplifying a formula after parsing.
    /// In canonical form, several useful guarantees hold:
    /// First, no sub-expression occurs twice in the syntax tree with different identifiers (structural sharing).
    /// Second, equality of sub-expressions is up to commutativity, idempotency, and unary expressions.
    /// Third, no `And` expression is below an `And` expression (and analogously for `Or`).
    /// Fourth, no `Not` expression is below a `Not` expression.
    /// To ensure these guarantees, this visitor must be called in a postorder traversal, preorder does not work.
    pub(crate) fn to_canon(&mut self, arena: &mut Arena) {
        arena.postorder_rev(&mut self.root_id, Arena::canon_visitor);
    }

    /// Transforms this formula into canonical negation normal form by applying De Morgan's laws (see [Formula::nnf_visitor]).
    ///
    /// The resulting formula is logically equivalent to the original formula.
    /// We do this by traversing the formula top-down, meanwhile, we push negations towards the leaves (i.e., [Var] expressions).
    /// Double negations cannot be encountered, as they have already been removed by [Formula::simp_expr].
    pub(crate) fn to_nnf(&mut self, arena: &mut Arena) {
        arena.prepostorder_rev(&mut self.root_id, Arena::nnf_visitor, Arena::canon_visitor);
    }

    /// Transforms this formula into canonical conjunctive normal form by applying distributivity laws (see [Formula::cnf_dist_visitor]).
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

    /// Transforms this formula into canonical conjunctive normal form by introducing auxiliary variables (see [Formula::cnf_tseitin_visitor]).
    ///
    /// The resulting formula is equivalent to the original formula in terms of its named variables (i.e., satisfiability and model count are preserved).
    /// If this formula is in canonical form (see [Formula::to_canon]), we introduce exactly one auxiliary variable per (complex) sub-expression.
    /// Thus, every sub-expression will be "abbreviated" with an auxiliary variable, including the root expression, which facilitates negation.
    /// Also, no sub-expression will be abbreviated twice, so the number of auxiliary variables is equal to the number of sub-expressions.
    /// If this formula is not in canonical form, more auxiliary variables might be introduced.
    /// Note that we only abbreviate complex sub-expressions (i.e., [And] and [Or] expressions).
    pub(crate) fn to_cnf_tseitin(&mut self, arena: &mut Arena) {
        arena.new_vars = Some(vec![]);
        arena.new_exprs = Some(vec![]);
        arena.postorder_rev(&mut self.root_id, Arena::cnf_tseitin_visitor);
        self.own_vars.extend(arena.new_vars.take().unwrap());
        let root_id = self.get_root_expr();
        arena.new_exprs.as_mut().unwrap().push(root_id);
        let new_expr = And(arena.new_exprs.take().unwrap());
        let root_id = arena.expr(new_expr);
        self.set_root_expr(root_id);
    }
}

pub(crate) struct FormulaContext<'a> {
    pub(crate) formula: &'a Formula,
    pub(crate) arena: &'a Arena,
}

/// Displays an expression in a formula.
impl<'a> fmt::Display for FormulaContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.arena.format_expr(self.formula.root_id, f)
    }
}
