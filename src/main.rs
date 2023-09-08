use std::{collections::HashMap, fmt, slice};
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
    fn new() -> Formula {
        Formula {
            root_id: 0,
            next_id: 0,
            next_var_id: 0,
            exprs: HashMap::new(),
            vars: HashMap::new(),
        }
    }

    fn is_valid(&self) -> bool {
        self.root_id > 0 && self.next_id > 0 && self.next_var_id > 0
    }

    fn set_root(&mut self, root_id: Id) {
        self.root_id = root_id;
    }

    fn add_expr(&mut self, expr: Expr) -> u32 {
        let id = self.next_id + 1;
        self.exprs.insert(id, expr);
        self.next_id = id;
        id
    }

    fn add_var_str(&mut self, var: String) -> u32 {
        let id = self.next_var_id + 1;
        self.vars.insert(id, var);
        self.next_var_id += 1;
        self.add_expr(Var(id))
    }

    fn add_var(&mut self, var: &str) -> u32 {
        self.add_var_str(String::from(var))
    }

    fn format_expr(&self, id: Id, f: &mut fmt::Formatter) -> fmt::Result {
        debug_assert!(self.is_valid());
        let mut write_helper = |kind: &str, ids: &[u32]| {
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

    // fn to_nnf(&mut self, id: Id) {
    //     let expr = self.exprs.get(&id).unwrap();
    //     match expr {
    //         Var(_) => todo!(),
    //         Not(_) => todo!(),
    //         And(child_ids) | Or(child_ids) => {
    //             for (i, child_id) in child_ids.iter().enumerate() {
    //                 let child = self.exprs.get(&child_id).unwrap();
    //                 match child {
    //                     Var(_) => todo!(),
    //                     Not(child2_id) => {
    //                         let child2 = self.exprs.get(child2_id).unwrap();
    //                         match child2 {
    //                             Var(_) => (),
    //                             Not(child3_id) => {
    //                                 if let And(c) = self.exprs.get_mut(&id).unwrap() {
    //                                     c[i] = *child3_id;
    //                                 }
    //                             }
    //                             And(_) => todo!(),
    //                             Or(_) => todo!(),
    //                         }
    //                     }
    //                     And(_) => todo!(),
    //                     Or(_) => todo!(),
    //                 }
    //             }
    //         }
    //     }
    // }
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
    let a_and_c = f.add_expr(And(vec![a, c]));
    let b_or_c = f.add_expr(Or(vec![b, c]));
    let not_b_or_c = f.add_expr(Not(b_or_c));
    let root = f.add_expr(Or(vec![a_and_c, not_b_or_c, b_or_c]));
    f.set_root(root);
    println!("{f}");
}

// tseitin formula also has a 'pointer' to another formula, to ease the actual substitution
// add optimizations for simplification? (e.g., idempotency)
// randomize clause order? (scrambler?)
// during parsing, when the hash of a particular subformula has already been mapped to a usize (already included in the formula), reuse that usize
// possibly, we need a HashMap<Expr, usize> during parsing to ensure structural sharing

// #[derive(Debug)]
// struct CNF {
//     variables: HashMap<Id, String>, // is this sorted?
//     clauses: Vec<Vec<i32>>,
// }
