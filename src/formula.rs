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
const PRINT_IDENTIFIER: bool = true;

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
/// We also use this type to represent literals in [crate::cnf::CNF], therefore we use a signed type.
/// Also, we do not expect too many variables, so a 32-bit integer should usually suffice.
pub(crate) type VarId = i32;

/// An expression in a formula.
///
/// Currently, we only allow propositional primitives.
/// An expression is always implicitly tied to a [Formula], to which the expression's [Id]s or [VarId] refer.
/// We implement expressions as an enum to avoid additional heap allocations for [Var] and [Not].
/// Note that we derive the default equality check and hashing algorithm here:
/// This is sensible because the associated [Formula] guarantees that each of its sub-expressions is assigned exactly one identifier.
/// Thus, a shallow equality check or hash on is equivalent to a deep one if they are sub-expressions of the same [Formula].
#[derive(Debug, PartialEq, Eq, Hash)]
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

/// A feature-model formula.
///
/// We represent a formula by storing its syntax tree; that is, each unique sub-expression that appears in it.
/// As an invariant, sub-expressions are uniquely stored, so no sub-expression can appear twice with distinct identifiers (structural sharing).
/// This invariant allows for concise representation and facilitates some algorithms (e.g., Tseitin transformation).
/// However, it also comes with the downside that each sub-expression has potentially many parents.
/// Thus, owners of sub-expressions are not easily trackable (see [Formula::exprs] on garbage collection).
/// Consequently, all algorithms must be implemented in a way that only mutates the children of an expression, not their parent(s).
/// By structural sharing, we effectively treat the syntax tree as a directed acyclic graph.
/// We represent this graph as an adjacency list stored in [Formula::exprs].
#[derive(Debug)]
pub struct Formula<'a> {
    /// Stores all expressions in this formula.
    ///
    /// Serves as a fast lookup for an expression, given its identifier.
    /// Expressions are stored in the order of their creation, so new expressions are appended with [Vec::push].
    /// Also, while some algorithms may update expressions in-place, no expression is ever removed.
    /// We refer to all expressions that appear below the auxiliary root expression as sub-expressions.
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
    /// For example, such a situation arises when [Formula::set_child_exprs] modifies its expression's children
    /// and the resulting expression is equal (but not identical) to an existing sub-expression.
    /// As an expression cannot easily change its own identifier (similar to how a variable cannot change its own type),
    /// [Formula::set_child_exprs] considers the first identifier in [Formula::exprs_inv] to be the canonical one (modulo hash collisions).
    /// Whenever [Formula::set_child_exprs] encounters the concerned expression, it then adapts its identifier to the canonical one.
    /// By this design, [Formula::exprs_inv] indeed maps any sub-expression (precisely: its hash) to its unique identifier
    /// (precisely: the first identifier whose expression is equal to the given sub-expression).
    /// Any algorithms that mutate this formula should take this into account to preserve structural sharing as an invariant.
    exprs_inv: HashMap<u64, Vec<Id>>,

    /// Stores all variables in this formula.
    ///
    /// Conceptually, this is analogous to [Formula::exprs].
    /// However, there is no distinction analogous to sub-expressions and expressions, as variables need not be removed.
    /// Consequently, feature-model slicing (variable forgetting) is currently not supported.
    /// Another difference to [Formula::exprs] is that variables (which are simply names) are not owned by this formula.
    /// Thus, we can borrow references to variable names from the parsed string and avoid cloning them.
    pub(crate) vars: Vec<&'a str>,

    /// Maps variables to their identifiers.
    ///
    /// Conceptually, this is analogous to [Formula::exprs_inv].
    /// However, the inverse lookup of variables is more simple:
    /// First, this formula does not own the variables, which avoids the hash collisions discussed for [Formula::exprs_inv].
    /// Second, variables and their identifiers are never mutated after creation, so no additional [Vec] is needed.
    vars_inv: HashMap<&'a str, VarId>,

    /// Specifies the auxiliary root expression of this formula.
    ///
    /// Serves as an index into [Formula::exprs].
    /// The corresponding expression is the auxiliary root of this formula's syntax tree and thus the starting point for most algorithms.
    /// We consider all expressions below this expression (including itself) to be sub-expressions.
    /// There might be other (non-sub-)expressions that are currently not relevant to this formula.
    /// Note that [Formula::get_root_expr] and [Formula::set_root_expr] do not store the user-supplied root expression here,
    /// but an auxiliary [And] expression that has the root expression as its single child.
    /// This allows algorithms to freely mutate the root expression if necessary (see [Formula] on mutating children).
    aux_root_id: Id,
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
    /// The variable with empty name and identifier 0 has no meaningful sign and can therefore not be used.
    /// This simplifies the representation of literals in [crate::cnf::CNF].
    pub(crate) fn new() -> Self {
        Self {
            exprs: vec![Var(0)],
            exprs_inv: HashMap::new(),
            vars: vec![""],
            vars_inv: HashMap::new(),
            aux_root_id: 0,
        }
    }

    /// Panics if this formula is invalid.
    ///
    /// A formula is valid if it has at least one variable (added with [Formula::var]) and a root expression (set with [Formula::set_root_expr]).
    fn assert_valid(&self) {
        assert!(
            self.aux_root_id > 0 && self.exprs.len() > 1 && self.vars.len() > 1,
            "formula is invalid"
        );
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
        self.exprs.insert(id, expr);
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

    /// Adds a new variable to this formula, returning the identifier of its [Var] expression.
    ///
    /// Works analogously to [Formula::add_expr] (see [Formula::vars_inv]).
    /// However, it does not return the variable's identifier, but its [Var] expression's identifier.
    /// This is usually more convenient.
    fn add_var(&mut self, var: &'a str) -> Id {
        let id = self.vars.len();
        let id_signed: i32 = id.try_into().unwrap();
        self.vars.insert(id, var);
        self.vars_inv.insert(var, id_signed);
        self.expr(Var(id_signed))
    }

    /// Looks ups the identifier for the [Var] expression of a variable in this formula.
    ///
    /// Works analogously to [Formula::get_expr] (see [Formula::vars_inv]).
    /// As for [Formula::add_var], it is usually more convenient to return the [Var] expression's identifier.
    fn get_var(&mut self, var: &str) -> Option<Id> {
        Some(self.expr(Var(*self.vars_inv.get(var)?)))
    }

    /// Adds or looks up a variable of this formula, returning its [Var] expression's identifier.
    ///
    /// This is the preferred way to obtain a [Var] expression's identifier (see [Formula::expr]).
    pub(crate) fn var(&mut self, var: &'a str) -> Id {
        self.get_var(var).unwrap_or_else(|| self.add_var(var))
    }

    /// Returns the root expression of this formula.
    ///
    /// That is, we return the only child of the auxiliary root expression (see [Formula::aux_root_id]).
    pub(crate) fn get_root_expr(&self) -> Id {
        self.assert_valid();
        if let And(ids) = &self.exprs[self.aux_root_id] {
            assert!(ids.len() == 1, "aux root has more than one child");
            ids[0]
        } else {
            panic!("formula is invalid")
        }
    }

    /// Sets the root expression of this formula.
    ///
    /// That is, we update this formula's auxiliary root expression with the given expression (see [Formula::aux_root_id]).
    /// For a formula to be valid, the root expression has to be set at least once.
    /// It may also be updated subsequently to focus on other expressions of the formula.
    pub(crate) fn set_root_expr(&mut self, root_id: Id) {
        self.aux_root_id = self.expr(And(vec![root_id]));
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
    fn set_child_exprs(&mut self, id: Id, mut ids: Vec<Id>) {
        for id in ids.iter_mut() {
            *id = self.get_expr(&self.exprs[*id]).unwrap(); // todo: (when) does this actually do something?
        }
        match &mut self.exprs[id] {
            Var(_) => (),
            Not(id) => *id = ids[0],
            And(child_ids) | Or(child_ids) => *child_ids = ids,
        };
        self.exprs_inv
            .entry(Self::hash_expr(&self.exprs[id]))
            .or_default()
            .push(id);
    }

    /// Resets the auxiliary root expression, if necessary.
    ///
    /// If the auxiliary root expression is mutated with [Formula::set_child_exprs], structural sharing might be violated.
    /// Because [Formula::set_child_exprs] can only address this issue for children,
    /// we need not explicitly address the only expression that is not a child itself - the auxiliary root expression.
    fn reset_aux_root_expr(&mut self) {
        self.aux_root_id = self.get_expr(&self.exprs[self.aux_root_id]).unwrap();
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
        let printed_id = if PRINT_IDENTIFIER { format!("@{id}") } else { String::from("") };
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
                write!(f, "{}{printed_id}", self.vars.get(var_id).unwrap())
            }
            Not(id) => write_helper("Not", slice::from_ref(id)),
            And(ids) => write_helper("And", ids),
            Or(ids) => write_helper("Or", ids),
        }
    }

    /// `TODO`
    fn is_non_aux_and(&self, id: Id) -> bool {
        if let And(_) = self.exprs[id] {
            id != self.aux_root_id
        } else {
            false
        }
    }

    /// `TODO`
    fn splice_or(&self, clause_id: Id, new_clause: &mut Vec<Id>) {
        // splice child or's
        if let Or(literal_ids) = &self.exprs[clause_id] {
            for literal_id in literal_ids {
                new_clause.push(*literal_id);
            }
        } else {
            new_clause.push(clause_id);
        }
    }

    /// `TODO`
    fn dedup(mut vec: Vec<Id>) -> Vec<Id> {
        // (inefficient) deduplication for idempotency
        vec.sort();
        vec.dedup();
        vec
    }

    /// Visits all sub-expressions of this formula using a reverse preorder traversal.
    ///
    /// To preserve structural sharing, we assume that the given visitor is idempotent and only performs mutation
    /// with the designated methods, such as [Formula::var], [Formula::expr] and [Formula::set_child_exprs].
    /// The visitor is called at most once per unique sub-expression:
    /// It will not be called several times on the same sub-expression - this leverages structural sharing.
    /// However, we can also not guarantee it to be called on all sub-expressions - as it might change the set of sub-expressions.
    /// For improved performance, the traversal is reversed, so children are traversed right-to-left.
    fn preorder_rev(&mut self, visitor: fn(&mut Self, Id) -> ()) {
        self.assert_valid();
        let mut remaining_ids = vec![self.aux_root_id];
        let mut visited_ids = HashSet::<Id>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.pop().unwrap();
            if !visited_ids.contains(&id) {
                visitor(self, id);
                remaining_ids.extend(Self::get_child_exprs(&self.exprs[id]));
                visited_ids.insert(id);
            }
        }
        self.reset_aux_root_expr();
    }

    /// Visits all sub-expressions of this formula using a reverse postorder traversal.
    ///
    /// Conceptually, this is similar to [Formula::preorder_rev], but sub-expressions are visited bottom-up instead of top-down.
    fn postorder_rev(&mut self, visitor: fn(&mut Self, Id) -> ()) {
        self.assert_valid();
        let mut remaining_ids = vec![self.aux_root_id];
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
        self.reset_aux_root_expr();
    }

    /// Prints all sub-expression of this formula.
    pub fn print_sub_exprs(&mut self) {
        self.postorder_rev(|formula, id| println!("{}", ExprInFormula(formula, &id)));
    }

    /// Panics if structural sharing is violated in this formula.
    ///
    /// That is, we assert that every sub-expression's identifier is indeed the canonical one.
    pub fn assert_shared(&mut self) {
        self.preorder_rev(|formula, id| {
            assert_eq!(formula.get_expr(&formula.exprs[id]).unwrap(), id)
        });
    }

    /// Transforms this formula into negation normal form by applying De Morgan's laws.
    ///
    /// We do this by traversing the formula top-down and pushing negations towards the leaves (i.e., [Var] expressions).
    /// `TODO`
    pub fn to_nnf(mut self) -> Self {
        self.preorder_rev(|formula, id| {
            let mut child_ids: Vec<Id> = Self::get_child_exprs(&formula.exprs[id]).to_vec();

            for child_id in child_ids.iter_mut() {
                let child = &formula.exprs[*child_id];
                match child {
                    Var(_) | And(_) | Or(_) => (),
                    Not(child2_id) => {
                        let child2 = &formula.exprs[*child2_id];
                        match child2 {
                            Var(_) => (),
                            Not(child3_id) => {
                                *child_id = *child3_id; // this is currently buggy ( violates sharing) in some way, probably due to this line?
                            }
                            And(child_ids2) => {
                                let new_expr = Or(formula.negate_exprs(child_ids2.clone()));
                                *child_id = formula.expr(new_expr);
                            }
                            Or(child_ids2) => {
                                let new_expr = And(formula.negate_exprs(child_ids2.clone()));
                                *child_id = formula.expr(new_expr); // what if we created an and, but are ourselves an and? could merge here!
                            }
                        }
                    }
                }
            }

            formula.set_child_exprs(id, child_ids);
        });
        self
    }

    /// Transforms this formula into conjunctive normal form by applying distributivity laws.
    ///
    /// We do this by traversing the formula bottom-up and pushing [Or] expressions below [And] expressions via multiplication.
    /// This algorithm has exponential worst-case complexity, but ensures logical equivalence to the original formula.
    /// `TODO`
    /// `TODO` refactor code
    /// `TODO` is this idempotent?
    pub fn to_cnf_dist(mut self) -> Self {
        self.postorder_rev(|formula, id| {
            // need the children two times on the stack here, could maybe be disabled, but then merging is more complicated
            let child_ids = Self::get_child_exprs(&formula.exprs[id]).to_vec();
            let mut new_child_ids = Vec::<Id>::new();

            for child_id in child_ids {
                let child = &formula.exprs[child_id];
                match child {
                    Var(_) | Not(_) => new_child_ids.push(child_id),
                    And(cnf_ids) => {
                        if formula.is_non_aux_and(id) || cnf_ids.len() == 1 {
                            new_child_ids.extend(cnf_ids.clone());
                            // new_child_ids.push(self.expr(And(cnf))); // unoptimized version
                        } else {
                            new_child_ids.push(child_id);
                        }
                    }
                    Or(cnf_ids) => {
                        let mut clauses = Vec::<Vec<Id>>::new();
                        for (i, cnf_id) in cnf_ids.iter().enumerate() {
                            // there might be a bug here: Or(...) should be moved to the first arm as | Or(_)
                            let clause_ids = match &formula.exprs[*cnf_id] {
                                Var(_) | Not(_) | Or(_) => slice::from_ref(cnf_id),
                                And(ids) => ids,
                            };

                            if i == 0 {
                                clauses.extend(
                                    // possibly this can be done with a neutral element instead
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
                            clause = Self::dedup(clause); // idempotency
                                                          // unary or
                            if clause.len() > 1 {
                                new_cnf_ids.push(formula.expr(Or(clause)));
                            } else {
                                new_cnf_ids.push(clause[0]);
                            }
                        }
                        if formula.is_non_aux_and(id) || new_cnf_ids.len() == 1 {
                            // splice into parent and
                            new_child_ids.extend(new_cnf_ids);
                            // new_child_ids.push(self.expr(And(cnf))); // unoptimized version
                        } else {
                            new_child_ids.push(formula.expr(And(new_cnf_ids)));
                        }
                    }
                }
            }

            formula.set_child_exprs(id, Self::dedup(new_child_ids));
        });

        //self.assert_shared(); // todo: this should be checked before and after each traversal (and it currently panics?)
        self
    }
}

/// Displays an expression in a formula.
impl<'a> fmt::Display for ExprInFormula<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.assert_valid();
        self.0.format_expr(*self.1, f)
    }
}

/// Displays a formula.
impl<'a> fmt::Display for Formula<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ExprInFormula(self, &self.get_root_expr()).fmt(f)
    }
}
