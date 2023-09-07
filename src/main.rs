use std::{collections::HashMap, fmt, slice};
use Expr::*;

// tseitin formula also has a 'pointer' to another formula, to ease the actual substitution
// add optimizations for simplification? (e.g., idempotency)
// randomize clause order? (scrambler?)
// during parsing, when the hash of a particular subformula has already been mapped to a usize (already included in the formula), reuse that usize

type Id = u32;

#[derive(Debug)]
struct Formula {
    root_id: Id,
    next_id: Id,
    next_var_id: Id,
    exprs: HashMap<Id, Expr>,
    vars: HashMap<Id, String>,
    // possibly, we need a HashMap<Expr, usize> during parsing to ensure structural sharing
}

// #[derive(Debug)]
// struct CNF {
//     variables: HashMap<Id, String>, // is this sorted?
//     clauses: Vec<Vec<i32>>,
// }

#[derive(Debug)]
enum Expr {
    Var(Id),
    Not(Id),
    And(Vec<Id>),
    Or(Vec<Id>),
}

impl Formula {
    fn new() -> Formula {
        Formula {
            root_id: 0,
            next_id: 0,
            next_var_id: 0,
            exprs: HashMap::new(),
            vars: HashMap::new(),
        }
    }

    fn set_root(&mut self, id: Id) {
        self.root_id = id;
    }

    fn add_expr(&mut self, expr: Expr) -> u32 {
        let next_id = self.next_id + 1;
        self.exprs.insert(next_id, expr);
        self.next_id += 1;
        next_id
    }

    fn add_var(&mut self, var: &str) -> u32 {
        let next_var_id = self.next_var_id + 1;
        self.vars.insert(next_var_id, String::from(var));
        self.next_var_id += 1;
        self.add_expr(Var(next_var_id))
    }

    fn fmt(&self, id: Id, f: &mut fmt::Formatter) {
        let mut write = |kind: &str, ids: &[u32]| {
            write!(f, "{kind}(").ok();
            let mut i = 0;
            for id in ids {
                if i > 0 {
                    write!(f, ", ").ok();
                }
                i += 1;
                self.fmt(*id, f);
            }
            write!(f, ")").ok();
        };
        match self.exprs.get(&id).unwrap() {
            Var(var_id) => {
                write!(f, "{}", self.vars.get(var_id).unwrap()).ok();
            }
            Not(id) => {
                write("Not", slice::from_ref(id));
            }
            And(ids) => {
                write("And", ids);
            }
            Or(ids) => {
                write("Or", ids);
            }
        }
    }

    fn to_nnf(&mut self, id: Id) {
        let expr = self.exprs.get(&id).unwrap();
        match expr {
            Var(_) => todo!(),
            Not(_) => todo!(),
            And(child_ids) | Or(child_ids) => {
                for (idx, child_id) in child_ids.iter().enumerate() {
                    let child = self.exprs.get(&child_id).unwrap();
                    match child {
                        Var(_) => todo!(),
                        Not(child2_id) => {
                            let child2 = self.exprs.get(child2_id).unwrap();
                            match child2 {
                                Var(_) => (),
                                Not(child3_id) => {
                                    if let And(c) = self.exprs.get_mut(&id).unwrap() {
                                        c[idx] = *child3_id;
                                    }
                                }
                                And(_) => todo!(),
                                Or(_) => todo!(),
                            }
                        }
                        And(_) => todo!(),
                        Or(_) => todo!(),
                    }
                }
            }
        }
    }
}

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt(self.root_id, f);
        write!(f, "")
    }
}

fn main() {
    let mut f = Formula::new();
    let a = f.add_var("a");
    let b = f.add_var("b");
    let c = f.add_var("c");
    let a_and_c = f.add_expr(And(vec![a, c]));
    let b_or_c = f.add_expr(Or(vec![b, c]));
    let not_b_or_c = f.add_expr(Not(b_or_c));
    let root = f.add_expr(Or(vec![a_and_c, not_b_or_c, b_or_c]));
    f.set_root(root);
    println!("{f}");
}
