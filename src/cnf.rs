use std::{collections::HashMap, fmt};

use crate::formula::Formula;

pub struct CNF {
    vars: HashMap<u32, String>,
    clauses: Vec<Vec<i32>>,
}

impl CNF {
    pub fn new(_formula: Formula) -> Self {
        let mut vars = HashMap::new();
        vars.insert(1, "bla".to_string());
        vars.insert(2, "test".to_string());
        Self {
            vars,
            clauses: vec![vec![1, -2], vec![-1], vec![2]],
        }
    }

    fn assert_valid(&self) {
        assert!(!self.vars.contains_key(&0) && self.vars.len() > 0 && self.clauses.len() > 0, "CNF is invalid");
    }
}

impl fmt::Display for CNF {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.assert_valid();
        write!(f, "p cnf {} {}\n", self.vars.len(), self.clauses.len())?;
        for (i, var) in &self.vars {
            assert!(!var.is_empty(), "variable {i} has empty name");
            write!(f, "c {i} {var}\n")?; // order of variables?
        }
        for clause in &self.clauses {
            assert_ne!(clause.len(), 0, "empty clause is not allowed");
            for literal in clause {
                assert_ne!(*literal, 0, "literal 0 is not allowed");
                assert!(self.vars.contains_key(&literal.unsigned_abs()), "variable {} not found", literal.abs());
                write!(f, "{literal} ")?;
            }
            write!(f, "0\n")?;
        }
        write!(f, "")
    }
}
