//! Clause representation of a feature-model formula.

use num::BigInt;

use super::{
    expr::{Expr::*, ExprId},
    formula_ref::FormulaRef,
    var::{Var, VarId},
};
use crate::util::{exec, log::log};
use std::fmt::Write as _;
use std::{
    collections::{HashMap, HashSet},
    fmt, slice,
};

/// A [super::formula::Formula] in its clause representation.
///
/// That is, this data structure enforces conjunctive normal form.
pub(crate) struct Clauses {
    /// The clauses of this clause representation.
    ///
    /// A clause is a [Vec] of literals, each given as an absolute-value index into [Clauses::vars].
    /// Negative values indicate negated variable occurrences.
    /// These variable IDs are unrelated to the variables from the original [Arena], but they index into [Clauses::vars].
    pub(crate) clauses: Vec<Vec<VarId>>,

    /// The variables of this clause representation.
    ///
    /// This list is indexed into by the absolute values stored in [Clauses::clauses].
    pub(crate) vars: Vec<Var>,

    /// Maps the original [Arena]'s variable IDs to [Clauses::vars]'s variable IDs.
    ///
    /// Must be stored explicitly for [Clauses::proj_count], in which users provide variable IDs from the
    /// original [Arena], while external model counters operate on [Clauses::vars]'s variable IDs.
    pub(crate) var_remap: HashMap<VarId, VarId>,
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
                    Not(_) => panic!(
                        "unexpected double negation in clause representation: {:?}",
                        formula_ref.arena.exprs[*child_id]
                    ),
                    And(child_ids) => {
                        if child_ids.is_empty() {
                            // Not(And()) is a contradiction and can be omitted from the current clause
                        } else {
                            panic!("unexpected And in clause representation");
                        }
                    }
                    Or(child_ids) => {
                        if child_ids.is_empty() {
                            // Not(Or()) is a vacuous truth and satisfies the current clause
                            return true;
                        } else {
                            panic!("unexpected Or in clause representation");
                        }
                    }
                },
                And(child_ids) => {
                    if child_ids.is_empty() {
                        // And() is a vacuous truth and satisfies the current clause
                        return true;
                    } else {
                        panic!("unexpected And in clause representation");
                    }
                }
                Or(child_ids) => {
                    if child_ids.is_empty() {
                        // Or() is a contradiction and can be omitted from the current clause
                    } else {
                        panic!("unexpected Or in clause representation");
                    }
                }
            };
            false
        };
        let mut add_clause = |child_ids: &[ExprId]| {
            let mut clause = Vec::<VarId>::new();
            let mut tautological = false;
            for child_id in child_ids {
                tautological = tautological || add_literal(*child_id, &mut clause);
            }
            if tautological {
                // this clause is a tautology, so we can skip it in principle
                if !var_remap.is_empty() {
                    // however, if there are variables, add a trivial clause, just to be sure
                    // we do this because not all solvers handle the zero-clause formula correctly
                    // (which may emerge if this is the only clause)
                    clauses.push(vec![1, -1]);
                } else {
                    log("[CLAUSES] WARNING: encountered a tautological clause in a zero-variable formula");
                    log("[CLAUSES] WARNING: in case that this leads to an overall tautology, not all solvers will handle it correctly");
                }
            } else if clause.is_empty() {
                // if the clause is still empty after all literals have been processed, it is a contradiction
                // in principle, we could just push the empty clause here - but again, not all solvers handle this correctly
                if !var_remap.is_empty() {
                    // so, if at least one variable is available, we push an explicit contradiction to be safe
                    clauses.push(vec![1]);
                    clauses.push(vec![-1]);
                } else {
                    // this contradiction cannot be expressed because there are no variables
                    // so we are forced to emit an empty clause
                    clauses.push(clause);
                    log("[CLAUSES] WARNING: encountered a contradictory clause in a zero-variable formula");
                    log("[CLAUSES] WARNING: this leads to an overall contradiction, but not all solvers will handle it correctly");
                }
            } else {
                // this is now a normal clause (the typical and nice case)
                clauses.push(clause);
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
                        And(_) => panic!("unexpected nested And in clause representation"),
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
                let idx: usize = literal
                    .unsigned_abs()
                    .try_into()
                    .expect("solution literal index does not fit into usize");
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
        exec::sat(&self.to_string()).map(|solution| self.solution_to_string(&solution))
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
        exec::sharp_sat(&self.to_string())
    }

    /// Counts the projected number of solutions of this clause representation.
    pub(crate) fn proj_count(&self, proj_vars: &HashSet<VarId>) -> BigInt {
        exec::sharp_sat(&self.to_projected_string(proj_vars))
    }

    /// Returns a CNF file of this clause representation that is annotated for projected model counting.
    ///
    /// The projection variables must be provided in the context of the original [Arena]
    /// from when this clause representation was created.
    /// We use the standard format for projected model counting here, as specified in `meta/mccomp_format_24.pdf`:
    /// `c t pmc\np cnf <num_vars> <num_clauses> ...\nc p show <proj_var_1> <proj_var_2> ... 0\n`
    /// d4 and Ganak both support this format. d4 also supports an alternative format:
    /// `p pcnf <num_vars> <num_clauses> <num_proj_vars>\nvp <proj_var_1> <proj_var_2> ... 0\n...`
    /// We do not use this format here because it is specific to d4.
    /// Note that `p show` may occur anywhere according to `meta/mccomp_format_24.pdf`,
    /// but Ganak requires it to be at the end.
    pub(crate) fn to_projected_string(&self, proj_vars: &HashSet<VarId>) -> String {
        let mut proj_vars: Vec<VarId> = proj_vars
            .iter()
            .map(|var_id| {
                *self.var_remap.get(var_id).unwrap_or_else(|| {
                    panic!(
                        "projection variable id {} is not part of this clause representation",
                        var_id
                    )
                })
            })
            .collect();
        proj_vars.sort_unstable();
        let mut cnf = String::new();
        cnf.push_str("c t pmc\n");
        write!(&mut cnf, "{}", self).unwrap();
        cnf.push_str("c p show ");
        for var in proj_vars {
            write!(&mut cnf, "{var} ").unwrap();
        }
        cnf.push_str("0\n");
        cnf
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
                var_remap.insert(
                    var_id,
                    vars.len()
                        .try_into()
                        .expect("number of clause variables does not fit into VarId"),
                );
                vars.push(var.clone());
            });
        Self {
            clauses: Self::clauses(&formula_ref, &var_remap),
            vars,
            var_remap,
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
                let var: usize = literal
                    .unsigned_abs()
                    .try_into()
                    .expect("clause literal index does not fit into usize");
                debug_assert_ne!(var, 0);
                debug_assert!(var < self.vars.len());
                write!(f, "{} ", literal)?;
            }
            writeln!(f, "0")?;
        }
        write!(f, "")
    }
}
