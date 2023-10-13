//! Parser for inline input in a .sat-like format.

use std::collections::HashSet;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::core::formula::{Expr::*, Formula, Id, VarId};

/// Parses inline input in a .sat-like format.
/// 
/// In this format, identifiers refer to previously parsed inputs.
/// Optionally, the parsed formula can add negative backbone variables to align differing sets of variables.
#[derive(Parser)]
#[grammar = "parser/sat_inline.pest"]
pub(crate) struct SatInlineFormulaParser {
    parsed_results: Vec<(Id, HashSet<VarId>)>,
    add_backbone_vars: bool
}

impl SatInlineFormulaParser {
    pub(crate) fn new(parsed_results: Vec<(Id, HashSet<VarId>)>, add_backbone_vars: bool) -> Self {
        SatInlineFormulaParser { parsed_results, add_backbone_vars }
    }

    pub(crate) fn can_parse(file: &String) -> bool {
        SatInlineFormulaParser::parse(Rule::file, file).is_ok()
    }

    pub(crate) fn parse_into(&self, file: &String, formula: &mut Formula) -> Id {
        let mut pairs = SatInlineFormulaParser::parse(Rule::file, file).unwrap();

        self.parse_pair(pairs.next().unwrap(), formula)
    }

    fn parse_children(
        &self,
        pair: Pair<Rule>,
        formula: &mut Formula,
    ) -> Vec<Id> {
        pair.into_inner()
            .map(|pair| self.parse_pair(pair, formula))
            .collect()
    }
    
    fn parse_pair(
        &self,
        pair: Pair<Rule>,
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
                let (mut root_id, ref var_ids) = self.parsed_results[idx - 1];
                if self.add_backbone_vars {
                    let mut ids = vec![root_id];
                    ids.extend(formula.named_vars_except(var_ids).iter().map(|var_id| {
                        let expr = formula.expr(Var(*var_id));
                        formula.expr(Not(expr))
                    }));
                    root_id = formula.expr(And(ids));
                }
                if pair.as_str().starts_with("-") {
                    formula.expr(Not(root_id))
                } else {
                    root_id
                }
            }
            Rule::not => {
                let child_id = self.parse_pair(pair.into_inner().next().unwrap(), formula);
                formula.expr(Not(child_id))
            }
            Rule::and => {
                let child_ids = self.parse_children(pair, formula);
                formula.expr(And(child_ids))
            }
            Rule::or => {
                let child_ids = self.parse_children(pair, formula);
                formula.expr(Or(child_ids))
            }
            _ => unreachable!(),
        }
    }
}