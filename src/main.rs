use std::{
    collections::HashMap,
    fmt, slice,
};
use Expr::*;

type Id = u32;

#[derive(Debug)]
struct Formula {
    root_id: Id,
    next_id: Id,
    next_var_id: Id,
    exprs: HashMap<Id, Expr>,
    vars: HashMap<Id, String>,
}

#[derive(Debug)]
enum Expr {
    Var(Id),
    Not(Id),
    And(Vec<Id>),
    Or(Vec<Id>),
}

impl Formula {
    fn new() -> Self {
        Self {
            root_id: 0,
            next_id: 0,
            next_var_id: 0,
            exprs: HashMap::new(),
            vars: HashMap::new(),
        }
    }

    fn assert_valid(&self) {
        debug_assert!(self.root_id > 0 && self.next_id > 0 && self.next_var_id > 0);
    }

    fn set_root_expr(&mut self, root_id: Id) {
        self.root_id = root_id;
    }

    fn add_expr(&mut self, expr: Expr) -> Id {
        let id = self.next_id + 1;
        self.exprs.insert(id, expr);
        self.next_id = id;
        id
    }

    fn add_var_str(&mut self, var: String) -> Id {
        let id = self.next_var_id + 1;
        self.vars.insert(id, var);
        self.next_var_id += 1;
        self.add_expr(Var(id))
    }

    fn add_var(&mut self, var: &str) -> Id {
        self.add_var_str(String::from(var))
    }

    fn get_child_exprs<'a>(&self, expr: &'a Expr) -> &'a [Id] {
        match expr {
            Var(_) => &[],
            Not(id) => slice::from_ref(id),
            And(ids) | Or(ids) => ids,
        }
    }

    fn set_child_exprs<'a>(&mut self, id: Id, ids: Vec<Id>) {
        match self.exprs.get_mut(&id).unwrap() {
            Var(_) => (),
            Not(child) => *child = ids[0],
            And(child_ids) | Or(child_ids) => *child_ids = ids,
        }
    }

    fn negate_exprs(&mut self, ids: Vec<Id>) -> Vec<Id> {
        ids.iter()
            .map(|id| self.add_expr(Not(*id))) // does not reuse existing formulas! => need cache to retrieve formulas
            .collect()
    }

    fn format_expr(&self, id: Id, f: &mut fmt::Formatter) -> fmt::Result {
        self.assert_valid();
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

    // adds new expressions without discarding the old ones if they get orphaned
    fn to_nnf_expr(&mut self, id: Id) -> &[Id] {
        self.assert_valid();
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

        self.set_child_exprs(id, child_ids);
        self.get_child_exprs(self.exprs.get(&id).unwrap())
    }

    fn reverse_preorder(mut self, visitor: fn(&mut Self, Id) -> &[Id]) -> Self {
        self.assert_valid();
        let mut id = Some(self.root_id);
        let mut remaining_ids = Vec::<Id>::new();
        while id.is_some() {
            remaining_ids.extend(visitor(&mut self, id.unwrap()));
            id = remaining_ids.pop();
        };
        self
    }

    fn to_nnf(self) -> Self {
        self.reverse_preorder(Self::to_nnf_expr)
    }

}

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format_expr(self.root_id, f)
    }
}

fn main() {
    let mut f = Formula::new();
    let a = f.add_var("a");
    let b = f.add_var("b");
    let c = f.add_var("c");
    let not_a = f.add_expr(Not(a));
    let not_b = f.add_expr(Not(b));
    let not_c = f.add_expr(Not(c));
    let not_not_c = f.add_expr(Not(not_c));
    let a_and_c = f.add_expr(And(vec![not_a, c]));
    let b_or_c = f.add_expr(Or(vec![not_b, not_not_c]));
    let not_b_or_c = f.add_expr(Not(b_or_c));
    let not_not_b_or_c = f.add_expr(Not(not_b_or_c));
    let root = f.add_expr(Or(vec![a_and_c, not_b_or_c, not_not_b_or_c]));
    f.set_root_expr(root);
    println!("{f}");
    f = f.to_nnf();
    println!("{f}");
}

// tseitin formula also has a 'pointer' to another formula, to ease the actual substitution
// add optimizations for simplification? (e.g., idempotency, pure literals, Plaisted, ... -> depending on whether equi-countability is preserved/necessary)
// https://cca.informatik.uni-freiburg.de/sat/ss23/05/
// randomize clause order? (scrambler?)
// during parsing, when the hash of a particular subformula has already been mapped to a usize (already included in the formula), reuse that usize
// possibly, we need a HashMap<Expr, usize> during parsing to ensure structural sharing
// the next_id approach does not work with multi-threading
// assumes that each expr only has each child at most once (idempotency is already reduced)

// #[derive(Debug)]
// struct CNF {
//     variables: HashMap<Id, String>, // is this sorted?
//     clauses: Vec<Vec<i32>>,
// }
