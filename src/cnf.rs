use std::fmt;

use crate::formula::{Formula, VarId};

pub struct CNF<'a> {
    clauses: Vec<Vec<VarId>>,
    vars: Vec<&'a str>,
}

impl<'a> From<&'a Formula<'a>> for CNF<'a> {
    fn from(formula: &'a Formula<'a>) -> Self {
        Self {
            clauses: formula.get_clauses(),
            vars: formula.get_vars(),
        }
    }
}

impl<'a> CNF<'a> {
    fn assert_valid(&self) {
        assert!(self.vars.len() > 0 && self.clauses.len() > 0, "CNF is invalid");
    }
}

impl<'a> fmt::Display for CNF<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.assert_valid();
        for (i, var) in self.vars.iter().enumerate() {
            if i == 0 {
                continue;
            }
            assert!(!var.is_empty(), "variable {i} has empty name");
            write!(f, "c {i} {var}\n")?;
        }
        write!(f, "p cnf {} {}\n", self.vars.len() - 1, self.clauses.len())?;
        for clause in &self.clauses {
            assert_ne!(clause.len(), 0, "empty clause is not allowed");
            for literal in clause {
                assert_ne!(*literal, 0, "literal 0 is not allowed");
                let var: usize = literal.unsigned_abs().try_into().unwrap();
                assert!(var < self.vars.len(), "variable {} not found", literal.abs());
                write!(f, "{literal} ")?;
            }
            write!(f, "0\n")?;
        }
        write!(f, "")
    }
}
