//! Parser for inline input in a .sat-like format.

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::core::formula::{Expr::*, Formula, Id};

/// Parses inline input in a .sat-like format.
#[derive(Parser)]
#[grammar = "parser/sat_inline.pest"]
pub(crate) struct SatInlineFormulaParser {
    parsed_ids: Vec<Id>
}

impl SatInlineFormulaParser {
    pub(crate) fn new(parsed_ids: Vec<Id>) -> Self {
        SatInlineFormulaParser { parsed_ids }
    }

    pub(crate) fn can_parse(file: &String) -> bool {
        SatInlineFormulaParser::parse(Rule::file, file).is_ok()
    }

    pub(crate) fn parse_into(&self, file: &String, formula: &mut Formula) -> Id {
        let mut pairs = SatInlineFormulaParser::parse(Rule::file, file).unwrap();
    
        parse_pair(pairs.next().unwrap(), &self.parsed_ids, formula)
    }
}

fn parse_children(pair: Pair<Rule>, parsed_ids: &[Id], formula: &mut Formula) -> Vec<Id> {
    pair.into_inner()
        .map(|pair| parse_pair(pair, parsed_ids, formula))
        .collect()
}

fn parse_pair(pair: Pair<Rule>, parsed_ids: &[Id], formula: &mut Formula) -> Id {
    match pair.as_rule() {
        Rule::var => {
            let arg: i32 = pair
                .clone()
                .into_inner()
                .peek()
                .unwrap()
                .as_str()
                .parse()
                .unwrap();
            let idx: usize = arg.try_into().unwrap();
            let id: Id = parsed_ids[idx - 1];
            if pair.as_str().starts_with("-") {
                formula.expr(Not(id))
            } else {
                id
            }
        }
        Rule::not => {
            let child_id = parse_pair(pair.into_inner().next().unwrap(), parsed_ids, formula);
            formula.expr(Not(child_id))
        }
        Rule::and => {
            let child_ids = parse_children(pair, parsed_ids, formula);
            formula.expr(And(child_ids))
        }
        Rule::or => {
            let child_ids = parse_children(pair, parsed_ids, formula);
            formula.expr(Or(child_ids))
        }
        _ => unreachable!(),
    }
}