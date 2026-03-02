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
        let mut pairs = CnfFormulaParser::parse(Rule::file, &file.contents)
            .unwrap_or_else(|err| panic!("failed to parse CNF file '{}': {err}", file.name));

        let mut sub_var_ids = HashSet::<VarId>::new();
        let mut variable_names = HashMap::<VarId, &str>::new();
        while let Rule::comment = pairs
            .peek()
            .expect("CNF parser unexpectedly reached EOF while scanning comments")
            .as_rule()
        {
            let pair = pairs
                .next()
                .expect("CNF parser expected comment pair")
                .into_inner()
                .next()
                .expect("CNF comment missing inner payload");
            if let Rule::comment_var = pair.as_rule() {
                let mut pairs = pair.into_inner();
                let var: VarId = pairs
                    .next()
                    .expect("CNF comment_var missing variable id")
                    .as_str()
                    .parse()
                    .expect("CNF comment_var contains invalid variable id");
                let name = pairs
                    .next()
                    .expect("CNF comment_var missing variable name")
                    .as_str()
                    .trim();
                debug_assert!(!variable_names.contains_key(&var));
                variable_names.insert(var, name);
            }
        }

        let mut vars: Vec<ExprId> = vec![0];
        let clause_n: VarId;
        {
            let mut pairs = pairs
                .next()
                .expect("CNF parser missing preamble")
                .into_inner();
            let var_n: VarId = pairs
                .next()
                .expect("CNF preamble missing variable count")
                .as_str()
                .parse()
                .expect("CNF preamble contains invalid variable count");
            clause_n = pairs
                .next()
                .expect("CNF preamble missing clause count")
                .as_str()
                .parse()
                .expect("CNF preamble contains invalid clause count");
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
                            let var: VarId = pair
                                .as_str()
                                .parse()
                                .expect("CNF clause contains invalid literal");
                            let var: usize = var
                                .unsigned_abs()
                                .try_into()
                                .expect("CNF literal index does not fit into usize");
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
        let parsed_clause_n: VarId = child_ids
            .len()
            .try_into()
            .expect("number of parsed CNF clauses does not fit into VarId");
        assert_eq!(
            clause_n, parsed_clause_n,
            "error: CNF preamble declares {} clauses, but parser produced {} clauses",
            clause_n, parsed_clause_n
        );

        let root_id = arena.expr(And(child_ids));
        Formula::new(sub_var_ids, root_id, Some(file))
    }
}
