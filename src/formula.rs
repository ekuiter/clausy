use std::{
    collections::{HashMap, HashSet},
    fmt, slice,
};
use Expr::*;

type Id = u32;

pub struct Formula {
    root_id: Id,
    next_id: Id,
    next_var_id: Id,
    // possibly, Rc is needed to let go of unused formulas
    // RefCell for internal mutability (do not create unnecessary copies)
    // and Box for faster moving??
    exprs: HashMap<Id, Expr>,
    vars: HashMap<Id, String>,
    // make structural sharing optional, so that we can evaluate its impact (e.g., then traversal does not need to track visited nodes)
}

pub enum Expr {
    Var(Id),
    Not(Id),
    And(Vec<Id>),
    Or(Vec<Id>),
}

pub struct ExprInFormula<'a>(&'a Formula, &'a Id);

impl Formula {
    pub fn new() -> Self {
        Self {
            root_id: 0,
            next_id: 0,
            next_var_id: 0,
            exprs: HashMap::new(),
            vars: HashMap::new(),
        }
    }

    fn assert_valid(&self) {
        assert!(self.root_id > 0 && self.next_id > 0 && self.next_var_id > 0, "formula is invalid");
    }

    pub fn set_root_expr(&mut self, root_id: Id) {
        self.root_id = root_id;
    }

    pub fn add_expr(&mut self, expr: Expr) -> Id {
        let id = self.next_id + 1;
        self.exprs.insert(id, expr);
        self.next_id = id;
        id
    }

    pub fn add_var_str(&mut self, var: String) -> Id {
        let id = self.next_var_id + 1;
        self.vars.insert(id, var);
        self.next_var_id += 1;
        self.add_expr(Var(id))
    }

    pub fn add_var(&mut self, var: &str) -> Id {
        self.add_var_str(String::from(var))
    }

    fn get_child_exprs<'a>(&self, expr: &'a Expr) -> &'a [Id] {
        match expr {
            Var(_) => &[],
            Not(id) => slice::from_ref(id),
            And(ids) | Or(ids) => ids,
        }
    }

    fn set_child_exprs<'a>(&mut self, id: Id, new_ids: Vec<Id>) -> &[Id] {
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
        ids.iter().map(|id| self.add_expr(Not(*id))).collect()
    }

    fn child_exprs_refl<'a>(&'a self, id: &'a Id) -> &'a [Id] {
        match self.exprs.get(&id).unwrap() {
            Var(_) | Not(_) => slice::from_ref(&id),
            And(ids) | Or(ids) => ids,
        }
    }

    fn format_expr(&self, id: Id, f: &mut fmt::Formatter) -> fmt::Result {
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
    // assumes that the root is not a Not (force auxiliary And as root?)
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
                            *child_id = self.add_expr(new_expr);
                        }
                    }
                }
            }
        }

        self.set_child_exprs(id, child_ids)
    }

    // assumes NNF
    fn to_cnf_expr_dist(&mut self, id: Id) -> () {
        let mut child_ids: Vec<Id> = self.get_child_exprs(self.exprs.get(&id).unwrap()).to_vec();

        for child_id in child_ids.iter_mut() {
            let child = self.exprs.get(&child_id).unwrap();
            match child {
                // it is still necessary to flatten And's and Or's
                // also, what about empty/unary Or and And?
                Var(_) | Not(_) | And(_) => (),
                Or(cnf_ids) => { // what happens if this is empty/unary?
                    let mut clauses = Vec::<Vec<Id>>::new();
                    for (i, cnf_id) in cnf_ids.iter().enumerate() {
                        let clause_ids = self.child_exprs_refl(cnf_id);
                        if i == 0 {
                            clauses.extend(clause_ids.iter().map(|clause_id| { vec![*clause_id] }).collect::<Vec<Vec<Id>>>());
                        } else {
                            let mut new_clauses = Vec::<Vec<Id>>::new();
                            for clause in &clauses {
                                for clause_id in clause_ids {
                                    let mut new_clause = clause.clone();
                                    new_clause.push(*clause_id);
                                    new_clauses.push(new_clause);
                                }
                            }
                            clauses = new_clauses;
                        }
                    }
                    let mut cnf = Vec::<Id>::new();
                    for clause in clauses {
                        cnf.push(self.add_expr(Or(clause)));
                    }
                    *child_id = self.add_expr(And(cnf));
                }
            }
        }

        self.set_child_exprs(id, child_ids);
    }

    fn reverse_preorder(&mut self, visitor: fn(&mut Self, Id) -> &[Id]) {
        self.assert_valid();
        let mut remaining_ids = vec![self.root_id];
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
        let mut remaining_ids = vec![self.root_id];
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

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ExprInFormula(self, &self.root_id).fmt(f)
    }
}

// todo: to cnf (distrib); structural reuse; parse SAT/write CNF; tseitin; RC for releasing old subformulas??

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
