//! Data structures and algorithms for feature-model formulas.

#![allow(unused_imports, rustdoc::private_intra_doc_links)]

use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    slice,
};
use Expr::*;

/// Whether to print identifiers of expressions.
///
/// Useful for debugging, but should generally be disabled, as this is expected by [crate::tests].
const PRINT_ID: bool = false;

/// Prefix for auxiliary variables.
///
/// Auxiliary variables are required by some algorithms on formulas and can be created with [Var::Aux].
const VAR_AUX_PREFIX: &str = "_aux_";

/// Identifier type for expressions.
///
/// Serves as an index into [Formula::exprs].
/// A note on terminology:
/// An expression can be any propositional term associated with a [Formula].
/// However, it is not necessarily contained in the syntax tree of said formula (e.g., when it was transformed into another expression).
/// A sub-expression, on the other hand, is a propositional term associated with a [Formula] that actually appears in its syntax tree.
/// Thus, all sub-expressions are expressions, but not vice versa.
pub(crate) type Id = usize;

/// Identifier type for variables.
///
/// Serves as an index into [Formula::vars].
/// We also use this type to represent literals in [crate::core::clauses::Clauses], therefore we use a signed type.
/// Also, we do not expect too many variables, so a 32-bit integer should suffice.
pub(crate) type VarId = i32;

/// An expression in a formula.
///
/// Currently, we only allow propositional primitives.
/// An expression is always implicitly tied to a [Formula], to which the expression's [Id]s or [VarId] refer.
/// We implement expressions as an enum to avoid additional heap allocations for [Var] and [Not].
/// Note that we derive the default equality check and hashing algorithm here:
/// This is sensible because the associated [Formula], if canonical, guarantees that each of its sub-expressions is assigned exactly one identifier.
/// Thus, a shallow equality check or hash on is equivalent to a deep one if they are sub-expressions of the same [Formula].
/// While we derive [Clone], its use may violate structural sharing, which can be fixed with [Formula::canon_visitor] if needed.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) enum Expr {
    /// A propositional variable.
    Var(VarId),

    /// A negation of an expression.
    Not(Id),

    /// A conjunction of an expression.
    And(Vec<Id>),

    /// A disjunction of an expression.
    Or(Vec<Id>),
}

/// Operations on expressions that are independent of its containing formula.
impl Expr {
    /// Calculates the hash of this expression.
    ///
    /// Used to look up an expression's identifier in [Formula::exprs_inv].
    fn calc_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    /// Returns the identifiers of the children of this expression.
    ///
    /// We return nothing for [Var] expressions, which have no expression identifiers as children (only a variable identifier).
    /// As [Var] expressions are leaves of a formula's syntax tree, this function is useful when traversing that tree.
    fn children(&self) -> &[Id] {
        match self {
            Var(_) => &[],
            Not(child_id) => slice::from_ref(child_id),
            And(child_ids) | Or(child_ids) => child_ids,
        }
    }
}

/// Simplifies an expression in this formula to an equivalent one.
///
/// For example, this transforms a given expression `Or(b, And(a, Not(a), a), b)` into `b`.
/// Implemented as a macro for repeated use in [Formula::simp_expr].
macro_rules! simp_expr {
    ($formula:expr, $expr:expr, $child_ids:expr, $constructor:ident) => {{
        $child_ids.sort_unstable_by_key(|child_id| match $formula.exprs[*child_id] {
            Not(grandchild_id) => grandchild_id * 2 + 1,
            _ => *child_id * 2,
        });
        $child_ids.dedup();
        if $child_ids.len() == 1 {
            *$expr = $formula.exprs[$child_ids[0]].clone();
        } else if $child_ids
            .windows(2)
            .flat_map(<&[Id; 2]>::try_from)
            .find(
                |&&[child_a_id, child_b_id]| match $formula.exprs[child_a_id] {
                    Not(_) => false,
                    _ => match $formula.exprs[child_b_id] {
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
/// Implemented as a macro for repeated use in [Formula::flatten_expr].
macro_rules! flatten_expr {
    ($formula:expr, $expr:expr, $child_ids:expr, $constructor:ident) => {
        *$child_ids = $child_ids
            .iter()
            .map(|child_id| match &$formula.exprs[*child_id] {
                $constructor(grandchild_ids) => grandchild_ids,
                _ => slice::from_ref(child_id),
            })
            .flatten()
            .map(|id| *id)
            .collect()
    };
}

/// A variable in a formula.
///
/// Variables can either be named or auxiliary.
/// Named variables refer to a string, which represents their name.
/// To avoid unnecessary copies, we use a reference that must outlive the [Formula] the variable is tied to (e.g., created by [mod@crate::parser]).
/// Some algorithms on formulas (e.g., [Formula::to_cnf_tseitin]) require creating new, auxiliary variables.
/// As these variables are anonymous and have no designated meaning in the feature-modeling domain, we assign them arbitrary numbers.
/// To avoid creating unnecessary strings, we store these as native numbers.
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub(crate) enum Var<'a> {
    /// A named variable.
    Named(&'a str),

    /// An auxiliary variable.
    Aux(u32),
}

/// A feature-model formula.
///
/// We represent a formula by storing its syntax tree; that is, each unique sub-expression that appears in it.
/// In canonical form, sub-expressions are uniquely stored, so no sub-expression appears twice with distinct identifiers (structural sharing).
/// This allows for concise representation and facilitates some algorithms (e.g., [Formula::to_cnf_tseitin]).
/// However, it also comes with the downside that each sub-expression has potentially many parents.
/// Thus, owners of sub-expressions are not easily trackable (see [Formula::exprs] on garbage collection).
/// Consequently, we cannot access any parents when mutating sub-expressions, only children.
/// Due to the structural-sharing, we effectively treat the syntax tree as a directed acyclic graph.
/// We represent this graph as an adjacency list stored in [Formula::exprs].
/// Note that due to performance reasons, structural sharing is not fully guaranteed by all algorithms (including parsers) until calling [Formula::canon_visitor].
#[derive(Debug)]
pub(crate) struct Formula<'a> {
    /// Stores all expressions in this formula.
    ///
    /// Serves as a fast lookup for an expression, given its identifier.
    /// Expressions are stored in the order of their creation, so new expressions are appended with [Vec::push].
    /// Also, while algorithms may update expressions in-place, no expression is ever removed.
    /// We refer to all expressions that appear below the root expression as sub-expressions (including the root expression).
    /// By not ever removing any expressions, we keep all non-sub-expressions indefinitely.
    /// This potentially requires a lot of memory, but avoids explicit reference counting or garbage collection.
    pub(crate) exprs: Vec<Expr>,

    /// Maps expressions to their identifiers.
    ///
    /// Serves as a fast inverse lookup for the unique identifier of a given sub-expression.
    /// To simplify ownership, we implement this lookup by mapping from the hash of a sub-expression to several identifiers.
    /// By structural sharing, the identifier for a sub-expression should be unique, but we still need a [Vec] for two reasons:
    /// First, there might be hash collisions, which we address by checking true equality when reading [Formula::exprs_inv].
    /// Second, while sub-expressions have a unique identifier, there might be distinct, orphaned expressions that are equal to a given sub-expression.
    /// For example, such a situation arises when [Formula::set_expr] modifies a sub-expression
    /// and the resulting expression is equal (but not identical) to an existing sub-expression.
    /// As an expression cannot easily update its own identifier in the whole syntax tree,
    /// [Formula::set_expr] considers the first identifier in [Formula::exprs_inv] to be the canonical one (modulo hash collisions).
    /// Whenever [Formula::set_expr] encounters the concerned expression later, it then adapts its identifier to the canonical one.
    /// By this design, [Formula::exprs_inv] indeed maps any sub-expression (precisely: its hash) to its unique identifier
    /// (precisely: the first identifier whose expression is equal to the given sub-expression).
    /// This information can be used to enforce structural sharing by calling [Formula::canon_visitor].
    exprs_inv: HashMap<u64, Vec<Id>>,

    /// Stores all variables in this formula.
    ///
    /// Conceptually, this is analogous to [Formula::exprs].
    /// However, there is no distinction analogous to sub-expressions and expressions, as variables need not be removed.
    /// Consequently, feature-model slicing (variable forgetting) is currently not supported.
    /// Another difference to [Formula::exprs] is that named variables are not owned by this formula.
    /// Thus, we can borrow references to variable names from the parsed string and avoid cloning them.
    pub(crate) vars: Vec<Var<'a>>,

    /// Maps variables to their identifiers.
    ///
    /// Conceptually, this is analogous to [Formula::exprs_inv].
    /// However, the inverse lookup of variables is less complex:
    /// First, this formula does not own the variable names, which avoids the hash collisions discussed for [Formula::exprs_inv].
    /// Second, variables and their identifiers are never mutated after creation, so no additional [Vec] is needed.
    vars_inv: HashMap<Var<'a>, VarId>,

    /// Specifies the root expression of this formula.
    ///
    /// Serves as an index into [Formula::exprs].
    /// The corresponding expression is the root of this formula's syntax tree and thus the starting point for most algorithms.
    /// We consider all expressions below this expression (including itself) to be sub-expressions.
    /// There might be other (non-sub-)expressions that are currently not relevant to this formula.
    root_id: Id,

    /// Specifies the identifier of the most recently added auxiliary variable.
    ///
    /// Ensures that new auxiliary variables (created with [Var::Aux]) are uniquely identified in the context of this formula.
    var_aux_id: u32,

    /// Stores new expressions created by an algorithm but not yet included in the syntax tree.
    ///
    /// Used by [Formula::to_cnf_tseitin] for holding on to definitional expressions.
    new_exprs: Option<Vec<Id>>,
}

/// An expression that is explicitly paired with the formula it is tied to.
///
/// This struct is useful whenever we need to pass an expression around, but the containing formula is not available.
/// Using this might be necessary when there is no `self` of type [Formula], for example whenever we want to [fmt::Display] an expression.
pub(crate) struct ExprInFormula<'a>(pub(crate) &'a Formula<'a>, pub(crate) &'a Id);

/// Algorithms for constructing, mutating, and analyzing formulas.
impl<'a> Formula<'a> {
    /// Creates a new, empty formula.
    ///
    /// The created formula is initially invalid (see [Formula::assert_valid]).
    /// The auxiliary variable with number 0 has no meaningful sign and can therefore not be used.
    /// This simplifies the representation of literals in [crate::core::clauses::Clauses], which can be negative.
    pub(crate) fn new() -> Self {
        Self {
            exprs: vec![Var(0)],
            exprs_inv: HashMap::new(),
            vars: vec![Var::Aux(0)],
            vars_inv: HashMap::new(),
            root_id: 0,
            var_aux_id: 0,
            new_exprs: None,
        }
    }

    /// Panics if this formula is invalid.
    ///
    /// A formula is valid if it has at least one variable (added with [Formula::var_expr]) and a root expression (set with [Formula::set_root_expr]).
    /// In addition, structural sharing must not be violated (see [Formula::canon_visitor]).
    /// All assertions are optional and therefore not included in `cargo build --release`.
    #[cfg(debug_assertions)]
    pub(crate) fn assert_valid(mut self) -> Self {
        debug_assert!(self.root_id > 0 && self.exprs.len() > 1 && self.vars.len() > 1);
        self.assert_canon();
        self
    }

    /// Adds a new expression to this formula, returning its new identifier.
    ///
    /// Appends the given expression to [Formula::exprs] and enables its lookup via [Formula::exprs_inv].
    /// Requires that no expression equal to the given expression is already in this formula.
    /// Thus, the created identifier will become the expression's canonical identifier (see [Formula::exprs_inv]).
    fn add_expr(&mut self, expr: Expr) -> Id {
        let id = self.exprs.len();
        let hash = expr.calc_hash();
        self.exprs.push(expr);
        self.exprs_inv.entry(hash).or_default().push(id);
        id
    }

    /// Looks ups the identifier for an expression of this formula.
    ///
    /// The canonical identifier for a given expression is the first one that is associated with its hash
    /// and whose expression is also equal to the given expression (see [Formula::exprs_inv]).
    fn get_expr(&self, expr: &Expr) -> Option<Id> {
        self.exprs_inv
            .get(&expr.calc_hash())?
            .iter()
            .filter(|id| self.exprs[**id] == *expr)
            .map(|id| *id)
            .next()
    }

    /// Adds or looks up an expression of this formula, returning its identifier.
    ///
    /// This is the preferred way to obtain an expression's identifier, as it ensures structural sharing.
    /// That is, the expression is only added to this formula if it does not already exist.
    /// Before we add the expression, we simplify it, which is a cheap operation (in contrast to [Formula::flatten_expr]).
    pub(crate) fn expr(&mut self, mut expr: Expr) -> Id {
        self.simp_expr(&mut expr);
        self.get_expr(&expr).unwrap_or_else(|| self.add_expr(expr))
    }

    /// Adds a new variable to this formula, returning its identifier.
    ///
    /// Works analogously to [Formula::add_expr] (see [Formula::vars_inv]).
    fn add_var(&mut self, var: Var<'a>) -> VarId {
        let id = self.vars.len();
        let id_signed: i32 = id.try_into().unwrap();
        self.vars.push(var);
        self.vars_inv.insert(var.clone(), id_signed);
        id_signed
    }

    /// Adds a new named variable to this formula, returning its identifier.
    fn add_var_named(&mut self, name: &'a str) -> VarId {
        self.add_var(Var::Named(name))
    }

    /// Adds a new auxiliary variable to this formula, returning its identifier.
    pub(crate) fn add_var_aux(&mut self) -> VarId {
        self.var_aux_id += 1;
        self.add_var(Var::Aux(self.var_aux_id))
    }

    /// Looks ups the identifier of a named variable in this formula.
    ///
    /// Works analogously to [Formula::get_expr] (see [Formula::vars_inv]).
    fn get_var_named(&mut self, name: &str) -> Option<VarId> {
        Some(*self.vars_inv.get(&Var::Named(name))?)
    }

    /// Adds or looks up a named variable of this formula, returning its [Var] expression's identifier.
    ///
    /// This is the preferred way to obtain a [Var] expression's identifier (see [Formula::expr]).
    pub(crate) fn var_expr(&mut self, var: &'a str) -> Id {
        let var_id = self
            .get_var_named(var)
            .unwrap_or_else(|| self.add_var_named(var));
        self.expr(Var(var_id))
    }

    /// Adds or looks up a named variable of this formula, returning its [Var] expression's and [Var]'s identifier.
    pub(crate) fn var_expr_with_id(&mut self, var: &'a str) -> (Id, VarId) {
        let expr_id = self.var_expr(var);
        if let Var(var_id) = self.exprs[expr_id] {
            (expr_id, var_id)
        } else {
            unreachable!()
        }
    }

    /// Adds a new auxiliary variable to this formula, returning its [Var] expression's identifier.
    pub(crate) fn add_var_aux_expr(&mut self) -> Id {
        let var_id = self.add_var_aux();
        self.expr(Var(var_id))
    }

    /// Returns the root expression of this formula.
    pub(crate) fn get_root_expr(&self) -> Id {
        self.root_id
    }

    /// Sets the root expression of this formula.
    ///
    /// For a formula to be valid, the root expression has to be set at least once.
    /// It may also be updated subsequently to focus on other expressions of the formula or build more complex expressions.
    pub(crate) fn set_root_expr(&mut self, root_id: Id) {
        self.root_id = root_id;
    }

    /// Simplifies an expression in this formula to an equivalent one.
    ///
    /// First, we sort the expression's children, thus equality is up to commutativity.
    /// Second, we remove duplicate children of the expressions, thus equality is up to idempotency.
    /// Third, we identify unary expressions with their operands (i.e., `And(x)` is simplified to `x`).
    /// Fourth, we remove double negations (i.e., `Not(Not(x))` is simplified to `x`).
    /// Fifth, we remove obvious tautologies and contradictions (i.e., `And(a, Not(a))` is simplified to `Or()`).
    /// Because we clone expressions, this function may violate structural sharing (see [Formula::canon_visitor]).
    /// As this is a cheap and useful operation to make the formula smaller, we already call it in the parsing stage.
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
    /// Analogously to [Formula::simp_expr], this performs a simplification of an expression.
    /// However, this may create new expressions and is therefore more expensive and not called in the parsing stage.
    /// This is useful to call during a postorder syntax tree traversal to ensure canonical form (see [Formula::canon_visitor]).
    fn flatten_expr(&mut self, expr: &mut Expr) {
        match expr {
            Var(_) | Not(_) => (),
            And(child_ids) => flatten_expr!(self, expr, child_ids, And),
            Or(child_ids) => flatten_expr!(self, expr, child_ids, Or),
        }
    }

    /// Invalidates an expression after it was mutated.
    ///
    /// Does so by updating its mapping in [Formula::exprs_inv].
    /// One of two cases applies, which can both be handled in the same way:
    /// Either the new expression has never been added before, so structural sharing was not violated.
    /// Thus, we can just append the expression's identifier as the new canonical identifier for the expression.
    /// In the second case, the expression already exists and already has a canonical identifier.
    /// Still, we can append the identifier anyway, as only the first identifier will be considered.
    /// In terms of correctness, appending the identifier suffices, although we may optimize by cleaning up [Formula::exprs_inv].
    fn inval_expr(&mut self, id: Id) {
        self.exprs_inv
            .entry(self.exprs[id].calc_hash())
            .or_default()
            .push(id);
    }

    /// Mutates an expression in this formula.
    ///
    /// This function replaces the expression for a given identifier with a new given expression.
    /// It has no effect on leaves in the syntax tree (i.e., variables).
    /// We must take several precautions to preserve structural sharing, as we perform an in-place mutation.
    /// While this function may temporarily violate structural sharing when called for a given expression,
    /// it also makes up for (i.e., "fixes") said violation when called for every parent of said expression afterwards (see [Formula::canon_visitor]).
    /// To do so, the function performs three steps:
    /// First, every new child expression is checked for potential duplicates with existing expressions,
    /// which we resolve using the canonical identifier obtained with [Formula::get_expr].
    /// Second, we replace the old expression with the new expression.
    /// Third, as we might have changed the hash of the expression, we must invalidate it with [Formula::inval_expr].
    /// Because this function cleans up violations of children, it must be called after, not before children have been mutated.
    /// Thus, it does not preserve structural sharing when used in [Formula::preorder_rev], only in [Formula::postorder_rev].
    /// Besides guaranteeing structural sharing, we perform flattening and simplification on the expression, which usually produces smaller formulas.
    fn set_expr(&mut self, id: Id, mut expr: Expr) {
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

    /// Resets the root expression, if necessary.
    ///
    /// If the root expression is mutated with [Formula::set_expr], structural sharing might be violated.
    /// Because [Formula::set_expr] can only address this issue for children,
    /// we need not explicitly address the only expression that is not a child itself - the root expression.
    fn reset_root_expr(&mut self) {
        self.root_id = self.get_expr(&self.exprs[self.root_id]).unwrap();
    }

    /// Returns expressions that negate the given expressions.
    ///
    /// The returned expression identifiers are either created or looked up (see [Formula::expr]).
    fn negate_exprs(&mut self, mut ids: Vec<Id>) -> Vec<Id> {
        for id in &mut ids {
            *id = self.expr(Not(*id));
        }
        ids
    }

    /// Returns all identifiers of named variables in this formula that are not in a given a set of variable identifiers.
    pub(crate) fn named_vars_except(&self, var_ids: &HashSet<VarId>) -> Vec<VarId> {
        self.vars
            .iter()
            .enumerate()
            .flat_map(|(idx, var)| match *var {
                Var::Named(_) => Some(idx),
                Var::Aux(_) => None,
            })
            .map(|idx| idx.try_into().unwrap())
            .filter(|var_id| !var_ids.contains(var_id))
            .collect()
    }

    /// Writes an expression of this formula to a formatter.
    ///
    /// Used by [fmt::Display] to print (parts of) a formula.
    /// Implements a recursive preorder traversal.
    /// For an iterative reversed preorder traversal, see [Formula::preorder_rev].
    fn format_expr(&self, id: Id, f: &mut fmt::Formatter) -> fmt::Result {
        let printed_id = if PRINT_ID {
            format!("@{id}")
        } else {
            String::from("")
        };
        let mut write_helper = |kind: &str, child_ids: &[Id]| {
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

    /// Visits all sub-expressions of this formula using a reverse preorder traversal.
    ///
    /// We assume that the given visitor only performs mutation with the designated methods,
    /// such as [Formula::var_expr], [Formula::expr] and [Formula::set_expr].
    /// The visitor is called at most once per unique sub-expression:
    /// It will not be called several times on the same sub-expression (if this formula is in canonical form).
    /// However, we can also not guarantee it to be called on all sub-expressions - as it might change the very set of sub-expressions.
    /// For improved performance, the traversal is reversed, so children are traversed right-to-left.
    fn preorder_rev(&mut self, first_id: Id, mut visitor: impl FnMut(&mut Self, Id) -> ()) {
        let mut remaining_ids = vec![first_id];
        let mut visited_ids = HashSet::<Id>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.pop().unwrap();
            if !visited_ids.contains(&id) {
                visitor(self, id);
                remaining_ids.extend(self.exprs[id].children());
                visited_ids.insert(id);
            }
        }
        self.reset_root_expr();
    }

    /// Visits all sub-expressions of this formula using a reverse postorder traversal.
    ///
    /// Conceptually, this is similar to [Formula::preorder_rev], but sub-expressions are visited bottom-up instead of top-down.
    /// Also, this traversal can be used to ensure structural sharing if the visitor is correctly implemented (see [Formula::canon_visitor]).
    fn postorder_rev(&mut self, first_id: Id, mut visitor: impl FnMut(&mut Self, Id) -> ()) {
        let mut remaining_ids = vec![first_id];
        let mut seen_ids = HashSet::<Id>::new();
        let mut visited_ids = HashSet::<Id>::new();
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
        self.reset_root_expr();
    }

    /// Visits all sub-expressions of this formula using a combined reverse pre- and postorder traversal.
    ///
    /// Can be used to efficiently interleave a preorder and postorder visitor.
    /// Note that each interior expression is visited twice (with the pre- and then the postorder visitor).
    /// However, the leaves (i.e., [Var] expressions) are only visited once (with the postorder visitor).
    fn prepostorder_rev(
        &mut self,
        first_id: Id,
        mut pre_visitor: impl FnMut(&mut Self, Id) -> (),
        mut post_visitor: impl FnMut(&mut Self, Id) -> (),
    ) {
        let mut remaining_ids = vec![first_id];
        let mut seen_ids: HashSet<usize> = HashSet::<Id>::new();
        let mut visited_ids = HashSet::<Id>::new();
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
        self.reset_root_expr();
    }

    /// Returns the identifiers of all sub-expressions of this formula.
    ///
    /// If in canonical form, each identifier is guaranteed to appear only once.
    pub(crate) fn sub_exprs(&mut self) -> Vec<Id> {
        let mut sub_exprs = Vec::<Id>::new();
        self.preorder_rev(self.root_id, |_, id| {
            sub_exprs.push(id);
        });
        sub_exprs
    }

    /// Panics if structural sharing is violated in this formula.
    ///
    /// That is, we assert that every sub-expression's identifier is indeed the canonical one.
    /// Does not currently check for commutativity, idempotency, or unary expressions.
    #[cfg(debug_assertions)]
    fn assert_canon(&mut self) {
        self.preorder_rev(self.root_id, |formula, id| {
            debug_assert_eq!(formula.get_expr(&formula.exprs[id]).unwrap(), id)
        });
    }

    /// Transforms an expression into canonical form (see [Formula::to_canon]).
    fn canon_visitor(&mut self, id: Id) {
        self.set_expr(id, self.exprs[id].clone());
    }

    /// Transforms an expression into negation normal form by applying De Morgan's laws (see [Formula::to_nnf]).
    fn nnf_visitor(&mut self, id: Id) {
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
    fn cnf_dist_visitor(&mut self, id: Id) {
        match &self.exprs[id] {
            Var(_) | Not(_) => (),
            And(_) => self.set_expr(id, self.exprs[id].clone()),
            Or(child_ids) => {
                let mut new_clauses: Vec<Vec<Id>> = vec![vec![]];
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
    /// We add the clauses defining the new variable to [Formula::new_exprs].
    fn def_and(&mut self, var_expr_id: Id, ids: &[Id]) -> VarId {
        let var_id = self.add_var_aux();
        let not_var_expr_id = self.expr(Not(var_expr_id));
        let mut clauses = Vec::<Id>::new();
        clauses.extend(
            ids.iter()
                .map(|id| self.expr(Or(vec![not_var_expr_id, *id]))),
        );
        let mut clause = vec![var_expr_id];
        clause.extend(self.negate_exprs(ids.to_vec()));
        clauses.push(self.expr(Or(clause)));
        self.new_exprs.as_mut().unwrap().extend(clauses);
        var_id
    }

    /// Defines an [Or] expression with a new auxiliary variable.
    ///
    /// That is, we create a new auxiliary variable and clauses that let it imply the disjunction and let it be implied by all disjuncts.
    /// Works analogously to [Formula::def_and].
    fn def_or(&mut self, var_expr_id: Id, ids: &[Id]) -> VarId {
        let var_id = self.add_var_aux();
        let not_var_expr_id = self.expr(Not(var_expr_id));
        let mut clause = vec![not_var_expr_id];
        clause.extend(ids);
        let mut clauses = vec![self.expr(Or(clause))];
        clauses.extend(ids.iter().map(|id| {
            let new_expr = Or(vec![var_expr_id, self.expr(Not(*id))]);
            self.expr(new_expr)
        }));
        self.new_exprs.as_mut().unwrap().extend(clauses);
        var_id
    }

    /// Transforms an expression into canonical conjunctive normal form by introducing auxiliary variables (see [Formula::to_cnf_tseitin]).
    fn cnf_tseitin_visitor(&mut self, id: Id) {
        match &self.exprs[id] {
            Var(_) | Not(_) => (),
            And(child_ids) => {
                let var_id = self.def_and(id, &child_ids.clone());
                self.set_expr(id, Var(var_id));
            }
            Or(grandchild_ids) => {
                let var_id = self.def_or(id, &grandchild_ids.clone());
                self.set_expr(id, Var(var_id));
            }
        }
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
    pub(crate) fn to_canon(mut self) -> Self {
        self.postorder_rev(self.root_id, Self::canon_visitor);
        self
    }

    /// Transforms this formula into canonical negation normal form by applying De Morgan's laws (see [Formula::nnf_visitor]).
    ///
    /// The resulting formula is logically equivalent to the original formula.
    /// We do this by traversing the formula top-down, meanwhile, we push negations towards the leaves (i.e., [Var] expressions).
    /// Double negations cannot be encountered, as they have already been removed by [Formula::simp_expr].
    pub(crate) fn to_nnf(mut self) -> Self {
        self.prepostorder_rev(self.root_id, Self::nnf_visitor, Self::canon_visitor);
        self
    }

    /// Transforms this formula into canonical conjunctive normal form by applying distributivity laws (see [Formula::cnf_dist_visitor]).
    ///
    /// The resulting formula is logically equivalent to the original formula.
    /// We do this by traversing the formula bottom-up and pushing [Or] expressions below [And] expressions via multiplication.
    /// This algorithm has exponential worst-case complexity, but ensures logical equivalence to the original formula.
    pub(crate) fn to_cnf_dist(mut self) -> Self {
        self.prepostorder_rev(self.root_id, Self::nnf_visitor, Self::cnf_dist_visitor);
        self
    }

    /// Transforms this formula into canonical conjunctive normal form by introducing auxiliary variables (see [Formula::cnf_tseitin_visitor]).
    ///
    /// The resulting formula is equivalent to the original formula in terms of its named variables (i.e., satisfiability and model count are preserved).
    /// If this formula is in canonical form (see [Formula::to_canon]), we introduce exactly one auxiliary variable per (complex) sub-expression.
    /// Thus, every sub-expression will be "abbreviated" with an auxiliary variable, including the root expression, which facilitates negation.
    /// Also, no sub-expression will be abbreviated twice, so the number of auxiliary variables is equal to the number of sub-expressions.
    /// If this formula is not in canonical form, more auxiliary variables might be introduced.
    /// Note that we only abbreviate complex sub-expressions (i.e., [And] and [Or] expressions).
    pub(crate) fn to_cnf_tseitin(mut self) -> Self {
        self.new_exprs = Some(vec![]);
        self.postorder_rev(self.root_id, Self::cnf_tseitin_visitor);
        let root_id = self.get_root_expr();
        self.new_exprs.as_mut().unwrap().push(root_id);
        let new_expr = And(self.new_exprs.unwrap());
        self.new_exprs = None;
        let root_id = self.expr(new_expr);
        self.set_root_expr(root_id);
        self
    }
}

/// Displays a formula.
impl<'a> fmt::Display for Var<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Var::Named(name) => write!(f, "{name}"),
            Var::Aux(id) => write!(f, "{}{id}", VAR_AUX_PREFIX),
        }
    }
}

/// Displays an expression in a formula.
impl<'a> fmt::Display for ExprInFormula<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.format_expr(*self.1, f)
    }
}

/// Displays a formula.
impl<'a> fmt::Display for Formula<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ExprInFormula(self, &self.get_root_expr()).fmt(f)
    }
}
