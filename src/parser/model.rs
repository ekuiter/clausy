//! Parser for KConfigReader .model files.

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::core::formula::{Expr::*, Formula, Id};

use super::FormulaParser;

/// Parses feature-model formula files in the .model format.
#[derive(Parser)]
#[grammar = "parser/model.pest"]
pub(crate) struct ModelFormulaParser;

fn parse_children<'a>(pair: Pair<'a, Rule>, formula: &mut Formula<'a>) -> Vec<Id> {
    pair.into_inner()
        .map(|pair| parse_pair(pair, formula))
        .collect()
}

fn parse_pair<'a>(pair: Pair<'a, Rule>, formula: &mut Formula<'a>) -> Id {
    match pair.as_rule() {
        Rule::var => formula.var_expr(pair.into_inner().next().unwrap().as_str().trim()),
        Rule::not => {
            let child_id = parse_pair(pair.into_inner().next().unwrap(), formula);
            formula.expr(Not(child_id))
        }
        Rule::and => {
            let child_ids = parse_children(pair, formula);
            formula.expr(And(child_ids))
        }
        Rule::or => {
            let child_ids = parse_children(pair, formula);
            formula.expr(Or(child_ids))
        }
        _ => unreachable!(),
    }
}

fn parse_into<'a>(file: &'a str, formula: &mut Formula<'a>) -> Id {
    let mut child_ids = Vec::<Id>::new();
    for line in file.lines() {
        let pair = ModelFormulaParser::parse(Rule::line, line)
            .unwrap()
            .next()
            .unwrap();

        match pair.as_rule() {
            Rule::EOI => (),
            _ => child_ids.push(parse_pair(pair, formula)),
        }
    }
    formula.expr(And(child_ids))
}

impl FormulaParser for ModelFormulaParser {
    fn parse_into<'a>(&self, file: &'a String, formula: &mut Formula<'a>) -> Id {
        parse_into(file, formula)
    }
}

impl<'a> From<&'a str> for Formula<'a> {
    fn from(file: &'a str) -> Self {
        let mut formula = Formula::new();
        let root_id = parse_into(file, &mut formula);
        formula.set_root_expr(root_id);
        formula
    }
}
