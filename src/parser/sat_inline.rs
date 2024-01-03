//! Parser for inline input in a .sat-like format.

use std::collections::HashSet;

use crate::core::{
    arena::Arena,
    expr::{Expr::*, ExprId},
    formula::Formula,
};
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

/// Parses inline input in a .sat-like format.
///
/// In this format, identifiers refer to previously parsed inputs.
/// Optionally, the parsed formula can add negative backbone literals to align differing sets of variables.
/// As sub-variables, the parsed formula has the union of the sub-variables of all given formulas.
#[derive(Parser)]
#[grammar = "parser/sat_inline.pest"]
pub(crate) struct SatInlineFormulaParser<'a> {
    formulas: &'a Vec<Formula>,
    force_foreign_vars: Option<bool>,
}

impl<'a> SatInlineFormulaParser<'a> {
    pub(crate) fn new(formulas: &'a Vec<Formula>, force_foreign_vars: Option<bool>) -> Self {
        SatInlineFormulaParser {
            formulas,
            force_foreign_vars,
        }
    }

    pub(crate) fn can_parse(file: &String) -> bool {
        SatInlineFormulaParser::parse(Rule::file, file).is_ok()
    }

    pub(crate) fn parse_into(&self, file: &String, arena: &mut Arena) -> Formula {
        let mut pairs = SatInlineFormulaParser::parse(Rule::file, file).unwrap();
        let root_id = self.parse_pair(pairs.next().unwrap(), arena);
        let sub_var_ids = if self.force_foreign_vars.is_some() {
            arena.var_ids()
        } else {
            self.formulas
                .iter()
                .flat_map(|formula| formula.sub_var_ids.clone())
                .collect()
        };
        Formula::new(sub_var_ids, root_id, None)
    }

    fn parse_children(&self, pair: Pair<Rule>, arena: &mut Arena) -> Vec<ExprId> {
        pair.into_inner()
            .map(|pair| self.parse_pair(pair, arena))
            .collect()
    }

    fn parse_pair(&self, pair: Pair<Rule>, arena: &mut Arena) -> ExprId {
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
                let formula = &self.formulas[idx - 1];
                let mut root_id = formula.root_id;
                if self.force_foreign_vars.is_some() {
                    root_id = formula
                        .force_foreign_vars(self.force_foreign_vars.unwrap(), &HashSet::new(), arena)
                        .root_id;
                }
                if pair.as_str().starts_with("-") {
                    arena.expr(Not(root_id))
                } else {
                    root_id
                }
            }
            Rule::not => {
                let child_id = self.parse_pair(pair.into_inner().next().unwrap(), arena);
                arena.expr(Not(child_id))
            }
            Rule::and => {
                let child_ids = self.parse_children(pair, arena);
                arena.expr(And(child_ids))
            }
            Rule::or => {
                let child_ids = self.parse_children(pair, arena);
                arena.expr(Or(child_ids))
            }
            _ => unreachable!(),
        }
    }
}
