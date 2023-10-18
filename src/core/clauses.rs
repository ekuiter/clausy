//! Clause representation of a feature-model formula.

use std::{fmt, slice, collections::HashMap};

use crate::{
    core::{expr::{Expr::*, ExprId}, var::{Var, VarId}},
    util::exec,
};

use super::formula::FormulaContext;

/// A [Formula] in its clause representation.
///
/// That is, this data structure enforces a conjunctive normal form.
pub(crate) struct Clauses {
    /// The clauses of this clause representation.
    ///
    /// A clause is a [Vec] of literals, each given as an absolute-value index into [Clauses::vars].
    /// Negative values indicate negated variable occurrences.
    clauses: Vec<Vec<VarId>>,

    /// The variables of this clause representation.
    ///
    /// This list is indexed into by the absolute values stored in [Clauses::clauses].
    vars: Vec<Var>,

    var_remap: HashMap<VarId, VarId>,
}

/// Algorithms for representing a [Formula] as [Clauses].
impl Clauses {
    /// Returns the sub-expressions of a formula as clauses.
    ///
    /// We require that the formula already is in conjunctive normal form (see [Formula::to_cnf_dist]).
    fn clauses(formula: &FormulaContext, var_remap: &HashMap<VarId, VarId>) -> Vec<Vec<VarId>> {
        let mut clauses = Vec::<Vec<VarId>>::new();

        let add_literal = |id, clause: &mut Vec<VarId>| match formula.arena.exprs[id] {
            Var(var_id) => clause.push(var_remap[&var_id]),
            Not(child_id) => match formula.arena.exprs[child_id] {
                Var(var_id) => clause.push(-var_remap[&var_id]),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        };

        let mut add_clause = |child_ids: &[ExprId]| {
            let mut clause = Vec::<VarId>::new();
            for child_id in child_ids {
                add_literal(*child_id, &mut clause);
            }
            clauses.push(clause);
        };

        match &formula.arena.exprs[formula.formula.get_root_expr()] {
            Var(_) | Not(_) => add_clause(slice::from_ref(&formula.formula.get_root_expr())),
            Or(child_ids) => add_clause(child_ids),
            And(child_ids) => {
                for child_id in child_ids {
                    match &formula.arena.exprs[*child_id] {
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
    /// A clause representation is valid if it has at least one variable.
    /// If there is no clause, the represented formula is a tautology.
    /// If there is an empty clause, the represented formula is a contradiction.
    #[cfg(debug_assertions)]
    pub fn assert_valid(&self) {
        debug_assert!(self.vars.len() > 0);
    }

    /// Returns a solution as a human-readable string.
    fn solution_to_string(&self, solution: &Vec<VarId>) -> String {
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
            .join(" ")
    }

    /// Attempts to finds a solution of this clause representation.
    pub(crate) fn satisfy(&self) -> Option<String> {
        exec::kissat(&self.to_string()).map(|solution| self.solution_to_string(&solution))
    }

    /// Enumerates all solutions of this clause representation.
    ///
    /// Prints solutions to standard output as soon as they are known, instead of returning them.
    pub(crate) fn enumerate(&self) {
        let (iter, tmp_in) = exec::bc_minisat_all(&self.to_string());
        iter.for_each(|solution| println!("{}", self.solution_to_string(&solution)));
        drop(tmp_in);
    }

    /// Counts the number of solutions of this clause representation.
    pub(crate) fn count(&self) -> String {
        exec::d4(&self.to_string())
    }

    /// Counts the number of solutions of a feature-model file using FeatureIDE.
    ///
    /// The file extension must be given so FeatureIDE can detect the correct format.
    fn count_featureide(file: &str, extension: &str) -> String {
        exec::d4(&exec::io(file, extension, "dimacs", &[]))
    }

    /// Panics if this clause representation has a different model count than that of FeatureIDE.
    ///
    /// Useful for checking the correctness of count-preserving algorithms (e.g., [Formula::to_cnf_tseitin]).
    pub(crate) fn assert_count(&self, file: &str, extension: &str) {
        assert_eq!(self.count(), Self::count_featureide(file, extension));
    }
}

impl<'a> From<FormulaContext<'a>> for Clauses {
    fn from(formula: FormulaContext) -> Self {
        let mut vars = vec![];
        let mut var_remap = HashMap::<VarId, VarId>::new();
        formula.formula.vars(formula.arena).into_iter().for_each(|(var_id, var)| {
            var_remap.insert(var_id, vars.len().try_into().unwrap());
            vars.push(var.clone());
        });
        Self {
            clauses: Self::clauses(&formula, &var_remap),
            vars,
            var_remap,
        }
    }
}

impl fmt::Display for Clauses {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, var) in self.vars.iter().enumerate() {
            if let Var::Named(name) = var {
                debug_assert!(!name.is_empty());
            }
            write!(f, "c {} {var}\n", i + 1)?;
        }
        write!(f, "p cnf {} {}\n", self.vars.len(), self.clauses.len())?;
        for clause in &self.clauses {
            for literal in clause {
                let var: usize = literal.unsigned_abs().try_into().unwrap();
                debug_assert_ne!(var + 1, 0);
                debug_assert!(var < self.vars.len());
                write!(f, "{} ", literal + 1)?;
            }
            write!(f, "0\n")?;
        }
        write!(f, "")
    }
}
