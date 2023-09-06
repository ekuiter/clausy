use indextree::*;
use std::fmt;
use ExprKind::*;

// this probably creates copies of identical expressions
// tseitin formula also has a 'pointer' to another formula, to ease the actual substitution
// // add optimizations for simplification?
// // randomize clause order? (scrambler?)
// struct CNF {
//     variables: HashMap<i32, String>, // is this sorted?
//     clauses: Vec<Vec<i32>>,
// }

#[derive(Debug)]
enum ExprKind {
    Var(String),
    Not,
    And,
    Or,
}

struct Expr<'a>(&'a NodeId, &'a Arena<ExprKind>);

fn node<'a>(node_id: &NodeId, arena: &'a Arena<ExprKind>) -> &'a Node<ExprKind> {
    arena.get(*node_id).expect("failed to retrieve node")
}

impl<'a> fmt::Display for Expr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let node_id = self.0;
        let arena = self.1;
        let expr_kind = node(node_id, arena).get();
        match expr_kind {
            Var(name) => {
                write!(f, "{name}")
            }
            _ => {
                write!(
                    f,
                    "{:?}({})",
                    expr_kind,
                    node_id
                        .children(arena)
                        .map(|child_id| Expr(&child_id, arena).to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

fn main() {
    // use Expr::*;
    // let expr = Or(
    //     Box::new(And(Box::new(Var(1)), Box::new(Not(Box::new(Var(2)))))),
    //     Box::new(And(Box::new(Var(3)), Box::new(Var(1))))
    // );
    // dbg!(CnfVec::from(dbg!(expr)));

    let arena = &mut Arena::new();
    let root = arena.new_node(Or);
    let a = arena.new_node(Var(String::from("a")));
    let b = arena.new_node(Var(String::from("b")));
    let c = arena.new_node(Var(String::from("c")));
    let b_or_c = arena.new_node(Or);
    let not_b_or_c = arena.new_node(Not);
    root.append(a, arena);
    root.append(not_b_or_c, arena);
    root.append(b_or_c, arena);
    b_or_c.append(b, arena);
    b_or_c.append(c, arena);
    not_b_or_c.append(b_or_c, arena);
    println!("{}", Expr(&root, arena));
}
