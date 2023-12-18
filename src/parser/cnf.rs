//! Parser for DIMACS .cnf files.

use super::FormulaParser;
use crate::core::{
    arena::Arena,
    expr::{Expr::*, ExprId},
    file::File,
    formula::Formula,
    var::VarId,
};
use pest::Parser;
use pest_derive::Parser;
use std::{
    collections::{HashMap, HashSet},
    vec,
};

/// Parses feature-model formula files in the .cnf format.
#[derive(Parser)]
#[grammar = "parser/cnf.pest"]
pub(super) struct CnfFormulaParser;

impl FormulaParser for CnfFormulaParser {
    fn parse_into(&self, file: File, arena: &mut Arena) -> Formula {
        let mut pairs = CnfFormulaParser::parse(Rule::file, &file.contents).unwrap();

        let mut sub_var_ids = HashSet::<VarId>::new();
        let mut variable_names = HashMap::<VarId, &str>::new();
        while let Rule::comment = pairs.peek().unwrap().as_rule() {
            let pair = pairs.next().unwrap().into_inner().next().unwrap();
            if let Rule::comment_var = pair.as_rule() {
                let mut pairs = pair.into_inner();
                let var: VarId = pairs.next().unwrap().as_str().parse().unwrap();
                let name = pairs.next().unwrap().as_str().trim();
                debug_assert!(!variable_names.contains_key(&var));
                variable_names.insert(var, name);
            }
        }

        let mut vars: Vec<ExprId> = vec![0];
        let clause_n: VarId;
        {
            let mut pairs = pairs.next().unwrap().into_inner();
            let var_n: VarId = pairs.next().unwrap().as_str().parse().unwrap();
            clause_n = pairs.next().unwrap().as_str().parse().unwrap();
            for i in 1..=var_n {
                if variable_names.contains_key(&i) {
                    let (expr_id, var_id) = arena.var_expr_with_id(variable_names[&i].to_string());
                    sub_var_ids.insert(var_id);
                    vars.push(expr_id);
                    variable_names.remove(&i);
                } else {
                    let (var_id, expr_id) = arena.add_var_aux_expr();
                    sub_var_ids.insert(var_id);
                    vars.push(expr_id);
                }
            }
            assert!(variable_names.is_empty());
        }

        let child_ids: Vec<ExprId> = pairs
            .map_while(|pair| match pair.as_rule() {
                Rule::clause => {
                    let children_ids = pair
                        .clone()
                        .into_inner()
                        .map(|pair| {
                            let var: VarId = pair.as_str().parse().unwrap();
                            let var: usize = var.unsigned_abs().try_into().unwrap();
                            if pair.as_str().starts_with("-") {
                                arena.expr(Not(vars[var]))
                            } else {
                                vars[var]
                            }
                        })
                        .collect();
                    Some(arena.expr(Or(children_ids)))
                }
                Rule::EOI => None,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(clause_n, child_ids.len().try_into().unwrap());

        let root_id = arena.expr(And(child_ids));
        Formula::new(sub_var_ids, root_id, Some(file))
    }
}
