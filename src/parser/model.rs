//! Parser for KConfigReader .model files.

use std::collections::HashSet;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::core::{
    arena::Arena,
    expr::{Expr::*, ExprId},
    formula::Formula,
    var::VarId,
};

use super::FormulaParser;

/// Parses feature-model formula files in the .model format.
#[derive(Parser)]
#[grammar = "parser/model.pest"]
pub(crate) struct ModelFormulaParser;

fn parse_children(pair: Pair<Rule>, arena: &mut Arena, var_ids: &mut HashSet<VarId>) -> Vec<ExprId> {
    pair.into_inner()
        .map(|pair| parse_pair(pair, arena, var_ids))
        .collect()
}

fn parse_pair(pair: Pair<Rule>, arena: &mut Arena, var_ids: &mut HashSet<VarId>) -> ExprId {
    match pair.as_rule() {
        Rule::var => {
            let (expr_id, var_id) = arena.var_expr_with_id(
                pair.into_inner()
                    .next()
                    .unwrap()
                    .as_str()
                    .trim()
                    .to_string(),
            );
            var_ids.insert(var_id);
            expr_id
        }
        Rule::not => {
            let child_id = parse_pair(pair.into_inner().next().unwrap(), arena, var_ids);
            arena.expr(Not(child_id))
        }
        Rule::and => {
            let child_ids = parse_children(pair, arena, var_ids);
            arena.expr(And(child_ids))
        }
        Rule::or => {
            let child_ids = parse_children(pair, arena, var_ids);
            arena.expr(Or(child_ids))
        }
        _ => unreachable!(),
    }
}

fn parse_into(file: &str, arena: &mut Arena) -> Formula {
    let mut child_ids = Vec::<ExprId>::new();
    let mut var_ids = HashSet::<VarId>::new();
    for line in file.lines() {
        let pair = ModelFormulaParser::parse(Rule::line, line)
            .unwrap()
            .next()
            .unwrap();

        match pair.as_rule() {
            Rule::EOI => (),
            _ => child_ids.push(parse_pair(pair, arena, &mut var_ids)),
        }
    }
    let root_id = arena.expr(And(child_ids));
    Formula::new(root_id, var_ids)
}

impl FormulaParser for ModelFormulaParser {
    fn parse_into(&self, file: &str, arena: &mut Arena) -> Formula {
        parse_into(file, arena)
    }
}
