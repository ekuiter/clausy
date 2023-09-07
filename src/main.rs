use std::{collections::HashMap, fmt, slice};
use Expr::*;

// tseitin formula also has a 'pointer' to another formula, to ease the actual substitution
// add optimizations for simplification? (e.g., idempotency)
// randomize clause order? (scrambler?)
// during parsing, when the hash of a particular subformula has already been mapped to a usize (already included in the formula), reuse that usize

#[derive(Debug)]
struct Formula {
    root_expr_id: usize,
    exprs: Vec<Expr>,
    vars: Vec<String>,
    // possibly, we need a HashMap<Expr, usize> during parsing to ensure structural sharing
}

#[derive(Debug)]
struct CNF {
    variables: Vec<String>, // is this sorted?
    clauses: Vec<Vec<i32>>,
}

#[derive(Debug)]
enum Expr {
    Var(usize),
    Not(usize),
    And(Vec<usize>),
    Or(Vec<usize>),
}

impl Formula {
    fn new() -> Formula {
        Formula {
            root_expr_id: 0,
            exprs: vec![Var(0)],
            vars: vec![String::new()],
        }
    }

    fn get_children_expr_ids(&self, expr_id: usize) -> &[usize] {
        match &self.exprs[expr_id] {
            Var(_) => &[],
            Not(expr_id) => slice::from_ref(&expr_id),
            And(expr_ids) => &expr_ids[..],
            Or(expr_ids) => &expr_ids[..],
        }
    }

    fn set_root(&mut self, expr_id: usize) {
        self.root_expr_id = expr_id;
    }

    fn add_expr(&mut self, expr: Expr) -> usize {
        let new_expr_id = self.exprs.len();
        self.exprs.push(expr);
        new_expr_id
    }

    fn add_child(&mut self, expr_id: usize, child: Expr) -> usize {
        let new_expr_id = self.exprs.len();
        match &mut self.exprs[expr_id] {
            Var(_) => panic!("can not add a child to a Var expr"),
            Not(ref mut expr_id) => *expr_id = new_expr_id,
            And(expr_ids) => expr_ids.push(new_expr_id),
            Or(expr_ids) => expr_ids.push(new_expr_id),
        }
        self.exprs.insert(new_expr_id, child);
        new_expr_id
    }

    fn add_var(&mut self, var: &str) -> usize {
        let new_var_id = self.vars.len();
        self.vars.push(String::from(var));
        self.add_expr(Var(new_var_id))
    }

    fn fmt(&self, expr_id: usize, f: &mut fmt::Formatter) {
        let mut write = |kind: &str, expr_ids: &[usize]| {
            write!(f, "{kind}(").ok();
            let mut i = 0;
            for expr_id in expr_ids {
                if i > 0 {
                    write!(f, ", ").ok();
                }
                i += 1;
                self.fmt(*expr_id, f);
            }
            write!(f, ")").ok();
        };
        match &self.exprs[expr_id] {
            Var(var_id) => {
                write!(f, "{}", self.vars[*var_id]).ok();
            }
            Not(expr_id) => {
                write("Not", slice::from_ref(expr_id));
            }
            And(expr_ids) => {
                write("And", expr_ids);
            }
            Or(expr_ids) => {
                write("Or", expr_ids);
            }
        }
    }

    fn to_nnf(&mut self, expr_id: usize) {
        let expr = &mut self.exprs[expr_id];
        match expr {
            Var(_) => todo!(),
            Not(_) => todo!(),
            And(ref mut child_ids) | Or(ref mut child_ids) => {
                for child_id in child_ids {
                    let child = &self.exprs[*child_id];
                    if let Not(child2_id) = child {
                        // let child2 = self.exprs.get(child2_id).unwrap();
                        // match child2 {
                        //     Var(_) => (),
                        //     Not(child3_id) => {
                        //         *child_id = *child3_id;
                        //     }
                        //     And(_) => todo!(),
                        //     Or(_) => todo!(),
                        // }
                    }
                }
            }
        }
    }
}

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.fmt(self.root_expr_id, f);
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
