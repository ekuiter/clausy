//! Unit tests.

#![allow(unused_imports)]
use crate::core::{cnf::Cnf, formula::{Expr::*, Formula}};

mod formula {
    use super::*;

    mod valid {
        use super::*;

        #[test]
        #[should_panic(expected = "formula is invalid")]
        fn empty() {
            Formula::new().assert_valid();
        }

        #[test]
        #[should_panic(expected = "formula is invalid")]
        fn no_root() {
            let mut f = Formula::new();
            let a = f.var("a");
            f.expr(Not(a));
            f.assert_valid();
        }

        #[test]
        fn valid() {
            let mut f = Formula::new();
            let a = f.var("a");
            let not_a = f.expr(Not(a));
            f.set_root_expr(not_a);
            f.assert_valid();
        }
    }

    mod nnf {
        use super::*;

        #[test]
        fn not_a() {
            let mut f = Formula::new();
            let a = f.var("a");
            let not_a = f.expr(Not(a));
            f.set_root_expr(not_a);
            assert_eq!(
                f.assert_valid().to_nnf().assert_valid().to_string(),
                "Not(a)"
            );
        }

        #[test]
        fn not_not_a() {
            let mut f = Formula::new();
            let a = f.var("a");
            let not_a = f.expr(Not(a));
            let not_not_a = f.expr(Not(not_a));
            f.set_root_expr(not_not_a);
            assert_eq!(f.assert_valid().to_nnf().assert_valid().to_string(), "a");
        }

        #[test]
        fn and_not_not_a() {
            let mut f = Formula::new();
            let a = f.var("a");
            let not_a = f.expr(Not(a));
            let not_not_a = f.expr(Not(not_a));
            let and = f.expr(And(vec![not_not_a]));
            f.set_root_expr(and);
            assert_eq!(
                f.assert_valid().to_nnf().assert_valid().to_string(),
                "And(a)"
            );
        }

        #[test]
        fn complex() {
            let mut f = Formula::new();
            let a = f.var("a");
            let b = f.var("b");
            let c = f.var("c");
            let not_a = f.expr(Not(a));
            let not_b = f.expr(Not(b));
            let not_c = f.expr(Not(c));
            let not_not_c = f.expr(Not(not_c));
            let not_a_and_c = f.expr(And(vec![not_a, c]));
            let not_b_or_not_not_c_or_not_a_and_c = f.expr(Or(vec![not_b, not_not_c, not_a_and_c]));
            let not_not_b_or_not_not_c_or_not_a_and_c =
                f.expr(Not(not_b_or_not_not_c_or_not_a_and_c));
            let not_not_not_b_or_not_not_c_or_not_a_and_c =
                f.expr(Not(not_not_b_or_not_not_c_or_not_a_and_c));
            let root = f.expr(Or(vec![
                not_a_and_c,
                not_not_b_or_not_not_c_or_not_a_and_c,
                not_not_not_b_or_not_not_c_or_not_a_and_c,
            ]));
            f.set_root_expr(root);
            assert_eq!(
                f.assert_valid().to_nnf().assert_valid().to_string(),
                "Or(And(Not(a), c), And(b, Not(c), Or(a, Not(c))), Or(Not(b), c, And(Not(a), c)))"
            );
        }

        #[test]
        fn idempotent() {
            let f = Formula::from(
                "(((!def(a)))&(((def(c)|!def(a)))|((def(a))&(def(c)|!(def(a)|def(b))))))",
            )
            .assert_valid()
            .to_nnf()
            .assert_valid();
            let s = f.to_string();
            assert_eq!(s, f.assert_valid().to_nnf().assert_valid().to_string());
        }

        #[test]
        fn shared() {
            Formula::from("((((!(def(a))&def(a)))&(!(!(def(a))&def(a))))&((!(!(def(a))&def(a)))&(!((def(a))&def(a)))))")
            .assert_valid()
            .to_nnf()
            .assert_valid();
        }
    }

    mod cnf_dist {
        use super::*;

        #[test]
        fn simple() {
            let mut f = Formula::new();
            let a = f.var("a");
            let b = f.var("b");
            let a_and_b = f.expr(And(vec![a, b]));
            let a_or_a_and_b = f.expr(Or(vec![a, a_and_b]));
            let a_and_a_or_a_and_b = f.expr(And(vec![a, a_or_a_and_b]));
            f.set_root_expr(a_and_a_or_a_and_b);
            f = f
                .assert_valid()
                .to_nnf()
                .assert_valid()
                .to_cnf_dist()
                .assert_valid();
            assert_eq!(f.to_string(), "And(a, Or(a, b))");
        }

        #[test]
        fn complex() {
            let mut f = Formula::new();
            let a = f.var("a");
            let b = f.var("b");
            let c = f.var("c");
            let not_a = f.expr(Not(a));
            let not_b = f.expr(Not(b));
            let not_c = f.expr(Not(c));
            let not_not_c = f.expr(Not(not_c));
            let a_and_c = f.expr(And(vec![not_a, c]));
            let b_or_c = f.expr(Or(vec![not_b, not_not_c, a_and_c]));
            let not_b_or_c = f.expr(Not(b_or_c));
            let not_not_b_or_c = f.expr(Not(not_b_or_c));
            let root = f.expr(Or(vec![a_and_c, not_b_or_c, not_not_b_or_c]));
            f.set_root_expr(root);
            f = f
                .assert_valid()
                .to_nnf()
                .assert_valid()
                .to_cnf_dist()
                .assert_valid();
            assert_eq!(
                f.to_string(),
                "And(Or(b, c, Not(b)), Or(b, c, Not(a), Not(b)), \
            Or(c, Not(b), Not(c)), Or(c, Not(a), Not(b), Not(c)), Or(a, c, Not(b), Not(c)), \
            Or(a, c, Not(a), Not(b), Not(c)), Or(b, c, Not(a), Not(b)), Or(b, c, Not(a), \
            Not(b)), Or(c, Not(a), Not(b), Not(c)), Or(c, Not(a), Not(b), Not(c)), \
            Or(a, c, Not(a), Not(b), Not(c)), Or(a, c, Not(a), Not(b), Not(c)))"
            );
        }

        #[test]
        fn shared_expr() {
            // todo: run this on larger formulas as well
            let model = "((def(a)|!def(a))&(def(a)|!(def(a)|def(a))))";
            let f = Formula::from(model)
                .assert_valid()
                .to_nnf()
                .assert_valid()
                .to_cnf_dist()
                .assert_valid();
            Cnf::from(f).assert_count(&model);
            let model = "(((!def(a)))&(((def(c)|!def(a)))|((def(a))&(def(c)|!(def(a)|def(b))))))";
            let f = Formula::from(model)
                .assert_valid()
                .to_nnf()
                .assert_valid()
                .to_cnf_dist()
                .assert_valid();
            Cnf::from(f).assert_count(&model);
        }

        #[test]
        fn idempotent() {
            let model = "(((!def(a)))&(((def(c)|!def(a)))|((def(a))&(def(c)|!(def(a)|def(b))))))";
            let f = Formula::from(model)
                .assert_valid()
                .to_nnf()
                .assert_valid()
                .to_cnf_dist()
                .assert_valid();
            let s = f.to_string();
            let f = f.assert_valid().to_cnf_dist().assert_valid();
            assert_eq!(s, f.to_string());
            Cnf::from(f).assert_count(&model);
        }

        #[test]
        fn shared() {
            let model = "((((!(def(a))&def(a)))&(!(!(def(a))&def(a))))&((!(!(def(a))&def(a)))&(!((def(a))&def(a)))))";
            let f = Formula::from(model)
                .assert_valid()
                .to_nnf()
                .assert_valid()
                .to_cnf_dist()
                .assert_valid();
            Cnf::from(f).assert_count(&model);
        }
    }
}

mod cnf {
    use super::*;

    #[test]
    fn simple() {
        let model = "((def(x)|def(y))&def(ab)&!def(n)&(def(abc)&!(def(x)|def(y))&def(bb)))";
        let f = Formula::from(model)
            .assert_valid()
            .to_nnf()
            .assert_valid()
            .to_cnf_dist()
            .assert_valid();
        let cnf = Cnf::from(f);
        assert_eq!(cnf.to_string().lines().count(), 14);
        cnf.assert_count(&model);
    }
}

mod parser {
    use super::*;

    #[test]
    fn simple() {
        let model = "# comment
            (def(x)|def(y))
            # coaoeu
            def( ab )
            !def(n)
            (def( abc)& !(def(x)|def(y))   & def( bb ))";
        let f = Formula::from(model).assert_valid();
        assert_eq!(
            f.to_string(),
            "And(Or(x, y), ab, Not(n), And(abc, Not(Or(x, y)), bb))"
        );
    }
}
