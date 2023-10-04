//! Data structures and algorithms for feature-model formulas.

#![allow(unused_imports, rustdoc::private_intra_doc_links)]
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    slice,
};
use Expr::*;

use super::clauses::Clauses;

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
/// This is sensible because the associated [Formula] guarantees that each of its sub-expressions is assigned exactly one identifier.
/// Thus, a shallow equality check or hash on is equivalent to a deep one if they are sub-expressions of the same [Formula].
/// While we derive [Clone], its use may violate structural sharing and must be fixed afterwards (e.g., with [Formula::make_shared_visitor]).
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
/// As an invariant, sub-expressions are uniquely stored, so no sub-expression can appear twice with distinct identifiers (structural sharing).
/// This invariant allows for concise representation and facilitates some algorithms (e.g., [Formula::to_cnf_tseitin]).
/// However, it also comes with the downside that each sub-expression has potentially many parents.
/// Thus, owners of sub-expressions are not easily trackable (see [Formula::exprs] on garbage collection).
/// Consequently, we cannot access any parents when mutating sub-expressions, only children.
/// By the structural-sharing invariant, we effectively treat the syntax tree as a directed acyclic graph.
/// We represent this graph as an adjacency list stored in [Formula::exprs].
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
    /// Any algorithms that mutate this formula should take this into account to preserve structural sharing as an invariant.
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
        }
    }

    /// Panics if this formula is invalid.
    ///
    /// A formula is valid if it has at least one variable (added with [Formula::var]) and a root expression (set with [Formula::set_root_expr]).
    /// In addition, structural sharing must not be violated.
    /// All assertions are optional and therefore not included in `cargo build --release`.
    #[cfg(debug_assertions)]
    pub(crate) fn assert_valid(mut self) -> Self {
        debug_assert!(self.root_id > 0 && self.exprs.len() > 1 && self.vars.len() > 1);
        self.assert_shared();
        self
    }

    /// Computes the hash of an expression.
    ///
    /// Used to look up an expression's identifier in [Formula::exprs_inv].
    fn hash_expr(expr: &Expr) -> u64 {
        let mut hasher = DefaultHasher::new();
        expr.hash(&mut hasher);
        hasher.finish()
    }

    /// Adds a new expression to this formula, returning its new identifier.
    ///
    /// Appends the given expression to [Formula::exprs] and enables its lookup via [Formula::exprs_inv].
    /// Requires that no expression equal to the given expression is already in this formula.
    /// Thus, the created identifier will become the expression's canonical identifier (see [Formula::exprs_inv]).
    fn add_expr(&mut self, expr: Expr) -> Id {
        let id = self.exprs.len();
        let hash = Self::hash_expr(&expr);
        self.exprs.push(expr);
        self.exprs_inv.entry(hash).or_default().push(id);
        id
    }

    /// Looks ups the identifier for an expression of this formula.
    ///
    /// The canonical identifier for a given expression is the first one that is associated with its hash
    /// and whose expression is also equal to the given expression (see [Formula::exprs_inv]).
    fn get_expr(&self, expr: &Expr) -> Option<Id> {
        let ids = self.exprs_inv.get(&Self::hash_expr(expr))?;
        for id in ids {
            if self.exprs[*id] == *expr {
                return Some(*id);
            }
        }
        None
    }

    /// Adds or looks up an expression of this formula, returning its identifier.
    ///
    /// This is the preferred way to obtain an expression's identifier, as it ensures structural sharing.
    /// That is, the expression is only added to this formula if it does not already exist.
    pub(crate) fn expr(&mut self, expr: Expr) -> Id {
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

    /// Adds a new auxiliary variable to this formula, returning its [Var] expression's identifier.
    pub(crate) fn add_var_aux_expr(&mut self) -> Id {
        let var_id = self.add_var_aux();
        self.expr(Var(var_id))
    }

    /// Returns the root expression of this formula.
    ///
    /// That is, we return the only child of the auxiliary root expression (see [Formula::aux_root_id]).
    pub(crate) fn get_root_expr(&self) -> Id {
        self.root_id
    }

    /// Sets the root expression of this formula.
    ///
    /// That is, we update this formula's auxiliary root expression with the given expression (see [Formula::aux_root_id]).
    /// For a formula to be valid, the root expression has to be set at least once.
    /// It may also be updated subsequently to focus on other expressions of the formula.
    pub(crate) fn set_root_expr(&mut self, root_id: Id) {
        self.root_id = root_id;
    }

    /// Returns the identifiers of the children of an expression.
    ///
    /// We return nothing for [Var] expressions, which have no expression identifiers as children (only a variable identifier).
    /// As [Var] expressions are leaves of a formula's syntax tree, this function is useful for traversing that tree.
    fn get_child_exprs<'b>(expr: &'b Expr) -> &'b [Id] {
        match expr {
            Var(_) => &[],
            Not(id) => slice::from_ref(id),
            And(ids) | Or(ids) => ids,
        }
    }

    /// Sets the children of an expression in this formula.
    ///
    /// This function must take several precautions to preserve structural sharing, as it performs in-place mutations.
    /// While this function may temporarily violate structural sharing when called for a given expression,
    /// it also makes up for said violation when called for any parent of said expression.
    /// To do this, the function performs three steps:
    /// First, every new child expression is checked for potential duplicates with existing expressions,
    /// which we resolve using the canonical identifier obtained with [Formula::get_expr].
    /// Second, we replace the old children with the new children.
    /// Third, as we might have changed the hash of the expression, we must update its mapping in [Formula::exprs_inv].
    /// One of two cases applies, which can both be handled in the same way:
    /// Either the new expression has never been added before, so structural sharing was not violated.
    /// Thus, we can just push the expression's identifier as the new canonical identifier for the expression.
    /// In the second case, the expression already exists and already has a canonical identifier.
    /// Still, we can push the identifier anyway, as only the first identifier will be considered.
    /// Because this function cleans up violations of children, it must be called after, not before children have been mutated.
    /// Thus, it does not preserve structural sharing on its own when used in [Formula::preorder_rev].
    /// Finally, we never mutate leaves in the syntax tree (i.e., variables).
    fn make_shared_visitor(&mut self, id: Id) {
        if let Var(_) = self.exprs[id] {
            return;
        }
        // clones children, could probably be improved by inlining get_expr
        let mut ids = Self::get_child_exprs(&self.exprs[id]).to_vec();
        for id in ids.iter_mut() {
            *id = self.get_expr(&self.exprs[*id]).unwrap();
        }
        let old_hash = Self::hash_expr(&self.exprs[id]);
        match &mut self.exprs[id] {
            Var(_) => unreachable!(),
            Not(id) => *id = ids[0],
            And(child_ids) | Or(child_ids) => *child_ids = ids,
        };
        // self.exprs_inv
        //     .entry(Self::hash_expr(&self.exprs[id]))
        //     .or_default()
        //     .push(id);
        let new_hash = Self::hash_expr(&self.exprs[id]);
        if new_hash != old_hash { // todo: extract into method?
            self.exprs_inv
                .entry(old_hash)
                .or_default()
                .retain(|id2| *id2 != id); // probably, here only the first matching element has to be removed https://stackoverflow.com/questions/26243025. it may even be possible to not remove anything.
            if self.exprs_inv.get(&old_hash).unwrap().is_empty() {
                // could also be dropped
                self.exprs_inv.remove(&old_hash);
            }
            self.exprs_inv.entry(new_hash).or_default().push(id); // probably, we could only push here if no equal expr has already been pushed (does this interact weirdly when there are true hash collisions involved?)
        }
    }

    // todo
    fn set_expr(&mut self, id: Id, mut expr: Expr) {
        if let Var(_) = self.exprs[id] {
            return;
        }
        // todo: this avoids cycles, can this be done better?
        if Self::get_child_exprs(&expr)
            .iter()
            .next()
            .map_or(false, |child| *child == id)
        {
            return;
        }
        let old_hash = Self::hash_expr(&self.exprs[id]);
        match expr {
            Var(_) => (),
            Not(ref mut id) => *id = self.get_expr(&self.exprs[*id]).unwrap(),
            And(ref mut ids) | Or(ref mut ids) => {
                for id in ids.iter_mut() {
                    *id = self.get_expr(&self.exprs[*id]).unwrap();
                }
            }
        }
        self.exprs[id] = expr;
        // self.exprs_inv
        //     .entry(Self::hash_expr(&self.exprs[id]))
        //     .or_default()
        //     .push(id);
        let new_hash = Self::hash_expr(&self.exprs[id]);
        if new_hash != old_hash {
            self.exprs_inv
                .entry(old_hash)
                .or_default()
                .retain(|id2| *id2 != id); // probably, here only the first matching element has to be removed https://stackoverflow.com/questions/26243025. it may even be possible to not remove anything.
            if self.exprs_inv.get(&old_hash).unwrap().is_empty() {
                // could also be dropped
                self.exprs_inv.remove(&old_hash);
            }
            self.exprs_inv.entry(new_hash).or_default().push(id); // probably, we could only push here if no equal expr has already been pushed (does this interact weirdly when there are true hash collisions involved?)
        }
    }

    /// Resets the auxiliary root expression, if necessary.
    ///
    /// If the auxiliary root expression is mutated with [Formula::set_child_exprs], structural sharing might be violated.
    /// Because [Formula::set_child_exprs] can only address this issue for children,
    /// we need not explicitly address the only expression that is not a child itself - the auxiliary root expression.
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
        let mut write_helper = |kind: &str, ids: &[Id]| {
            write!(f, "{kind}{printed_id}(")?;
            for (i, id) in ids.iter().enumerate() {
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
            Not(id) => write_helper("Not", slice::from_ref(id)),
            And(ids) => write_helper("And", ids),
            Or(ids) => write_helper("Or", ids),
        }
    }

    // todo (maybe combine with unary simplification?)
    fn splice_or(&self, clause_id: Id, new_clause: &mut Vec<Id>) {
        // todo splice child or's
        if let Or(literal_ids) = &self.exprs[clause_id] {
            for literal_id in literal_ids {
                new_clause.push(*literal_id);
            }
        } else {
            new_clause.push(clause_id);
        }
    }

    // todo
    fn dedup(mut vec: Vec<Id>) -> Vec<Id> {
        // todo (inefficient) deduplication for idempotency
        vec.sort();
        vec.dedup();
        vec
    }

    /// Visits all sub-expressions of this formula using a reverse preorder traversal.
    ///
    /// To preserve structural sharing, we assume that the given visitor is idempotent (in terms of formula mutation) and only
    /// performs mutation with the designated methods, such as [Formula::var], [Formula::expr] and [Formula::set_child_exprs]. (todo: update, does not fit with set_expr and to_nnf anymore)
    /// The visitor is called at most once per unique sub-expression:
    /// It will not be called several times on the same sub-expression - this leverages structural sharing.
    /// However, we can also not guarantee it to be called on all sub-expressions - as it might change the very set of sub-expressions.
    /// For improved performance, the traversal is reversed, so children are traversed right-to-left.
    fn preorder_rev(&mut self, first_id: Id, mut visitor: impl FnMut(&mut Self, Id) -> ()) {
        let mut remaining_ids = vec![first_id];
        let mut visited_ids = HashSet::<Id>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.pop().unwrap();
            if !visited_ids.contains(&id) {
                visitor(self, id);
                remaining_ids.extend(Self::get_child_exprs(&self.exprs[id]));
                visited_ids.insert(id);
            }
        }
        self.reset_root_expr();
    }

    /// Visits all sub-expressions of this formula using a reverse postorder traversal.
    ///
    /// Conceptually, this is similar to [Formula::preorder_rev], but sub-expressions are visited bottom-up instead of top-down.
    /// todo: can guarantee structural sharing opposed to preorder_rev (but is the responsibility of the visitor, still)
    fn postorder_rev(&mut self, first_id: Id, mut visitor: impl FnMut(&mut Self, Id) -> ()) {
        let mut remaining_ids = vec![first_id];
        let mut seen_ids = HashSet::<Id>::new();
        let mut visited_ids = HashSet::<Id>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.last().unwrap();
            let child_ids = Self::get_child_exprs(&self.exprs[*id]);
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

    // todo
    // pre visitor does not visit leaves, currently (interior nodes are visited twice, leaves once)
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

            if !Self::get_child_exprs(&self.exprs[*id]).is_empty()
                && !seen_ids.contains(id)
                && !visited_ids.contains(id)
            {
                seen_ids.insert(*id);
                pre_visitor(self, *id);
                remaining_ids.extend(Self::get_child_exprs(&self.exprs[*id]));
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

    /// Panics if structural sharing is violated in this formula.
    ///
    /// That is, we assert that every sub-expression's identifier is indeed the canonical one.
    #[cfg(debug_assertions)]
    fn assert_shared(&mut self) {
        self.preorder_rev(self.root_id, |formula, id| {
            debug_assert_eq!(formula.get_expr(&formula.exprs[id]).unwrap(), id)
        });
    }

    /// Manually enforces structural sharing in this formula.
    ///
    /// As [Formula::preorder_rev] mutates expressions before their children, it may violate structural sharing.
    /// The easiest way to fix this is by calling this function, which establishes the invariant again with a postorder traversal.
    // fn make_shared(&mut self) {
    //     self.postorder_rev(self.root_id, Self::make_shared_visitor);
    // }

    /// Returns the identifiers of all sub-expressions of this formula.
    ///
    /// Due to structural sharing, each identifier is guaranteed to appear only once.
    pub(crate) fn sub_exprs(&mut self) -> Vec<Id> {
        let mut sub_exprs = Vec::<Id>::new();
        self.preorder_rev(self.root_id, |_, id| {
            sub_exprs.push(id);
        });
        sub_exprs
    }

    /// Returns all named sub-variables of this formula.
    ///
    /// Analogously to a sub-expression, a sub-variable is a variable in [Formula::vars] that appear below the auxiliary root expression.
    fn named_vars(&mut self, first_id: Id) -> HashSet<Var<'a>> {
        let mut named_vars = HashSet::<Var<'a>>::new();
        self.preorder_rev(first_id, |formula, id| {
            if let Var(var_id) = formula.exprs[id] {
                let var_id: usize = var_id.try_into().unwrap();
                if let Var::Named(_) = formula.vars[var_id] {
                    named_vars.insert(formula.vars[var_id]);
                }
            }
        });
        named_vars // todo: when using this set, take care of order to guarantee determinism
    }

    fn nnf_visitor(&mut self, id: Id) {
        // todo splicing/unary?
        match &self.exprs[id] {
            Var(_) | And(_) | Or(_) => (),
            Not(child_id) => {
                match &self.exprs[*child_id] {
                    Var(_) => (),
                    Not(grandchild_id) => {
                        // todo what if this is an and and we are, too? could splice (maybe also remove unary)
                        self.set_expr(id, self.exprs[*grandchild_id].clone());
                    }
                    And(grandchild_ids) => {
                        let new_expr = Or(self.negate_exprs(grandchild_ids.clone()));
                        self.set_expr(id, new_expr);
                    }
                    Or(grandchild_ids) => {
                        // todo: what if we created an and, but are ourselves an and? could splice here!
                        let new_expr = And(self.negate_exprs(grandchild_ids.clone()));
                        self.set_expr(id, new_expr);
                    }
                }
            }
        }
    }

    /// Transforms this formula into negation normal form by applying De Morgan's laws and removing double negations.
    ///
    /// We do this by traversing the formula top-down.
    /// Meanwhile, we push negations towards the leaves (i.e., [Var] expressions) and we remove double negations.
    /// After the traversal, we re-establish structural sharing (see [Formula::make_shared]).
    pub(crate) fn to_nnf(mut self) -> Self {
        self.prepostorder_rev(self.root_id, Self::nnf_visitor, Self::make_shared_visitor);
        self
    }

    /// Transforms this formula into conjunctive normal form by applying distributivity laws.
    ///
    /// We do this by traversing the formula bottom-up and pushing [Or] expressions below [And] expressions via multiplication.
    /// This algorithm has exponential worst-case complexity, but ensures logical equivalence to the original formula.
    /// Note that the formula must already be in negation normal form (see [Formula::to_nnf]).
    /// `TODO`
    pub(crate) fn to_cnf_dist(mut self) -> Self {
        // todo: refactor code
        // todo also, is this idempotent?
        // todo currently, this seems correct, but much less efficient than FeatureIDE, possibly optimize
        self.postorder_rev(self.root_id, |formula, id| {
            // todo need the children two times on the stack here, could maybe be disabled, but then merging is more complicated
            // let child_ids = Self::get_child_exprs(&formula.exprs[id]).to_vec();
            // let mut new_child_ids = Vec::<Id>::new();

            // todo extract this as a helper function for hybrid tseitin
            match &formula.exprs[id] {
                Var(_) | Not(_) => (),
                And(child_ids) => {
                    // if formula.is_non_aux_and(id) || grandchild_ids.len() == 1 {
                    //     new_child_ids.extend(grandchild_ids.clone());
                    //     // todo new_child_ids.push(self.expr(And(cnf))); // todo unoptimized version
                    // }
                    // todo: how to do splicing if we don't have the parent?
                    let mut new_child_ids = vec![];
                    for child_id in child_ids {
                        match &formula.exprs[*child_id] {
                            And(grandchild_ids) => {
                                new_child_ids.extend(grandchild_ids);
                            }
                            _ => new_child_ids.push(*child_id),
                        }
                    }
                    formula.set_expr(id, And(new_child_ids));
                }
                Or(child_ids) => {
                    let mut clauses = Vec::<Vec<Id>>::new();
                    for (i, child_id) in child_ids.iter().enumerate() {
                        let clause_ids = match &formula.exprs[*child_id] {
                            // todo could multiply all len's to calculate a threshold for hybrid tseitin
                            Var(_) | Not(_) | Or(_) => slice::from_ref(child_id),
                            And(ids) => ids,
                        };

                        if i == 0 {
                            clauses.extend(
                                // todo possibly this can be done with a neutral element instead
                                clause_ids
                                    .iter()
                                    .map(|clause_id| {
                                        let mut new_clause = Vec::<Id>::new();
                                        formula.splice_or(*clause_id, &mut new_clause);
                                        new_clause
                                    })
                                    .collect::<Vec<Vec<Id>>>(),
                            );
                        } else {
                            let mut new_clauses = Vec::<Vec<Id>>::new();
                            for clause in &clauses {
                                for clause_id in clause_ids {
                                    let mut new_clause = clause.clone();
                                    formula.splice_or(*clause_id, &mut new_clause);
                                    new_clauses.push(new_clause);
                                }
                            }
                            clauses = new_clauses;
                        }
                    }
                    let mut new_cnf_ids = Vec::<Id>::new();
                    for mut clause in clauses {
                        clause = Self::dedup(clause); // todo idempotency
                                                      // todo unary or
                        if clause.len() > 1 {
                            let value = formula.expr(Or(clause));
                            new_cnf_ids.push(value);
                        } else {
                            new_cnf_ids.push(clause[0]);
                        }
                    }
                    // if formula.is_non_aux_and(id) || new_cnf_ids.len() == 1 {
                    //     // todo splice into parent and
                    //     new_child_ids.extend(new_cnf_ids);
                    //     // todo new_child_ids.push(self.expr(And(new_cnf_ids))); // todo unoptimized version
                    // } else {
                    //     new_child_ids.push(formula.expr(And(new_cnf_ids)));
                    // }
                    formula.set_expr(id, And(new_cnf_ids));
                }
            }

            //formula.set_child_exprs(id, Self::dedup(new_child_ids));
        });
        self
    }

    /// Defines an [And] expression with a new auxiliary variable.
    ///
    /// That is, we create a new auxiliary variable and clauses that let it imply all conjuncts and let it be implied by the conjunction.
    /// As an optimization, we do not create a [Var] expression for the new variable, as we are replacing an existing expression.
    fn def_and(&mut self, var_expr_id: Id, ids: &[Id]) -> (VarId, Vec<Id>) {
        let var_id = self.add_var_aux();
        let not_var_expr_id = self.expr(Not(var_expr_id));
        let mut clauses = Vec::<Id>::new();
        for id in ids {
            clauses.push(self.expr(Or(vec![not_var_expr_id, *id])));
        }
        let mut clause = vec![var_expr_id];
        // todo might create double negation here, avoid this (presumably already in expr(Not(...))? although this would affect parsing, maybe extra method)
        clause.extend(self.negate_exprs(ids.to_vec()));
        clauses.push(self.expr(Or(clause)));
        // todo add these to the formula, ideally also splicing correctly
        (var_id, clauses)
    }

    /// Defines an [Or] expression with a new auxiliary variable.
    ///
    /// That is, we create a new auxiliary variable and clauses that let it imply the disjunction and let it be implied by all disjuncts.
    /// Works analogously to [Formula::def_and].
    fn def_or(&mut self, var_expr_id: Id, ids: &[Id]) -> (VarId, Vec<Id>) {
        let var_id = self.add_var_aux();
        let not_var_expr_id = self.expr(Not(var_expr_id));
        let mut clause = vec![not_var_expr_id];
        clause.extend(ids);
        let mut clauses = vec![self.expr(Or(clause))];
        for id in ids {
            let new_expr = Or(vec![var_expr_id, self.expr(Not(*id))]);
            clauses.push(self.expr(new_expr));
        }
        (var_id, clauses)
    }

    // todo currently assumes NNF for simplicity, but not a good idea generally - also, does not guarantee NNF itself, to_nnf must be called again (which is quite expensive, actually!!)
    // todo this ensures "minimal aux vars" - minimal in the sense that no two aux vars have the same children
    pub(crate) fn to_cnf_tseitin(mut self) -> Self {
        // todo is this idempotent?
        let mut new_clauses = Vec::<Id>::new();

        self.prepostorder_rev(self.root_id, Self::nnf_visitor, |formula, id| {
            match &formula.exprs[id] {
                Var(_) | Not(_) => (),
                And(child_ids) => {
                    // todo what about unary And?
                    // todo ...
                    // todo this assumes deduplicated child_ids, as otherwise, duplicate children will get _two_ aux vars
                    let (var_id, clauses) = formula.def_and(id, &child_ids.clone());
                    new_clauses.extend(clauses);
                    formula.set_expr(id, Var(var_id));
                }
                Or(grandchild_ids) => {
                    let (var_id, clauses) = formula.def_or(id, &grandchild_ids.clone());
                    new_clauses.extend(clauses);
                    formula.set_expr(id, Var(var_id));
                }
            }
        });

        new_clauses.push(self.get_root_expr());
        let root_id = self.expr(And(new_clauses));
        self.set_root_expr(root_id);
        self
    }

    pub(crate) fn to_clauses(&self) -> Clauses<'a> {
        Clauses::from(self)
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
