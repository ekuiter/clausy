use std::{collections::HashMap, fmt};

use crate::formula::{Formula, VarId};

pub struct CNF<'a> {
    clauses: Vec<Vec<VarId>>,
    vars: HashMap<VarId, &'a str>,
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
        assert!(!self.vars.contains_key(&0) && self.vars.len() > 0 && self.clauses.len() > 0, "CNF is invalid");
    }
}

impl<'a> fmt::Display for CNF<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.assert_valid();
        for (i, var) in &self.vars {
            assert!(!var.is_empty(), "variable {i} has empty name");
            write!(f, "c {i} {var}\n")?; // order of variables?
        }
        write!(f, "p cnf {} {}\n", self.vars.len(), self.clauses.len())?;
        for clause in &self.clauses {
            assert_ne!(clause.len(), 0, "empty clause is not allowed");
            for literal in clause {
                assert_ne!(*literal, 0, "literal 0 is not allowed");
                assert!(self.vars.contains_key(&literal.abs()), "variable {} not found", literal.abs());
                write!(f, "{literal} ")?;
            }
            write!(f, "0\n")?;
        }
        write!(f, "")
    }
}
