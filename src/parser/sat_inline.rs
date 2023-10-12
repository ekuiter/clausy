//! Parser for inline input in a .sat-like format.

use std::collections::HashSet;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::core::formula::{Expr::*, Formula, Id, VarId};

/// Parses inline input in a .sat-like format.
#[derive(Parser)]
#[grammar = "parser/sat_inline.pest"]
pub(crate) struct SatInlineFormulaParser {
    parsed_results: Vec<(Id, HashSet<VarId>)>,
}

impl SatInlineFormulaParser {
    pub(crate) fn new(parsed_results: Vec<(Id, HashSet<VarId>)>) -> Self {
        SatInlineFormulaParser { parsed_results }
    }

    pub(crate) fn can_parse(file: &String) -> bool {
        SatInlineFormulaParser::parse(Rule::file, file).is_ok()
    }

    pub(crate) fn parse_into(&self, file: &String, formula: &mut Formula) -> Id {
        let mut pairs = SatInlineFormulaParser::parse(Rule::file, file).unwrap();

        parse_pair(pairs.next().unwrap(), &self.parsed_results, formula)
    }
}

fn parse_children(
    pair: Pair<Rule>,
    parsed_results: &Vec<(Id, HashSet<VarId>)>,
    formula: &mut Formula,
) -> Vec<Id> {
    pair.into_inner()
        .map(|pair| parse_pair(pair, parsed_results, formula))
        .collect()
}

fn parse_pair(
    pair: Pair<Rule>,
    parsed_results: &Vec<(Id, HashSet<VarId>)>,
    formula: &mut Formula,
) -> Id {
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
            let (root_id, ref var_ids) = parsed_results[idx - 1];
            let mut ids = vec![root_id];
            ids.extend(formula.named_vars_except(var_ids).iter().map(|var_id| {
                let expr = formula.expr(Var(*var_id));
                formula.expr(Not(expr))
            }));
            let root_id = formula.expr(And(ids));
            if pair.as_str().starts_with("-") {
                formula.expr(Not(root_id))
            } else {
                root_id
            }
        }
        Rule::not => {
            let child_id = parse_pair(pair.into_inner().next().unwrap(), parsed_results, formula);
            formula.expr(Not(child_id))
        }
        Rule::and => {
            let child_ids = parse_children(pair, parsed_results, formula);
            formula.expr(And(child_ids))
        }
        Rule::or => {
            let child_ids = parse_children(pair, parsed_results, formula);
            formula.expr(Or(child_ids))
        }
        _ => unreachable!(),
    }
}
