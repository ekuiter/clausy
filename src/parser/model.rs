//! Parser for KConfigReader .model files.

use std::collections::HashSet;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::core::formula::{Expr::*, Formula, Id, VarId};

use super::FormulaParser;

/// Parses feature-model formula files in the .model format.
#[derive(Parser)]
#[grammar = "parser/model.pest"]
pub(crate) struct ModelFormulaParser;

fn parse_children<'a>(
    pair: Pair<'a, Rule>,
    formula: &mut Formula<'a>,
    var_ids: &mut HashSet<VarId>,
) -> Vec<Id> {
    pair.into_inner()
        .map(|pair| parse_pair(pair, formula, var_ids))
        .collect()
}

fn parse_pair<'a>(pair: Pair<'a, Rule>, formula: &mut Formula<'a>, var_ids: &mut HashSet<VarId>) -> Id {
    match pair.as_rule() {
        Rule::var => {
            let (expr_id, var_id) =
                formula.var_expr_with_id(pair.into_inner().next().unwrap().as_str().trim());
            var_ids.insert(var_id);
            expr_id
        }
        Rule::not => {
            let child_id = parse_pair(pair.into_inner().next().unwrap(), formula, var_ids);
            formula.expr(Not(child_id))
        }
        Rule::and => {
            let child_ids = parse_children(pair, formula, var_ids);
            formula.expr(And(child_ids))
        }
        Rule::or => {
            let child_ids = parse_children(pair, formula, var_ids);
            formula.expr(Or(child_ids))
        }
        _ => unreachable!(),
    }
}

fn parse_into<'a>(file: &'a str, formula: &mut Formula<'a>) -> (Id, HashSet<VarId>) {
    let mut child_ids = Vec::<Id>::new();
    let mut var_ids = HashSet::<VarId>::new();
    for line in file.lines() {
        let pair = ModelFormulaParser::parse(Rule::line, line)
            .unwrap()
            .next()
            .unwrap();

        match pair.as_rule() {
            Rule::EOI => (),
            _ => child_ids.push(parse_pair(pair, formula, &mut var_ids)),
        }
    }
    (formula.expr(And(child_ids)), var_ids)
}

impl FormulaParser for ModelFormulaParser {
    fn parse_into<'a>(&self, file: &'a String, formula: &mut Formula<'a>) -> (Id, HashSet<VarId>) {
        parse_into(file, formula)
    }
}

impl<'a> From<&'a str> for Formula<'a> {
    fn from(file: &'a str) -> Self {
        let mut formula = Formula::new();
        let (root_id, _) = parse_into(file, &mut formula);
        formula.set_root_expr(root_id);
        formula
    }
}
