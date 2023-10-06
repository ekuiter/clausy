//! Clause representation of a feature-model formula.

use std::{fmt, slice};

use crate::{
    core::formula::{Expr::*, Formula, Id, Var, VarId},
    util::exec,
};

/// A [Formula] in its clause representation.
///
/// That is, this data structure enforces a conjunctive normal form.
pub(crate) struct Clauses<'a> {
    /// The clauses of this clause representation.
    ///
    /// A clause is a [Vec] of literals, each given as an absolute-value index into [Clauses::vars].
    /// Negative values indicate negated variable occurrences.
    clauses: Vec<Vec<VarId>>,

    /// The variables of this clause representation.
    ///
    /// This list is indexed into by the absolute values stored in [Clauses::clauses].
    vars: Vec<Var<'a>>,
}

/// Algorithms for representing a [Formula] as [Clauses].
impl<'a> Clauses<'a> {
    /// Returns the sub-expressions of a formula as clauses.
    ///
    /// We require that the formula already is in conjunctive normal form (see [Formula::to_cnf_dist]).
    fn clauses(formula: &Formula) -> Vec<Vec<VarId>> {
        let mut clauses = Vec::<Vec<VarId>>::new();

        let add_literal = |id, clause: &mut Vec<VarId>| match formula.exprs[id] {
            Var(var_id) => clause.push(var_id),
            Not(child_id) => match formula.exprs[child_id] {
                Var(var_id) => clause.push(-var_id),
                _ => unreachable!(),
            },
            _ => unreachable!(),
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
                        _ => unreachable!(),
                    }
                }
            }
        }

        clauses
    }

    /// Panics if this clause representation is invalid.
    ///
    /// A clause representation is valid if it has at least one variable and one clause.
    #[cfg(debug_assertions)]
    pub fn assert_valid(&self) {
        debug_assert!(self.vars.len() > 0 && self.clauses.len() > 0);
    }

    /// Attempts to finds a solution of this clause representation.
    pub(crate) fn satisfy(&self) -> Option<String> {
        exec::kissat(&self.to_string()).map(|solution| {
            solution
                .iter()
                .map(|literal| {
                    let idx: usize = literal.unsigned_abs().try_into().unwrap();
                    format!(
                        "{}{}",
                        if *literal > 0 { "+" } else { "-" },
                        self.vars[idx].to_string()
                    )
                })
                .collect::<Vec<String>>()
                .join("\n")
        })
    }

    /// Counts the number of solutions of this clause representation.
    pub(crate) fn count(&self) -> String {
        exec::d4(&self.to_string())
    }

    /// Counts the number of solutions of a feature-model file using FeatureIDE.
    ///
    /// The file extension must be given so FeatureIDE can detect the correct format.
    fn count_featureide(file: &str, extension: String) -> String {
        exec::d4(&exec::io(file, &extension, "dimacs"))
    }

    /// Panics if this clause representation has a different model count than that of FeatureIDE.
    ///
    /// Useful for checking the correctness of count-preserving algorithms (e.g., [Formula::to_cnf_tseitin]).
    pub(crate) fn assert_count(&self, file: &str, extension: String) {
        assert_eq!(self.count(), Self::count_featureide(file, extension));
    }
}

impl<'a> From<&Formula<'a>> for Clauses<'a> {
    fn from(formula: &Formula<'a>) -> Self {
        Self {
            clauses: Self::clauses(&formula),
            vars: formula.vars.clone(),
        }
    }
}

impl<'a> fmt::Display for Clauses<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, var) in self.vars.iter().enumerate() {
            if i == 0 {
                continue;
            }
            if let Var::Named(name) = var {
                debug_assert!(!name.is_empty());
            }
            write!(f, "c {i} {var}\n")?;
        }
        write!(f, "p cnf {} {}\n", self.vars.len() - 1, self.clauses.len())?;
        for clause in &self.clauses {
            debug_assert_ne!(clause.len(), 0);
            for literal in clause {
                debug_assert_ne!(*literal, 0);
                let var: usize = literal.unsigned_abs().try_into().unwrap();
                debug_assert!(var < self.vars.len());
                write!(f, "{literal} ")?;
            }
            write!(f, "0\n")?;
        }
        write!(f, "")
    }
}
