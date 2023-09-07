use std::{collections::HashMap, fmt, slice};
use Expr::*;

// tseitin formula also has a 'pointer' to another formula, to ease the actual substitution
// // add optimizations for simplification?
// // randomize clause order? (scrambler?)

type VarId = u32;
type ExprId = u32;

#[derive(Debug)]
struct Formula {
    root_expr_id: ExprId,
    next_expr_id: ExprId,
    next_var_id: VarId,
    exprs: HashMap<ExprId, Expr>,
    vars: HashMap<VarId, String>,
}

#[derive(Debug)]
struct CNF {
    variables: HashMap<VarId, String>, // is this sorted?
    clauses: Vec<Vec<i32>>,
}

#[derive(Debug)]
enum Expr {
    Var(VarId),
    Not(ExprId),
    And(Vec<ExprId>),
    Or(Vec<ExprId>),
}

impl Formula {
    fn new() -> Formula {
        Formula {
            root_expr_id: 0,
            next_expr_id: 0,
            next_var_id: 0,
            exprs: HashMap::new(),
            vars: HashMap::new(),
        }
    }

    fn expr(&self, expr_id: ExprId) -> &Expr {
        self.exprs
            .get(&expr_id)
            .expect(&format!("could not retrieve expr with ID {}", expr_id))
    }

    fn expr_mut(&mut self, expr_id: ExprId) -> &mut Expr {
        self.exprs
            .get_mut(&expr_id)
            .expect(&format!("could not retrieve expr with ID {}", expr_id))
    }

    fn var(&self, var_id: VarId) -> &String {
        self.vars
            .get(&var_id)
            .expect(&format!("could not retrieve var with ID {}", var_id))
    }

    fn children_expr_ids(&self, expr_id: ExprId) -> &[ExprId] {
        match self.expr(expr_id) {
            Var(_) => &[],
            Not(expr_id) => slice::from_ref(expr_id),
            And(expr_ids) => &expr_ids[..],
            Or(expr_ids) => &expr_ids[..],
        }
    }

    fn set_root(&mut self, expr_id: ExprId) {
        self.root_expr_id = expr_id;
    }

    fn add_expr(&mut self, expr: Expr) -> u32 {
        let next_expr_id = self.next_expr_id + 1;
        self.exprs.insert(next_expr_id, expr);
        self.next_expr_id += 1;
        next_expr_id
    }

    fn add_child(&mut self, expr_id: ExprId, child: Expr) -> u32 {
        let next_expr_id = self.next_expr_id + 1;
        match self.expr_mut(expr_id) {
            Var(_) => panic!("can not add a child to a Var expr"),
            Not(expr_id) => *expr_id = next_expr_id,
            And(expr_ids) => expr_ids.push(next_expr_id),
            Or(expr_ids) => expr_ids.push(next_expr_id),
        }
        self.exprs.insert(next_expr_id, child);
        self.next_expr_id += 1;
        next_expr_id
    }

    fn add_var(&mut self, var: &str) -> u32 {
        let next_var_id = self.next_var_id + 1;
        self.vars.insert(next_var_id, String::from(var));
        self.next_var_id += 1;
        self.add_expr(Var(next_var_id))
    }

    fn display(&self, expr_id: ExprId, f: &mut fmt::Formatter) {
        let mut write = |kind: &str, expr_ids: &[u32]| {
            write!(f, "{kind}(").ok();
            let mut i = 0;
            for expr_id in expr_ids {
                if i > 0 {
                    write!(f, ", ").ok();
                }
                i += 1;
                self.display(*expr_id, f);
            }
            write!(f, ")").ok();
        };
        match self.expr(expr_id) {
            Var(var_id) => {
                write!(f, "{}", self.var(*var_id)).ok();
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
        };
    }
}

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display(self.root_expr_id, f);
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
