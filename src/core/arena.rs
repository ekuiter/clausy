//! Defines an arena of variables and expressions.

use super::{
    expr::{Expr, Expr::*, ExprId},
    formula::Formula,
    var::{Var, VarId},
};
use crate::shell::options;
use std::{
    collections::{HashMap, HashSet},
    fmt, slice,
};

/// Simplifies an expression in an arena to an equivalent one.
///
/// For example, this transforms a given expression `Or(b, And(a, Not(a), a), b)` into `b`.
/// Implemented as a macro for repeated use in [Arena::simp_expr].
macro_rules! simp_expr {
    ($arena: expr, $expr: expr, $child_ids: expr, $constructor: ident) => {{
        $child_ids.sort_unstable_by_key(|child_id| match $arena.exprs[*child_id] {
            Not(grandchild_id) => grandchild_id * 2 + 1,
            _ => *child_id * 2,
        });
        $child_ids.dedup();
        if $child_ids.len() == 1 {
            *$expr = $arena.exprs[$child_ids[0]].clone();
        } else if $child_ids
            .windows(2)
            .flat_map(<&[ExprId; 2]>::try_from)
            .find(
                |&&[child_a_id, child_b_id]| match $arena.exprs[child_a_id] {
                    Not(_) => false,
                    _ => match $arena.exprs[child_b_id] {
                        Not(grandchild_b_id) => child_a_id == grandchild_b_id,
                        _ => false,
                    },
                },
            )
            .is_some()
        {
            *$expr = $constructor(vec![]);
        }
    }};
}

/// Flattens children of an expression into their parent.
///
/// That is, this transforms a given expression `And(And(a), Or(b, c))` into `And(a, Or(b, c))`.
/// Implemented as a macro for repeated use in [Arena::flatten_expr].
macro_rules! flatten_expr {
    ($arena: expr, $expr: expr, $child_ids: expr, $constructor: ident) => {
        *$child_ids = $child_ids
            .iter()
            .map(|child_id| match &$arena.exprs[*child_id] {
                $constructor(grandchild_ids) => grandchild_ids,
                _ => slice::from_ref(child_id),
            })
            .flatten()
            .map(|id| *id)
            .collect()
    };
}

/// An arena of variables and expressions.
///
/// We aim to represent any [Formula] by storing its syntax tree; that is, each unique sub-expression that appears in it.
/// To allow reuse of expressions and variables across similar formulas, we store them in a shared
/// [arena](https://en.wikipedia.org/wiki/Region-based_memory_management) referenced by each [Formula].
/// In a canonical [Formula], sub-expressions are uniquely stored, so no sub-expression appears twice with distinct identifiers (structural sharing).
/// This allows for concise representation and facilitates some algorithms (e.g., [Formula::to_cnf_tseitin]).
/// However, it also comes with the downside that each sub-expression has potentially many parents.
/// Thus, owners of sub-expressions are not easily trackable (see [Arena::exprs] on garbage collection).
/// Consequently, we cannot access any parents when mutating sub-expressions, only children.
/// Due to structural sharing, we effectively treat the syntax tree as a directed acyclic graph.
/// We represent this graph as an adjacency list stored in [Arena::exprs].
/// Note that, for performance reasons, structural sharing is not guaranteed by all algorithms (including parsers) until calling [Arena::canon_visitor].
#[derive(Clone)]
pub(crate) struct Arena {
    /// Stores all variables in this arena.
    ///
    /// Conceptually, this is analogous to [Arena::exprs].
    /// Accordingly, the set of variables referenced by a given [Formula] constitutes its sub-variables (see [Formula::sub_var_ids]).
    /// Variable-forgetting operations, such as feature-model slicing, are possible by adapting this set.
    pub(crate) vars: Vec<Var>,

    /// Maps variables to their identifiers.
    ///
    /// Conceptually, this is analogous to [Arena::exprs_inv].
    /// However, we simplify the inverse lookup of variables by cloning variables.
    /// This needs more memory, but (a) we do not expect many variables and (b) this avoids the hash collisions discussed for [Arena::exprs_inv].
    /// Also, variables and their identifiers are never mutated after creation, so we need no additional [Vec] for updates.
    vars_inv: HashMap<Var, VarId>,

    /// Stores all expressions in this arena.
    ///
    /// Serves as a fast lookup for an expression, given its identifier.
    /// Expressions are stored in the order of their creation, so new expressions are appended with [Vec::push].
    /// Also, while algorithms may update expressions in-place, no expression is ever removed.
    /// We refer to all expressions that appear below the root expression of a [Formula] as its sub-expressions (including the root expression).
    /// By not ever removing any expressions, we keep all non-sub-expressions indefinitely.
    /// This potentially requires a lot of memory, but avoids explicit reference counting or garbage collection.
    pub(super) exprs: Vec<Expr>,

    /// Maps expressions to their identifiers.
    ///
    /// Serves as a fast inverse lookup for the unique identifier of a given sub-expression.
    /// To simplify ownership, we implement this lookup by mapping from the hash of a sub-expression to several identifiers.
    /// By structural sharing, the identifier for a sub-expression should be unique, but we still need a [Vec] for two reasons:
    /// First, there might be hash collisions, which we address by checking true equality when reading [Arena::exprs_inv].
    /// Second, while sub-expressions have a unique identifier, there might be distinct, orphaned expressions that are equal to a given sub-expression.
    /// For example, such a situation arises when [Arena::set_expr] modifies a sub-expression
    /// and the resulting expression is equal (but not identical) to an existing sub-expression.
    /// As an expression cannot easily update its own identifier in the whole syntax tree,
    /// [Arena::set_expr] considers the first identifier in [Arena::exprs_inv] to be the canonical one (modulo hash collisions).
    /// Whenever [Arena::set_expr] encounters the concerned expression later, it then adapts its identifier to the canonical one.
    /// By this design, [Arena::exprs_inv] indeed maps any sub-expression (precisely: its hash) to its unique identifier
    /// (precisely: the first identifier whose expression is equal to the given sub-expression).
    /// This information can be used to enforce structural sharing by calling [Arena::canon_visitor].
    exprs_inv: HashMap<u64, Vec<ExprId>>,

    /// Specifies the identifier of the most recently added auxiliary variable.
    ///
    /// Ensures that new auxiliary variables (created with [Var::Aux]) are uniquely identified in the context of this arena.
    var_aux_id: u32,

    /// Temporarily stores new variables created that are not yet included in any [Formula]'s syntax tree.
    ///
    /// Used by [Arena::cnf_tseitin_visitor] for holding on to auxiliary variables.
    pub(super) new_vars: Option<Vec<VarId>>,

    /// Temporarily stores new expressions that are not yet included in any [Formula]'s syntax tree.
    ///
    /// Used by [Arena::cnf_tseitin_visitor] for holding on to definitional expressions.
    pub(super) new_exprs: Option<Vec<ExprId>>,
}

impl Arena {
    /// Creates a new, empty arena.
    ///
    /// This simplifies the representation of literals in [crate::core::clauses::Clauses], which can be negative.
    pub(crate) fn new() -> Self {
        Self {
            vars: vec![],
            vars_inv: HashMap::new(),
            exprs: vec![],
            exprs_inv: HashMap::new(),
            var_aux_id: 0,
            new_exprs: None,
            new_vars: None,
        }
    }

    /// Adds a new variable to this arena, returning its identifier.
    ///
    /// Works analogously to [Arena::add_expr] (see [Arena::vars_inv]).
    fn add_var(&mut self, var: Var) -> VarId {
        let id = self.vars.len();
        let id_signed: i32 = id.try_into().unwrap();
        self.vars.push(var.clone());
        self.vars_inv.insert(var, id_signed);
        id_signed
    }

    /// Adds a new named variable to this arena, returning its identifier.
    fn add_var_named(&mut self, name: String) -> VarId {
        self.add_var(Var::Named(name))
    }

    /// Adds a new auxiliary variable to this arena, returning its identifier.
    pub(super) fn add_var_aux(&mut self) -> VarId {
        self.var_aux_id += 1;
        self.add_var(Var::Aux(self.var_aux_id))
    }

    /// Looks ups the identifier of a named variable in this arena.
    ///
    /// Works analogously to [Arena::get_expr] (see [Arena::vars_inv]).
    fn get_var_named(&mut self, name: String) -> Option<VarId> {
        Some(*self.vars_inv.get(&Var::Named(name))?)
    }

    /// Adds or looks up a named variable in this arena, returning its [Var] expression's identifier.
    ///
    /// This is the preferred way to obtain a [Var] expression's identifier (see [Arena::expr]).
    pub(crate) fn var_expr(&mut self, var: String) -> ExprId {
        let var_id = self
            .get_var_named(var.clone())
            .unwrap_or_else(|| self.add_var_named(var));
        self.expr(Var(var_id))
    }

    /// Adds or looks up a named variable in this arena, returning its [Var] expression's and [Var]'s identifier.
    pub(crate) fn var_expr_with_id(&mut self, var: String) -> (ExprId, VarId) {
        let expr_id = self.var_expr(var);
        if let Var(var_id) = self.exprs[expr_id] {
            (expr_id, var_id)
        } else {
            unreachable!()
        }
    }

    /// Adds a new auxiliary variable to this arena, returning its identifier and its [Var] expression's identifier.
    pub(crate) fn add_var_aux_expr(&mut self) -> (VarId, ExprId) {
        let var_id = self.add_var_aux();
        (var_id, self.expr(Var(var_id)))
    }

    /// Returns all variable identifiers in this arena.
    pub(crate) fn var_ids(&self) -> HashSet<VarId> {
        (0..self.vars.len().try_into().unwrap()).collect()
    }

    /// Returns all variables and their identifiers in this arena that match a given predicate.
    pub(crate) fn vars(&self, predicate: impl Fn(VarId, &Var) -> bool) -> Vec<(VarId, Var)> {
        self.vars
            .iter()
            .enumerate()
            .flat_map(|(var_id, var)| {
                let var_id: VarId = var_id.try_into().unwrap();
                if predicate(var_id, var) {
                    Some((var_id, var.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Given a set of variable identifiers, returns a set of their associated sting representations.
    #[allow(unused)]
    pub(crate) fn var_strs(&self, var_ids: &HashSet<VarId>) -> Vec<String> {
        var_ids
            .into_iter()
            .map(|id| self.vars[TryInto::<usize>::try_into(*id).unwrap()].to_string())
            .collect::<Vec<String>>()
    }

    /// Adds a new expression to this arena, returning its new identifier.
    ///
    /// Appends the given expression to [Arena::exprs] and enables its lookup via [Arena::exprs_inv].
    /// Requires that no expression equal to the given expression is already in this arena.
    /// Thus, the created identifier will become the expression's canonical identifier (see [Arena::exprs_inv]).
    fn add_expr(&mut self, expr: Expr) -> ExprId {
        let id = self.exprs.len();
        let hash = expr.calc_hash();
        self.exprs.push(expr);
        self.exprs_inv.entry(hash).or_default().push(id);
        id
    }

    /// Looks ups the identifier for an expression in this arena.
    ///
    /// The canonical identifier for a given expression is the first one that is associated with its hash
    /// and whose expression is also equal to the given expression (see [Arena::exprs_inv]).
    pub(super) fn get_expr(&self, expr: &Expr) -> Option<ExprId> {
        self.exprs_inv
            .get(&expr.calc_hash())?
            .iter()
            .filter(|id| self.exprs[**id] == *expr)
            .map(|id| *id)
            .next()
    }

    /// Adds or looks up an expression in this arena, returning its identifier.
    ///
    /// This is the preferred way to obtain an expression's identifier, as it ensures structural sharing.
    /// That is, the expression is only added to this arena if it does not already exist.
    /// Before we add the expression, we simplify it, which is a cheap operation (in contrast to [Arena::flatten_expr]).
    pub(crate) fn expr(&mut self, mut expr: Expr) -> ExprId {
        self.simp_expr(&mut expr);
        self.get_expr(&expr).unwrap_or_else(|| self.add_expr(expr))
    }

    /// Simplifies an expression in this arena to an equivalent one.
    ///
    /// First, we sort the expression's children, thus equality is up to commutativity.
    /// Second, we remove duplicate children of the expressions, thus equality is up to idempotency.
    /// Third, we identify unary expressions with their operands (i.e., `And(x)` is simplified to `x`).
    /// Fourth, we remove double negations (i.e., `Not(Not(x))` is simplified to `x`).
    /// Fifth, we remove obvious tautologies and contradictions (i.e., `And(a, Not(a))` is simplified to `Or()`).
    /// Because we clone expressions, this function may violate structural sharing (see [Arena::canon_visitor]).
    /// As this is a cheap and useful operation to make the arena smaller, we already call it in the parsing stage.
    fn simp_expr(&mut self, expr: &mut Expr) {
        match expr {
            Var(_) => (),
            Not(child_id) => match &self.exprs[*child_id] {
                Var(_) | And(_) | Or(_) => (),
                Not(grandchild_id) => {
                    *expr = self.exprs[*grandchild_id].clone();
                }
            },
            And(child_ids) => simp_expr!(self, expr, child_ids, Or),
            Or(child_ids) => simp_expr!(self, expr, child_ids, And),
        }
    }

    /// Flattens children of an expression into their parent.
    ///
    /// Analogously to [Arena::simp_expr], this performs a simplification of an expression.
    /// However, this may create new expressions and is therefore more expensive, thus not called in the parsing stage.
    /// This is useful to call during a postorder syntax tree traversal to ensure canonical form (see [Arena::canon_visitor]).
    pub(super) fn flatten_expr(&mut self, expr: &mut Expr) {
        match expr {
            Var(_) | Not(_) => (),
            And(child_ids) => flatten_expr!(self, expr, child_ids, And),
            Or(child_ids) => flatten_expr!(self, expr, child_ids, Or),
        }
    }

    /// Invalidates an expression after it was mutated.
    ///
    /// Does so by updating its mapping in [Arena::exprs_inv].
    /// One of two cases applies, which can both be handled in the same way:
    /// Either the new expression has never been added before, so structural sharing was not violated.
    /// Thus, we can just append the expression's identifier as the new canonical identifier for the expression.
    /// In the second case, the expression already exists and already has a canonical identifier.
    /// Still, we can append the identifier anyway, as only the first identifier will be considered.
    /// In terms of correctness, appending the identifier suffices, although we may optimize in the future by cleaning up [Arena::exprs_inv].
    fn inval_expr(&mut self, id: ExprId) {
        self.exprs_inv
            .entry(self.exprs[id].calc_hash())
            .or_default()
            .push(id);
    }

    /// Mutates an expression in this arena.
    ///
    /// This function replaces the expression for a given identifier with a new given expression.
    /// It has no effect on leaves in the syntax tree (i.e., variables).
    /// We must take several precautions to preserve structural sharing, as we perform an in-place mutation.
    /// While this function may temporarily violate structural sharing when called for a given expression,
    /// it also makes up for (i.e., "fixes") said violation when called for every parent of said expression afterwards (see [Arena::canon_visitor]).
    /// To do so, the function performs three steps:
    /// First, every new child expression is checked for potential duplicates with existing expressions,
    /// which we resolve using the canonical identifier obtained with [Arena::get_expr].
    /// Second, we replace the old expression with the new expression.
    /// Third, as we might have changed the hash of the expression, we must invalidate it with [Arena::inval_expr].
    /// Because this function cleans up violations of children, it must be called after, not before children have been mutated.
    /// Thus, it does not preserve structural sharing when used in [Arena::preorder_rev], only in [Arena::postorder_rev].
    /// Besides guaranteeing structural sharing, we perform flattening and simplification on the expression, which usually produces smaller formulas.
    fn set_expr(&mut self, id: ExprId, mut expr: Expr) {
        if let Var(_) = self.exprs[id] {
            return;
        }
        match expr {
            Var(_) => (),
            Not(ref mut id) => *id = self.get_expr(&self.exprs[*id]).unwrap(),
            And(ref mut ids) | Or(ref mut ids) => {
                for id in ids.iter_mut() {
                    *id = self.get_expr(&self.exprs[*id]).unwrap();
                }
            }
        }
        self.flatten_expr(&mut expr);
        self.simp_expr(&mut expr);
        self.exprs[id] = expr;
        self.inval_expr(id);
    }

    /// Returns expressions that negate the given expressions.
    ///
    /// The returned expression identifiers are either created or looked up (see [Arena::expr]).
    fn negate_exprs(&mut self, mut ids: Vec<ExprId>) -> Vec<ExprId> {
        for id in &mut ids {
            *id = self.expr(Not(*id));
        }
        ids
    }

    /// Writes an expression in this arena to a formatter.
    ///
    /// Used by [fmt::Display] to print (parts of) a [Formula].
    /// Implements a recursive preorder traversal.
    /// For an iterative reversed preorder traversal, see [Arena::preorder_rev].
    pub(super) fn format_expr(&self, id: ExprId, f: &mut fmt::Formatter) -> fmt::Result {
        let printed_id = if options().output.print_ids {
            format!("@{id}")
        } else {
            String::from("")
        };
        let mut write_helper = |kind: &str, child_ids: &[ExprId]| {
            write!(f, "{kind}{printed_id}(")?;
            for (i, id) in child_ids.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                self.format_expr(*id, f)?;
            }
            write!(f, ")")
        };
        match &self.exprs[id] {
            Var(var_id) => {
                let var_id: usize = (*var_id).try_into().unwrap();
                write!(f, "{}{printed_id}", self.vars[var_id])
            }
            Not(child_id) => write_helper("Not", slice::from_ref(child_id)),
            And(child_ids) => write_helper("And", child_ids),
            Or(child_ids) => write_helper("Or", child_ids),
        }
    }

    /// Returns a formula with the given root expression.
    ///
    /// The created formula references all variables of this arena, use [Formula::new] for more fine-grained sub-variables.
    pub(crate) fn as_formula<'a>(&'a self, root_id: ExprId) -> Formula {
        Formula::new(
            self.var_ids(),
            root_id,
            None,
        )
    }

    /// Visits all sub-expressions of a given root expression using a reverse preorder traversal.
    ///
    /// We assume that the given visitor only performs mutation with the designated methods,
    /// such as [Arena::var_expr], [Arena::expr] and [Arena::set_expr].
    /// The visitor is called at most once per unique sub-expression:
    /// It will not be called several times on the same sub-expression (if the formula is in canonical form).
    /// However, we can also not guarantee it to be called on all sub-expressions - as it might change the very set of sub-expressions.
    /// For improved performance, the traversal is reversed, so children are traversed right-to-left.
    pub(super) fn preorder_rev(
        &mut self,
        root_id: &mut ExprId,
        mut visitor: impl FnMut(&mut Self, ExprId) -> (),
    ) {
        let mut remaining_ids = vec![*root_id];
        let mut visited_ids = HashSet::<ExprId>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.pop().unwrap();
            if !visited_ids.contains(&id) {
                visitor(self, id);
                remaining_ids.extend(self.exprs[id].children());
                visited_ids.insert(id);
            }
        }
        Formula::reset_root_expr(root_id, &self);
    }

    /// Visits all sub-expressions of a given root expression using a reverse postorder traversal.
    ///
    /// Conceptually, this is similar to [Arena::preorder_rev], but sub-expressions are visited bottom-up instead of top-down.
    /// Also, this traversal can be used to ensure structural sharing if the visitor is correctly implemented (see [Arena::canon_visitor]).
    pub(super) fn postorder_rev(
        &mut self,
        root_id: &mut ExprId,
        mut visitor: impl FnMut(&mut Self, ExprId) -> (),
    ) {
        let mut remaining_ids = vec![*root_id];
        let mut seen_ids = HashSet::<ExprId>::new();
        let mut visited_ids = HashSet::<ExprId>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.last().unwrap();
            let child_ids = self.exprs[*id].children();
            if !child_ids.is_empty() && !seen_ids.contains(id) && !visited_ids.contains(id) {
                seen_ids.insert(*id);
                remaining_ids.extend(child_ids);
            } else {
                if !visited_ids.contains(&id) {
                    visitor(self, *id);
                    visited_ids.insert(*id);
                    seen_ids.remove(id);
                }
                remaining_ids.pop();
            }
        }
        Formula::reset_root_expr(root_id, &self);
    }

    /// Visits all sub-expressions of a given root expression using a combined reverse pre- and postorder traversal.
    ///
    /// Can be used to efficiently interleave a preorder and postorder visitor.
    /// Note that each interior expression is visited twice (with the pre- and then the postorder visitor).
    /// However, the leaves (i.e., [Var] expressions) are only visited once (with the postorder visitor).
    pub(super) fn prepostorder_rev(
        &mut self,
        root_id: &mut ExprId,
        mut pre_visitor: impl FnMut(&mut Self, ExprId) -> (),
        mut post_visitor: impl FnMut(&mut Self, ExprId) -> (),
    ) {
        let mut remaining_ids = vec![*root_id];
        let mut seen_ids: HashSet<usize> = HashSet::<ExprId>::new();
        let mut visited_ids = HashSet::<ExprId>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.last().unwrap();
            if !self.exprs[*id].children().is_empty()
                && !seen_ids.contains(id)
                && !visited_ids.contains(id)
            {
                seen_ids.insert(*id);
                pre_visitor(self, *id);
                remaining_ids.extend(self.exprs[*id].children());
            } else {
                if !visited_ids.contains(id) {
                    post_visitor(self, *id);
                    visited_ids.insert(*id);
                    seen_ids.remove(id);
                }
                remaining_ids.pop();
            }
        }
        Formula::reset_root_expr(root_id, &self);
    }

    /// Transforms an expression into canonical form (see [Formula::to_canon]).
    pub(super) fn canon_visitor(&mut self, id: ExprId) {
        self.set_expr(id, self.exprs[id].clone());
    }

    /// Transforms an expression into negation normal form by applying De Morgan's laws (see [Formula::to_nnf]).
    pub(super) fn nnf_visitor(&mut self, id: ExprId) {
        match &self.exprs[id] {
            Var(_) | And(_) | Or(_) => (),
            Not(child_id) => match &self.exprs[*child_id] {
                Var(_) => (),
                Not(_) => unreachable!(),
                And(grandchild_ids) => {
                    let new_expr = Or(self.negate_exprs(grandchild_ids.clone()));
                    self.set_expr(id, new_expr);
                }
                Or(grandchild_ids) => {
                    let new_expr = And(self.negate_exprs(grandchild_ids.clone()));
                    self.set_expr(id, new_expr);
                }
            },
        }
    }

    /// Transforms an expression into canonical conjunctive normal form by applying distributivity laws (see [Formula::to_cnf_dist]).
    pub(super) fn cnf_dist_visitor(&mut self, id: ExprId) {
        match &self.exprs[id] {
            Var(_) | Not(_) => (),
            And(_) => self.set_expr(id, self.exprs[id].clone()),
            Or(child_ids) => {
                let mut new_clauses: Vec<Vec<ExprId>> = vec![vec![]];
                for child_id in child_ids {
                    let clause_ids = match &self.exprs[*child_id] {
                        Var(_) | Not(_) | Or(_) => slice::from_ref(child_id),
                        And(child_ids) => child_ids,
                    };
                    new_clauses = new_clauses
                        .iter()
                        .map(|new_clause| {
                            clause_ids.iter().map(|clause_id| {
                                new_clause
                                    .iter()
                                    .chain(match &self.exprs[*clause_id] {
                                        Or(literal_ids) => literal_ids,
                                        _ => slice::from_ref(clause_id),
                                    })
                                    .cloned()
                                    .collect()
                            })
                        })
                        .flatten()
                        .collect();
                }
                let new_clause_ids = new_clauses
                    .into_iter()
                    .map(|new_clause| self.expr(Or(new_clause)))
                    .collect();
                self.set_expr(id, And(new_clause_ids));
            }
        }
    }

    /// Defines an [And] expression with a new auxiliary variable.
    ///
    /// That is, we create a new auxiliary variable and clauses that let it imply all conjuncts and let it be implied by the conjunction.
    /// As an optimization, we do not create a [Var] expression for the new variable, as we are replacing an existing expression.
    /// We add the new variable to [Arena::new_vars] and the clauses defining the new variable to [Arena::new_exprs].
    fn def_and(&mut self, var_expr_id: ExprId, ids: &[ExprId]) -> VarId {
        let var_id = self.add_var_aux();
        let not_var_expr_id = self.expr(Not(var_expr_id));
        let mut clauses = Vec::<ExprId>::new();
        clauses.extend(
            ids.iter()
                .map(|id| self.expr(Or(vec![not_var_expr_id, *id]))),
        );
        let mut clause = vec![var_expr_id];
        clause.extend(self.negate_exprs(ids.to_vec()));
        clauses.push(self.expr(Or(clause)));
        self.new_vars.as_mut().unwrap().push(var_id);
        self.new_exprs.as_mut().unwrap().extend(clauses);
        var_id
    }

    /// Defines an [Or] expression with a new auxiliary variable.
    ///
    /// That is, we create a new auxiliary variable and clauses that let it imply the disjunction and let it be implied by all disjuncts.
    /// Works analogously to [Arena::def_and].
    fn def_or(&mut self, var_expr_id: ExprId, ids: &[ExprId]) -> VarId {
        let var_id = self.add_var_aux();
        let not_var_expr_id = self.expr(Not(var_expr_id));
        let mut clause = vec![not_var_expr_id];
        clause.extend(ids);
        let mut clauses = vec![self.expr(Or(clause))];
        clauses.extend(ids.iter().map(|id| {
            let new_expr = Or(vec![var_expr_id, self.expr(Not(*id))]);
            self.expr(new_expr)
        }));
        self.new_vars.as_mut().unwrap().push(var_id);
        self.new_exprs.as_mut().unwrap().extend(clauses);
        var_id
    }

    /// Transforms an expression into canonical conjunctive normal form by introducing auxiliary variables (see [Formula::to_cnf_tseitin]).
    pub(super) fn cnf_tseitin_visitor(&mut self, id: ExprId) {
        match &self.exprs[id] {
            Var(_) | Not(_) => (),
            And(child_ids) => {
                if !child_ids.is_empty() {
                    let var_id = self.def_and(id, &child_ids.clone());
                    self.set_expr(id, Var(var_id));
                }
            }
            Or(child_ids) => {
                if !child_ids.is_empty() {
                    let var_id = self.def_or(id, &child_ids.clone());
                    self.set_expr(id, Var(var_id));
                }
            }
        }
    }

    /// Returns whether the given expression contains any given expression.
    pub(super) fn contains_expr(&self, root_id: ExprId, ids: &HashSet<ExprId>) -> bool {
        let mut remaining_ids = vec![root_id];
        let mut visited_ids = HashSet::<ExprId>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.pop().unwrap();
            if !visited_ids.contains(&id) {
                if ids.contains(&id) {
                    return true;
                }
                remaining_ids.extend(self.exprs[id].children());
                visited_ids.insert(id);
            }
        }
        false
    }

    /// Returns whether the given expression contains any given variable.
    pub(super) fn contains_var(&mut self, root_id: ExprId, ids: &HashSet<VarId>) -> bool {
        let ids = ids.iter().map(|id| self.expr(Var(*id))).collect();
        self.contains_expr(root_id, &ids)
    }
}
