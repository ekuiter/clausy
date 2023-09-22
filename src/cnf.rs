//! Clause representation of a feature-model formula.

use std::{fmt, fs, process::{Command, Stdio}, slice, io::{Write, Read}};

use crate::formula::{Expr::*, ExprInFormula, Formula, Id, Var, VarId};

pub struct CNF<'a> {
    clauses: Vec<Vec<VarId>>,
    vars: Vec<Var<'a>>,
}

impl<'a> CNF<'a> {
    /// Returns the sub-expressions of a formula as clauses.
    ///
    /// We require that the formula is in conjunctive normal form (see [Formula::to_cnf_dist]).
    /// Clauses are represented as [Vec]s of literals, which are (possibly negative) variable identifiers.
    pub(crate) fn get_clauses(formula: &Formula) -> Vec<Vec<VarId>> {
        let mut clauses = Vec::<Vec<VarId>>::new();

        let add_literal = |id, clause: &mut Vec<VarId>| match formula.exprs[id] {
            Var(var_id) => clause.push(var_id),
            Not(child_id) => {
                if let Var(var_id) = formula.exprs[child_id] {
                    clause.push(-var_id);
                } else {
                    panic!(
                        "expected Var below Not, got {}",
                        ExprInFormula(formula, &id)
                    );
                }
            }
            _ => panic!(
                "expected Var or Not literal, got {}",
                ExprInFormula(formula, &id)
            ),
        };

        let mut add_clause = |child_ids: &[Id]| {
            let mut clause = Vec::<VarId>::new();
            for child_id in child_ids {
                add_literal(*child_id, &mut clause);
            }
            clauses.push(clause);
        };

        match &formula.exprs[formula.get_root_expr()] {
            Var(_) | Not(_) => add_clause(slice::from_ref(&formula.get_root_expr())),
            Or(child_ids) => add_clause(child_ids),
            And(child_ids) => {
                for child_id in child_ids {
                    match &formula.exprs[*child_id] {
                        Var(_) | Not(_) => add_clause(slice::from_ref(child_id)),
                        Or(child_ids) => add_clause(&child_ids),
                        _ => panic!(
                            "expected Var, Not, or Or expression, got {}",
                            ExprInFormula(formula, child_id)
                        ),
                    }
                }
            }
        }

        clauses
    }

    fn assert_valid(&self) {
        assert!(
            self.vars.len() > 0 && self.clauses.len() > 0,
            "CNF is invalid"
        );
    }

    // requires d4 and write access to .
    fn count_d4(dimacs: &str) -> String {
        fs::write("tmp.dimacs", dimacs).expect("could not write temporary DIMACS file");
        let output = Command::new("./d4")
            .arg("-i")
            .arg("tmp.dimacs")
            .arg("-m")
            .arg("counting")
            .arg("-p")
            .arg("sharp-equiv")
            .output()
            .unwrap();
        fs::remove_file("tmp.dimacs").expect("could not remove temporary DIMACS file");
        String::from(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .find(|line| line.starts_with("s "))
                .unwrap()
                .split_at(2)
                .1,
        )
    }

    // requires java + io.jar
    fn dimacs_featureide(model: &str) -> String {
        let process = Command::new("java")
            .arg("-jar")
            .arg("io.jar")
            .arg("-.model")
            .arg("dimacs")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        process.stdin.unwrap().write_all(model.as_bytes()).unwrap();
        let mut dimacs = String::new();
        process.stdout.unwrap().read_to_string(&mut dimacs).unwrap();
        dimacs
    }

    pub fn count(&self) -> String {
        Self::count_d4(&self.to_string())
    }

    pub fn count_featureide(model: &str) -> String {
        Self::count_d4(&Self::dimacs_featureide(model))
    }

    pub fn assert_count(&self, model: &str) {
        assert_eq!(self.count(), Self::count_featureide(model));
    }
}

impl<'a> From<Formula<'a>> for CNF<'a> {
    fn from(formula: Formula<'a>) -> Self {
        Self {
            clauses: Self::get_clauses(&formula),
            vars: formula.vars,
        }
    }
}

impl<'a> fmt::Display for CNF<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.assert_valid();
        for (i, var) in self.vars.iter().enumerate() {
            if i == 0 {
                continue;
            }
            if let Var::Named(name) = var {
                assert!(!name.is_empty(), "variable {i} has empty name");
            }
            write!(f, "c {i} {var}\n")?;
        }
        write!(f, "p cnf {} {}\n", self.vars.len() - 1, self.clauses.len())?;
        for clause in &self.clauses {
            assert_ne!(clause.len(), 0, "empty clause is not allowed");
            for literal in clause {
                assert_ne!(*literal, 0, "literal 0 is not allowed");
                let var: usize = literal.unsigned_abs().try_into().unwrap();
                assert!(
                    var < self.vars.len(),
                    "variable {} not found",
                    literal.abs()
                );
                write!(f, "{literal} ")?;
            }
            write!(f, "0\n")?;
        }
        write!(f, "")
    }
}
