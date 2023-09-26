//! Clause representation of a feature-model formula.

use std::{fmt, slice};

use crate::{core::formula::{Expr::*, ExprInFormula, Formula, Id, Var, VarId}, util::exec};

/// A [Formula] in its clause representation.
/// 
/// That is, this data structure enforces a conjunctive normal form.
pub(crate) struct CNF<'a> {
    /// The clauses of this CNF.
    /// 
    /// A clause is a [Vec] of literals, each given as an absolute-value index into [CNF::vars].
    /// Negative values indicate negated variable occurrences.
    clauses: Vec<Vec<VarId>>,

    /// The variables of this CNF.
    /// 
    /// This list is indexed into by the absolute values stored in [CNF::clauses].
    vars: Vec<Var<'a>>,
}

/// Algorithms for representing a [Formula] as a [CNF].
impl<'a> CNF<'a> {
    /// Returns the sub-expressions of a formula as clauses.
    ///
    /// We require that the formula already is in conjunctive normal form (see [Formula::to_cnf_dist]).
    fn get_clauses(formula: &Formula) -> Vec<Vec<VarId>> {
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

    /// Panics if this CNF is invalid.
    ///
    /// A CNF is valid if it has at least one variable and one clause.
    fn assert_valid(&self) {
        assert!(
            self.vars.len() > 0 && self.clauses.len() > 0,
            "CNF is invalid"
        );
    }

    /// Counts the number of satisfying assignments of this CNF.
    fn count(&self) -> String {
        exec::d4(&self.to_string())
    }

    fn count_featureide(model: &str) -> String {
        exec::d4(&exec::io(model, "model", "dimacs"))
    }

    pub(crate) fn assert_count(&self, model: &str) {
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
            write!(f, "c {i} {var}\n")?; // to save space, do not print aux variables? or pass an option for that (with configurable prefix?)
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
