use cnfrust::formula::{Formula,Expr::*};

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
    let b_or_c = f.add_expr(Or(vec![not_b, not_not_c, a_and_c]));
    let not_b_or_c = f.add_expr(Not(b_or_c));
    let not_not_b_or_c = f.add_expr(Not(not_b_or_c));
    let root = f.add_expr(Or(vec![a_and_c, not_b_or_c, not_not_b_or_c]));
    f.set_root_expr(root);
    println!("{f}");
    f = f.to_nnf();
    println!("{f}");
    f = f.to_cnf_dist();
    println!("{f}");
}