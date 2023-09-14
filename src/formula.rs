use std::{
    collections::{HashMap, HashSet},
    fmt, slice,
};
use Expr::*;

pub type Id = u32;
pub type VarId = i32;

// optional features (disable with #cfg, so binary can be optimized):
// - invariant: no formula is in memory twice, so parse with structural sharing or without, reuse cached formulas
// - run NNF before other transformations, or don't run it before (interacts with PG and structural sharing)
// - merge And/Or, auto-simplify terms while creating NNF/CNF or only do it afterwards??

#[derive(Debug)]
pub struct Formula<'a> {
    aux_root_id: Id,
    next_id: Id,
    next_var_id: VarId,
    // possibly, Rc is needed to let go of unused formulas
    // RefCell for internal mutability (do not create unnecessary copies)
    // and Box for faster moving??
    exprs: HashMap<Id, Expr>,
    vars: HashMap<VarId, &'a str>,
    vars_inv: HashMap<&'a str, VarId>,
    // make structural sharing optional, so that we can evaluate its impact (e.g., then traversal does not need to track visited nodes)
}

#[derive(Debug)]
pub enum Expr {
    Var(VarId),
    Not(Id),
    And(Vec<Id>),
    Or(Vec<Id>),
}

pub struct ExprInFormula<'a>(&'a Formula<'a>, &'a Id);

impl<'a> Formula<'a> {
    pub fn new() -> Self {
        Self {
            aux_root_id: 0,
            next_id: 0,
            next_var_id: 0,
            exprs: HashMap::new(),
            vars: HashMap::new(),
            vars_inv: HashMap::new(),
        }
    }

    fn assert_valid(&self) {
        assert!(
            self.aux_root_id > 0 && self.next_id > 0 && self.next_var_id > 0,
            "formula is invalid"
        );
    }

    fn get_root_expr(&self) -> Id {
        self.assert_valid();
        if let And(ids) = self.exprs.get(&self.aux_root_id).unwrap() {
            assert!(ids.len() == 1, "aux root has more than one child");
            ids[0]
        } else {
            panic!("formula is invalid")
        }
    }

    pub fn set_root_expr(&mut self, root_id: Id) {
        let aux_root_id = self.add_expr(And(vec![root_id]));
        self.aux_root_id = aux_root_id;
    }

    pub fn add_expr(&mut self, expr: Expr) -> Id {
        let id = self.next_id + 1;
        self.exprs.insert(id, expr);
        self.next_id = id;
        id
    }

    pub fn add_var(&mut self, var: &'a str) -> Id { // remove all pub's
        let id = self.next_var_id + 1;
        self.vars.insert(id, var);
        self.vars_inv.insert(var, id);
        self.next_var_id += 1;
        self.add_expr(Var(id))
    }

    pub fn get_var(&mut self, var: &str) -> Id {
        self.add_expr(Var(*self.vars_inv.get(var).unwrap()))
    }

    // pub fn var(&mut self, var: &str) -> Id {

    // }

    fn get_child_exprs<'b>(&self, expr: &'b Expr) -> &'b [Id] {
        match expr {
            Var(_) => &[],
            Not(id) => slice::from_ref(id),
            And(ids) | Or(ids) => ids,
        }
    }

    fn set_child_exprs(&mut self, id: Id, new_ids: Vec<Id>) -> &[Id] {
        match self.exprs.get_mut(&id).unwrap() {
            Var(_) => &[],
            Not(id) => {
                *id = new_ids[0];
                slice::from_ref(id)
            }
            And(ids) | Or(ids) => {
                *ids = new_ids;
                ids
            }
        }
    }

    fn negate_exprs(&mut self, ids: Vec<Id>) -> Vec<Id> {
        ids.iter().map(|id| self.add_expr(Not(*id))).collect() // use cached formulas!
    }

    fn child_exprs_refl<'b>(&'b self, id: &'b Id) -> &'b [Id] {
        match self.exprs.get(&id).unwrap() {
            Var(_) | Not(_) => slice::from_ref(&id),
            And(ids) | Or(ids) => ids,
        }
    }

    fn is_non_aux_and(&self, id: Id) -> bool {
        if let And(_) = self.exprs.get(&id).unwrap() {
            id != self.aux_root_id
        } else {
            false
        }
    }

    fn splice_or(&self, clause_id: Id, new_clause: &mut Vec<Id>) {
        // splice child or's
        if let Or(literal_ids) = self.exprs.get(&clause_id).unwrap() {
            for literal_id in literal_ids {
                new_clause.push(*literal_id);
            }
        } else {
            new_clause.push(clause_id);
        }
    }

    fn dedup(mut vec: Vec<Id>) -> Vec<Id> {
        // (inefficient) deduplication for idempotency
        vec.sort();
        vec.dedup();
        vec
    }

    pub(crate) fn get_vars(&self) -> HashMap<VarId, &str> {
        self.vars.clone()
    }

    // requires CNF
    pub(crate) fn get_clauses(&self) -> Vec<Vec<VarId>> {
        let mut clauses = Vec::<Vec<VarId>>::new();

        let add_literal = |id, clause: &mut Vec<VarId>| match self.exprs.get(&id).unwrap() {
            Var(var_id) => clause.push(*var_id),
            Not(child_id) => {
                if let Var(var_id) = self.exprs.get(&child_id).unwrap() {
                    clause.push(-*var_id);
                } else {
                    panic!("expected Var below Not, got {}", ExprInFormula(self, &id));
                }
            }
            _ => panic!("expected Var or Not literal, got {}", ExprInFormula(self, &id)),
        };

        let mut add_clause = |child_ids: &[Id]| {
            let mut clause = Vec::<VarId>::new();
            for child_id in child_ids {
                add_literal(*child_id, &mut clause);
            }
            clauses.push(clause);
        };

        match self.exprs.get(&self.get_root_expr()).unwrap() {
            Var(_) | Not(_) => add_clause(slice::from_ref(&self.get_root_expr())),
            Or(child_ids) => add_clause(child_ids),
            And(child_ids) => {
                for child_id in child_ids {
                    match self.exprs.get(&child_id).unwrap() {
                        Var(_) | Not(_) => add_clause(slice::from_ref(child_id)),
                        Or(child_ids) => add_clause(child_ids),
                        _ => panic!("expected Var, Not, or Or expression, got {}", ExprInFormula(self, child_id)),
                    }
                }
            }
        }

        clauses
    }

    fn format_expr(&self, id: Id, f: &mut fmt::Formatter) -> fmt::Result {
        // rewrite with preorder traversal?
        let mut write_helper = |kind: &str, ids: &[Id]| {
            write!(f, "{kind}(")?;
            for (i, id) in ids.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                self.format_expr(*id, f)?;
            }
            write!(f, ")")
        };
        match self.exprs.get(&id).unwrap() {
            Var(var_id) => write!(f, "{}", self.vars.get(var_id).unwrap()),
            Not(id) => write_helper("Not", slice::from_ref(id)),
            And(ids) => write_helper("And", ids),
            Or(ids) => write_helper("Or", ids),
        }
    }

    fn print_expr(&mut self, id: Id) {
        println!("{}", ExprInFormula(self, &id));
    }

    // adds new expressions without discarding the old ones if they get orphaned (use Rc?)
    // creates temporary vector (use RefCell?)
    // may destroy structural sharing of originally shared subformulas,
    // so might be beneficial to not run this before Tseitin
    // (this might largely influence negation-CNF reasoning);
    // so, also a polarity-based PG implementation is necessary
    fn to_nnf_expr(&mut self, id: Id) -> &[Id] {
        let mut child_ids: Vec<Id> = self.get_child_exprs(self.exprs.get(&id).unwrap()).to_vec();

        for child_id in child_ids.iter_mut() {
            let child = self.exprs.get(&child_id).unwrap();
            match child {
                Var(_) | And(_) | Or(_) => (),
                Not(child2_id) => {
                    let child2 = self.exprs.get(child2_id).unwrap();
                    match child2 {
                        Var(_) => (),
                        Not(child3_id) => {
                            *child_id = *child3_id;
                        }
                        And(child_ids2) => {
                            // this does not reuse existing formulas yet! => need cache to retrieve formulas (similar for other expr constructions)
                            let new_expr = Or(self.negate_exprs(child_ids2.clone()));
                            *child_id = self.add_expr(new_expr);
                        }
                        Or(child_ids2) => {
                            let new_expr = And(self.negate_exprs(child_ids2.clone()));
                            *child_id = self.add_expr(new_expr); // what if we created an and, but are ourselves an and? could merge here!
                        }
                    }
                }
            }
        }

        self.set_child_exprs(id, child_ids)
    }

    // assumes NNF
    fn to_cnf_expr_dist(&mut self, id: Id) -> () {
        // need the children two times on the stack here, could maybe be disabled, but then merging is more complicated
        let child_ids = self.get_child_exprs(self.exprs.get(&id).unwrap()).to_vec();
        let mut new_child_ids = Vec::<Id>::new();

        for child_id in child_ids {
            let child = self.exprs.get(&child_id).unwrap();
            match child {
                Var(_) | Not(_) => new_child_ids.push(child_id),
                And(cnf_ids) => {
                    if self.is_non_aux_and(id) || cnf_ids.len() == 1 {
                        new_child_ids.extend(cnf_ids.clone());
                        // new_child_ids.push(self.add_expr(And(cnf))); // unoptimized version
                    } else {
                        new_child_ids.push(child_id);
                    }
                }
                Or(cnf_ids) => {
                    let mut clauses = Vec::<Vec<Id>>::new();
                    for (i, cnf_id) in cnf_ids.iter().enumerate() {
                        let clause_ids = self.child_exprs_refl(cnf_id);
                        if i == 0 {
                            clauses.extend( // possibly this can be done with a neutral element instead
                                clause_ids
                                    .iter()
                                    .map(|clause_id| {
                                        let mut new_clause = Vec::<Id>::new();
                                        self.splice_or(*clause_id, &mut new_clause);
                                        new_clause
                                    })
                                    .collect::<Vec<Vec<Id>>>(),
                            );
                        } else {
                            let mut new_clauses = Vec::<Vec<Id>>::new();
                            for clause in &clauses {
                                for clause_id in clause_ids {
                                    let mut new_clause = clause.clone();
                                    self.splice_or(*clause_id, &mut new_clause);
                                    new_clauses.push(new_clause);
                                }
                            }
                            clauses = new_clauses;
                        }
                    }
                    let mut new_cnf_ids = Vec::<Id>::new();
                    for mut clause in clauses {
                        clause = Self::dedup(clause); // idempotency
                        if clause.len() > 1 { // unary or
                            new_cnf_ids.push(self.add_expr(Or(clause))); // use cached formula
                        } else {
                            new_cnf_ids.push(clause[0]);
                        }
                    }
                    if self.is_non_aux_and(id) || new_cnf_ids.len() == 1 { // splice into parent and
                        new_child_ids.extend(new_cnf_ids);
                        // new_child_ids.push(self.add_expr(And(cnf))); // unoptimized version
                    } else {
                        new_child_ids.push(self.add_expr(And(new_cnf_ids)));
                    }
                }
            }
        }

        self.set_child_exprs(id, Self::dedup(new_child_ids));
    }

    fn reverse_preorder(&mut self, visitor: fn(&mut Self, Id) -> &[Id]) {
        self.assert_valid();
        let mut remaining_ids = vec![self.aux_root_id];
        // presumably, the following set can get large for large formulas (some for postorder traversal).
        // maybe it can be compacted in some way. (bit matrix? pre-sized vec<bool> with false as default?)
        let mut visited_ids = HashSet::<Id>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.pop().unwrap();
            if !visited_ids.contains(&id) {
                remaining_ids.extend(visitor(self, id));
                visited_ids.insert(id);
            }
        }
    }

    fn reverse_postorder(&mut self, visitor: fn(&mut Self, Id) -> ()) {
        self.assert_valid();
        let mut remaining_ids = vec![self.aux_root_id];
        let mut seen_ids = HashSet::<Id>::new();
        let mut visited_ids = HashSet::<Id>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.last().unwrap();
            let child_ids = self.get_child_exprs(self.exprs.get(id).unwrap());
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
    }

    // combine pre- and postorder to a DFS that creates NNF on first and distributive CNF on last visit

    pub fn print_subexprs(&mut self) {
        self.reverse_postorder(|s, i| s.print_expr(i));
    }

    pub fn to_nnf(mut self) -> Self {
        self.reverse_preorder(Self::to_nnf_expr);
        self
    }

    pub fn to_cnf_dist(mut self) -> Self {
        self.reverse_postorder(Self::to_cnf_expr_dist);
        self
    }
}

impl<'a> fmt::Display for ExprInFormula<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.assert_valid();
        self.0.format_expr(*self.1, f)
    }
}

impl<'a> fmt::Display for Formula<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ExprInFormula(self, &self.get_root_expr()).fmt(f)
    }
}

// todo: structural reuse; parse SAT; tseitin; RC for releasing old subformulas??

// tseitin formula also has a 'pointer' to another formula, to ease the actual substitution
// add optimizations for simplification? (e.g., idempotency, pure literals, Plaisted, ... -> depending on whether equi-countability is preserved/necessary)
// what about eliminating implies/bi-implies? can be exponential, too
// https://cca.informatik.uni-freiburg.de/sat/ss23/04/
// https://cca.informatik.uni-freiburg.de/sat/ss23/05/
// randomize clause order? (scrambler?)
// during parsing, when the hash of a particular subformula has already been mapped to a usize (already included in the formula), reuse that usize
// possibly, we need a HashMap<Expr, usize> during parsing to ensure structural sharing
// the next_id approach does not work with multi-threading
// assumes that each expr only has each child at most once (idempotency is already reduced)

// how much impact does structural sharing of common sub-formulas have? does it even happen for FMs?

#[cfg(test)]
mod tests {
    use super::*;

    mod valid {
        use super::*;

        #[test]
        #[should_panic(expected = "formula is invalid")]
        fn empty() {
            Formula::new().to_string();
        }

        #[test]
        #[should_panic(expected = "formula is invalid")]
        fn no_root() {
            let mut f = Formula::new();
            let a = f.add_var("a");
            f.add_expr(Not(a));
            f.to_string();
        }

        #[test]
        fn valid() {
            let mut f = Formula::new();
            let a = f.add_var("a");
            let not_a = f.add_expr(Not(a));
            f.set_root_expr(not_a);
            f.to_string();
        }
    }

    mod nnf {
        use super::*;

        #[test]
        fn not_a() {
            let mut f = Formula::new();
            let a = f.add_var("a");
            let not_a = f.add_expr(Not(a));
            f.set_root_expr(not_a);
            assert_eq!(f.to_nnf().to_string(), "Not(a)");
        }

        #[test]
        fn not_not_a() {
            let mut f = Formula::new();
            let a = f.add_var("a");
            let not_a = f.add_expr(Not(a));
            let not_not_a = f.add_expr(Not(not_a));
            f.set_root_expr(not_not_a);
            assert_eq!(f.to_nnf().to_string(), "a");
        }

        #[test]
        fn and_not_not_a() {
            let mut f = Formula::new();
            let a = f.add_var("a");
            let not_a = f.add_expr(Not(a));
            let not_not_a = f.add_expr(Not(not_a));
            let and = f.add_expr(And(vec![not_not_a]));
            f.set_root_expr(and);
            assert_eq!(f.to_nnf().to_string(), "And(a)");
        }

        #[test]
        fn complex() {
            let mut f = Formula::new();
            let a = f.add_var("a");
            let b = f.add_var("b");
            let c = f.add_var("c");
            let not_a = f.add_expr(Not(a));
            let not_b = f.add_expr(Not(b));
            let not_c = f.add_expr(Not(c));
            let not_not_c = f.add_expr(Not(not_c));
            let not_a_and_c = f.add_expr(And(vec![not_a, c]));
            let not_b_or_not_not_c_or_not_a_and_c =
                f.add_expr(Or(vec![not_b, not_not_c, not_a_and_c]));
            let not_not_b_or_not_not_c_or_not_a_and_c =
                f.add_expr(Not(not_b_or_not_not_c_or_not_a_and_c));
            let not_not_not_b_or_not_not_c_or_not_a_and_c =
                f.add_expr(Not(not_not_b_or_not_not_c_or_not_a_and_c));
            let root = f.add_expr(Or(vec![
                not_a_and_c,
                not_not_b_or_not_not_c_or_not_a_and_c,
                not_not_not_b_or_not_not_c_or_not_a_and_c,
            ]));
            f.set_root_expr(root);
            assert_eq!(
                f.to_nnf().to_string(),
                "Or(And(Not(a), c), And(b, Not(c), Or(a, Not(c))), Or(Not(b), c, And(Not(a), c)))"
            );
        }
    }

    mod cnf_dist {
        use super::*;

        #[test]
        fn simple() {
            let mut f = Formula::new();
            let a = f.add_var("a");
            let b = f.add_var("b");
            let a_and_b = f.add_expr(And(vec![a, b]));
            let a_or_a_and_b = f.add_expr(Or(vec![a, a_and_b]));
            let a_and_a_or_a_and_b = f.add_expr(And(vec![a, a_or_a_and_b]));
            f.set_root_expr(a_and_a_or_a_and_b);
            f = f.to_nnf().to_cnf_dist();
            assert_eq!(f.to_string(), "And(a, Or(a, b))");
        }

        #[test]
        fn complex() {
            let mut f = Formula::new();
            let a = f.add_var("a");
            let b = f.add_var("b");
            let c = f.add_var("c");
            let not_a = f.add_expr(Not(a));
            let not_b = f.add_expr(Not(b));
            let not_c = f.add_expr(Not(c));
            let not_not_c = f.add_expr(Not(not_c));
            let a_and_c = f.add_expr(And(vec![not_a, c]));
            let b_or_c = f.add_expr(Or(vec![not_b, not_not_c, a_and_c]));
            let not_b_or_c = f.add_expr(Not(b_or_c));
            let not_not_b_or_c = f.add_expr(Not(not_b_or_c));
            let root = f.add_expr(Or(vec![a_and_c, not_b_or_c, not_not_b_or_c]));
            f.set_root_expr(root);
            f = f.to_nnf().to_cnf_dist();
            assert_eq!(f.to_string(), "And(Or(b, c, Not(b)), Or(b, c, Not(a), Not(b)), Or(c, Not(b), Not(c)), Or(c, Not(a), Not(b), Not(c)), Or(a, c, Not(b), Not(c)), Or(a, c, Not(a), Not(b), Not(c)), Or(b, c, Not(a), Not(b)), Or(b, c, Not(a), Not(b)), Or(c, Not(a), Not(b), Not(c)), Or(c, Not(a), Not(b), Not(c)), Or(a, c, Not(a), Not(b), Not(c)), Or(a, c, Not(a), Not(b), Not(c)))");
        }
    }
}
