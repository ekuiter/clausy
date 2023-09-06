// use std::collections::{HashMap, HashSet};

// // struct Formula {
// //     variables: HashSet<String>,
// //     sub_expressions: HashSet<Expression>,
// //     root_expression: Expression
// // }

// // this probably creates copies of identical expressions
// #[derive(Debug)]
// enum Formula {
//     Var(String),
//     Not(Box<Formula>),
//     And(Vec<Formula>),
//     Or(Vec<Formula>),
//     // tseitin formula also has a 'pointer' to another formula, to ease the actual substitution
// }

// impl Formula {
//     fn to_nnf(&mut self) {
//         match self {
//             Formula::Var(_) => (),
//             Formula::Not(formula) => {
//                 match **formula {
//                     Formula::Var(_) => (),
//                     Formula::Not(inner_formula) => {
//                         self = *inner_formula;
//                     }
//                     Formula::And(_) => {
//                         //self = Formula::Or(vec![]);
//                     }
//                     Formula::Or(_) => {}
//                 }
//             }
//             Formula::And(formulas) => {
//                 for formula in formulas {
//                     formula.to_nnf();
//                 }
//             }
//             Formula::Or(formulas) => {
//                 for formula in formulas {
//                     formula.to_nnf();
//                 }
//             }
//         }
//     }

//     fn to_cnf_distributive(self) {}
// }

// struct CNF {
//     variables: HashMap<i32, String>, // is this sorted?
//     clauses: Vec<Vec<i32>>,
// }

// // add featjar optimizations for simplification?
// // randomize clause order? (scrambler?)

// fn main() {
//     let formula = Formula::And(vec![
//         Formula::Or(vec![
//             Formula::Var("x".to_owned()),
//             Formula::Var("y".to_owned()),
//         ]),
//         Formula::Var("v".to_owned()),
//     ]);

//     formula.to_nnf();

//     println!("{:#?}", formula);
// }

// https://docs.rs/simple_predicates/latest/simple_predicates/ (MIT/Apache)

#[derive(Debug, Clone, Eq, Hash)]
enum Expr {
    Var(i32), // string?
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

impl Expr {
    fn pushdown_not(self) -> Self {
        use Expr::*;
        if let Not(expr) = self {
            match *expr {
                Var(p) => Not(Box::new(Var(p))),
                Not(p) => p.pushdown_not(),
                Or(a, b) => And(Box::new(Not(a)), Box::new(Not(b))),
                And(a, b) => Or(Box::new(Not(a)), Box::new(Not(b))),
            }
        } else {
            self
        }
    }
}

impl Expr {
    fn simplify(self) -> Self {
        use Expr::*;

        match self {
            Not(p) => match *p {
                Not(q) => q.simplify(),
                q => Not(Box::new(q.simplify())),
            },
            And(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                if a == b {
                    a
                } else {
                    And(Box::new(a), Box::new(b))
                }
            }
            Or(a, b) => {
                let a = a.simplify();
                let b = b.simplify();
                if a == b {
                    a
                } else {
                    Or(Box::new(a), Box::new(b))
                }
            }
            _ => self,
        }
    }
}

impl Expr
{
    fn distribute_or(self) -> Self {
        use Expr::*;
        if let Or(a, b) = self {
            match (*a, *b) {
                (p, And(q, r)) | (And(q, r), p) => And(
                    Box::new(Or(Box::new(p.clone()), q)),
                    Box::new(Or(Box::new(p), r)),
                ),
                (a, b) => Or(Box::new(a), Box::new(b)),
            }
        } else {
            self
        }
    }
}

impl PartialEq for Expr
{
    fn eq(&self, other: &Self) -> bool {
        use Expr::*;

        match (self, other) {
            (Var(p1), Var(p2)) => p1 == p2,
            (Not(p1), Not(p2)) => p1 == p2,
            (Or(a1, b1), Or(a2, b2)) => (a1 == a2 && b1 == b2) || (a1 == b2 && b1 == a2),
            (And(a1, b1), And(a2, b2)) => (a1 == a2 && b1 == b2) || (a1 == b2 && b1 == a2),
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
struct CnfVec(Vec<Expr>);

impl From<Expr> for CnfVec
{
    fn from(expr: Expr) -> Self {
        use Expr::*;
        let mut clauses = Vec::new();
        let mut queue = Vec::with_capacity(2);
        queue.push(expr.simplify());

        while let Some(expr) = queue.pop() {
            match expr.pushdown_not().distribute_or() {
                And(a, b) => {
                    queue.push(*a);
                    queue.push(*b);
                }
                other => {
                    let _ = clauses.push(other);
                }
            }
        }
        CnfVec(clauses)
    }
}

fn main() {
    use Expr::*;
    let expr = Or(
        Box::new(And(Box::new(Var(1)), Box::new(Not(Box::new(Var(2)))))),
        Box::new(And(Box::new(Var(3)), Box::new(Var(1))))
    );
    dbg!(CnfVec::from(dbg!(expr)));
}
