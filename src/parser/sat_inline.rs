//! Parser for inline input in a .sat-like format.

use crate::core::{
    arena::Arena,
    expr::{Expr::*, ExprId},
    formula::Formula,
    var::VarId,
};
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use std::collections::HashSet;

/// Parses inline input in a .sat-like format.
///
/// In this format, identifiers refer to previously parsed inputs.
/// Optionally, the parsed formula can add negative backbone variables to align differing sets of variables.
#[derive(Parser)]
#[grammar = "parser/sat_inline.pest"]
pub(crate) struct SatInlineFormulaParser {
    formulas: Vec<Formula>,
    add_backbone_vars: bool,
}

impl SatInlineFormulaParser {
    pub(crate) fn new(formulas: Vec<Formula>, add_backbone_vars: bool) -> Self {
        SatInlineFormulaParser {
            formulas,
            add_backbone_vars,
        }
    }

    pub(crate) fn can_parse(file: &String) -> bool {
        SatInlineFormulaParser::parse(Rule::file, file).is_ok()
    }

    pub(crate) fn parse_into(&self, file: &String, arena: &mut Arena) -> Formula {
        let mut pairs = SatInlineFormulaParser::parse(Rule::file, file).unwrap();
        let root_id = self.parse_pair(pairs.next().unwrap(), arena);
        Formula::new(HashSet::new(), root_id) // todo: merge variables of used formulas
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
                let mut root_id = formula.get_root_expr();
                if self.add_backbone_vars {
                    let mut ids = vec![root_id];
                    let var_ids = formula
                        .sub_vars(arena)
                        .iter()
                        .map(|(var_id, _)| *var_id)
                        .collect::<HashSet<VarId>>();
                    ids.extend(
                        arena
                            .vars(|var_id, _| !var_ids.contains(&var_id))
                            .into_iter()
                            .map(|(var_id, _)| {
                                let expr = arena.expr(Var(var_id));
                                arena.expr(Not(expr))
                            }),
                    );
                    root_id = arena.expr(And(ids));
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
