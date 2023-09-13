use crate::formula::{Expr::*, Formula};

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
    #[ignore] // todo
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
        let a_and_c = f.add_expr(And(vec![not_a, c]));
        let b_or_c = f.add_expr(Or(vec![not_b, not_not_c, a_and_c]));
        let not_b_or_c = f.add_expr(Not(b_or_c));
        let not_not_b_or_c = f.add_expr(Not(not_b_or_c));
        let root = f.add_expr(Or(vec![a_and_c, not_b_or_c, not_not_b_or_c]));
        f.set_root_expr(root);
        assert_eq!(
            f.to_nnf().to_string(),
            "Or(And(Not(a), c), And(b, Not(c), Or(a, Not(c))), Or(Not(b), c, And(Not(a), c)))"
        );
    }
}

mod cnf_dist {}
