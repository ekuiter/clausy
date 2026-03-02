//! Parser for DIMACS .sat files.

use super::FormulaParser;
use crate::core::{
    arena::Arena,
    expr::{Expr::*, ExprId},
    file::File,
    formula::Formula,
    var::VarId,
};
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use std::{
    collections::{HashMap, HashSet},
    vec,
};

/// Parses feature-model formula files in the .sat format.
#[derive(Parser)]
#[grammar = "parser/sat.pest"]
pub(super) struct SatFormulaParser;

fn parse_children(pair: Pair<Rule>, vars: &[ExprId], arena: &mut Arena) -> Vec<ExprId> {
    pair.into_inner()
        .map(|pair| parse_pair(pair, vars, arena))
        .collect()
}

fn parse_pair(pair: Pair<Rule>, vars: &[ExprId], arena: &mut Arena) -> ExprId {
    match pair.as_rule() {
        Rule::var => {
            let var: VarId = pair
                .clone()
                .into_inner()
                .peek()
                .expect("SAT variable token missing integer payload")
                .as_str()
                .parse()
                .expect("SAT variable token contains invalid integer");
            let var: usize = var
                .unsigned_abs()
                .try_into()
                .expect("SAT variable index does not fit into usize");
            if pair.as_str().starts_with("-") {
                arena.expr(Not(vars[var]))
            } else {
                vars[var]
            }
        }
        Rule::not => {
            let child_id = parse_pair(
                pair.into_inner()
                    .next()
                    .expect("SAT NOT expression missing child"),
                vars,
                arena,
            );
            arena.expr(Not(child_id))
        }
        Rule::and => {
            let child_ids = parse_children(pair, vars, arena);
            arena.expr(And(child_ids))
        }
        Rule::or => {
            let child_ids = parse_children(pair, vars, arena);
            arena.expr(Or(child_ids))
        }
        _ => unreachable!(),
    }
}

impl FormulaParser for SatFormulaParser {
    fn parse_into(&self, file: File, arena: &mut Arena) -> Formula {
        let mut pairs = SatFormulaParser::parse(Rule::file, &file.contents)
            .unwrap_or_else(|err| panic!("failed to parse SAT file '{}': {err}", file.name));

        let mut sub_var_ids = HashSet::<VarId>::new();
        let mut variable_names = HashMap::<VarId, &str>::new();
        while let Rule::comment = pairs
            .peek()
            .expect("SAT parser unexpectedly reached EOF while scanning comments")
            .as_rule()
        {
            let pair = pairs
                .next()
                .expect("SAT parser expected comment pair")
                .into_inner()
                .next()
                .expect("SAT comment missing inner payload");
            if let Rule::comment_var = pair.as_rule() {
                let mut pairs = pair.into_inner();
                let var: VarId = pairs
                    .next()
                    .expect("SAT comment_var missing variable id")
                    .as_str()
                    .parse()
                    .expect("SAT comment_var contains invalid variable id");
                let name = pairs
                    .next()
                    .expect("SAT comment_var missing variable name")
                    .as_str()
                    .trim();
                debug_assert!(!variable_names.contains_key(&var));
                variable_names.insert(var, name);
            }
        }

        let n: VarId = pairs
            .next()
            .expect("SAT parser missing preamble")
            .into_inner()
            .next()
            .expect("SAT preamble missing variable count")
            .as_str()
            .parse()
            .expect("SAT preamble contains invalid variable count");
        let mut vars: Vec<ExprId> = vec![0];
        for i in 1..=n {
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
        debug_assert!(variable_names.is_empty());

        let root_id = parse_pair(
            pairs.next().expect("SAT parser missing root expression"),
            &vars,
            arena,
        );
        Formula::new(sub_var_ids, root_id, Some(file))
    }
}
