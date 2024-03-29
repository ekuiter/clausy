//! Clause representation of a feature-model formula.

use num::BigInt;

use super::{
    expr::{Expr::*, ExprId},
    formula_ref::FormulaRef,
    var::{Var, VarId},
};
use crate::{core::file::File, util::exec};
use std::{collections::HashMap, fmt, slice};

/// A [super::formula::Formula] in its clause representation.
///
/// That is, this data structure enforces conjunctive normal form.
pub(crate) struct Clauses {
    /// The clauses of this clause representation.
    ///
    /// A clause is a [Vec] of literals, each given as an absolute-value index into [Clauses::vars].
    /// Negative values indicate negated variable occurrences.
    pub(crate) clauses: Vec<Vec<VarId>>,

    /// The variables of this clause representation.
    ///
    /// This list is indexed into by the absolute values stored in [Clauses::clauses].
    pub(crate) vars: Vec<Var>,
}

impl Clauses {
    /// Returns the sub-expressions of a formula as clauses.
    ///
    /// We require that the formula already is in conjunctive normal form (see [super::formula::Formula::to_cnf_dist]).
    /// If there is no clause, the represented formula is a tautology.
    /// If there is an empty clause, the represented formula is a contradiction.
    /// If there is at least one variable, the empty clause is translated as And(1, -1), as some solvers do not treat the empty clause correctly.
    fn clauses(formula_ref: &FormulaRef, var_remap: &HashMap<VarId, VarId>) -> Vec<Vec<VarId>> {
        let mut clauses = Vec::<Vec<VarId>>::new();
        let add_literal = |id, clause: &mut Vec<VarId>| {
            match &formula_ref.arena.exprs[id] {
                Var(var_id) => clause.push(var_remap[&var_id]),
                Not(child_id) => match &formula_ref.arena.exprs[*child_id] {
                    Var(var_id) => clause.push(-var_remap[&var_id]),
                    Not(_) => unreachable!(),
                    And(child_ids) => {
                        if child_ids.is_empty() {
                        } else {
                            unreachable!();
                        }
                    }
                    Or(child_ids) => {
                        if child_ids.is_empty() {
                            return true;
                        } else {
                            unreachable!();
                        }
                    }
                },
                And(child_ids) => {
                    if child_ids.is_empty() {
                        return true;
                    } else {
                        unreachable!();
                    }
                }
                Or(child_ids) => {
                    if !child_ids.is_empty() {
                        unreachable!();
                    }
                }
            };
            false
        };
        let mut add_clause = |child_ids: &[ExprId]| {
            let mut clause = Vec::<VarId>::new();
            let contradictory = child_ids.is_empty();
            let mut tautological = false;
            for child_id in child_ids {
                tautological = tautological || add_literal(*child_id, &mut clause);
            }
            if tautological {
                if !var_remap.is_empty() {
                    clauses.push(vec![1, -1]);
                }
            } else {
                if contradictory && !var_remap.is_empty() {
                    clauses.push(vec![1]);
                    clauses.push(vec![-1]);
                } else {
                    clauses.push(clause);
                }
            }
        };
        match &formula_ref.arena.exprs[formula_ref.formula.root_id] {
            Var(_) | Not(_) => add_clause(slice::from_ref(&formula_ref.formula.root_id)),
            Or(child_ids) => add_clause(child_ids),
            And(child_ids) => {
                for child_id in child_ids {
                    match &formula_ref.arena.exprs[*child_id] {
                        Var(_) | Not(_) => add_clause(slice::from_ref(child_id)),
                        Or(child_ids) => add_clause(&child_ids),
                        _ => unreachable!(),
                    }
                }
            }
        }
        clauses
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
    pub(crate) fn count(&self) -> BigInt {
        let file = File::new("-.dimacs".to_string(), self.to_string());
        exec::d4(&file.contents)
    }
}

impl<'a> From<FormulaRef<'a>> for Clauses {
    fn from(formula_ref: FormulaRef) -> Self {
        let mut vars = vec![Var::Aux(0)];
        let mut var_remap = HashMap::<VarId, VarId>::new();
        formula_ref
            .formula
            .sub_vars(formula_ref.arena)
            .into_iter()
            .for_each(|(var_id, var)| {
                var_remap.insert(var_id, vars.len().try_into().unwrap());
                vars.push(var.clone());
            });
        Self {
            clauses: Self::clauses(&formula_ref, &var_remap),
            vars,
        }
    }
}

impl fmt::Display for Clauses {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, var) in self.vars.iter().enumerate() {
            if i == 0 {
                continue;
            }
            if let Var::Named(name) = var {
                debug_assert!(!name.is_empty());
            }
            writeln!(f, "c {} {var}", i)?;
        }
        writeln!(f, "p cnf {} {}", self.vars.len() - 1, self.clauses.len())?;
        for clause in &self.clauses {
            for literal in clause {
                let var: usize = literal.unsigned_abs().try_into().unwrap();
                debug_assert_ne!(var, 0);
                debug_assert!(var < self.vars.len());
                write!(f, "{} ", literal)?;
            }
            writeln!(f, "0")?;
        }
        write!(f, "")
    }
}
