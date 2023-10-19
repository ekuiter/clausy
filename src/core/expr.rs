//! Defines expressions in an arena.

use super::var::VarId;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    slice,
};
use Expr::*;

/// Identifier type for expressions.
///
/// Serves as an index into [Arena::exprs].
/// A note on terminology:
/// An expression can be any propositional term associated with an [Arena].
/// However, it is not necessarily contained in the syntax tree of a formula.
/// A sub-expression, on the other hand, is a propositional term associated with a [Formula] that actually appears in said formula's syntax tree.
/// Thus, all sub-expressions are expressions, but not vice versa.
pub(crate) type ExprId = usize;

/// An expression in an arena.
///
/// Currently, we only allow propositional primitives.
/// An expression is always implicitly tied to an [Arena], to which the expression's [ExprId]s or [VarId] refer.
/// We implement expressions as an enum to avoid additional heap allocations for [Var] and [Not].
/// Note that we derive the default equality check and hashing algorithm here:
/// This is sensible because any containing [Formula], if canonical, guarantees that each of its sub-expressions is assigned exactly one identifier.
/// Thus, a shallow equality check or hash on expressions is equivalent to a deep one if they are sub-expressions of the same [Formula].
/// While we derive [Clone], its use may violate structural sharing, which can be fixed with [Arena::canon_visitor] if needed.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) enum Expr {
    /// A propositional variable.
    Var(VarId),

    /// A negation of an expression.
    Not(ExprId),

    /// A conjunction of an expression.
    And(Vec<ExprId>),

    /// A disjunction of an expression.
    Or(Vec<ExprId>),
}

impl Expr {
    /// Calculates the hash of this expression.
    ///
    /// Used to look up an expression's identifier in [Arena::exprs_inv].
    pub(super) fn calc_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    /// Returns the identifiers of the children of this expression.
    ///
    /// We return nothing for [Var] expressions, which have no expression identifiers as children (only a variable identifier).
    /// As [Var] expressions are leaves of a formula's syntax tree, this function is useful when traversing that tree.
    pub(super) fn children(&self) -> &[ExprId] {
        match self {
            Var(_) => &[],
            Not(child_id) => slice::from_ref(child_id),
            And(child_ids) | Or(child_ids) => child_ids,
        }
    }
}
