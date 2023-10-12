//! Parser for DIMACS .sat files.

use std::{collections::{HashMap, HashSet}, vec};

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::core::formula::{Expr::*, Formula, Id, VarId};

use super::FormulaParser;

/// Parses feature-model formula files in the .sat format.
#[derive(Parser)]
#[grammar = "parser/sat.pest"]
pub(crate) struct SatFormulaParser;

fn parse_children<'a>(pair: Pair<'a, Rule>, vars: &[Id], formula: &mut Formula<'a>) -> Vec<Id> {
    pair.into_inner()
        .map(|pair| parse_pair(pair, vars, formula))
        .collect()
}

fn parse_pair<'a>(pair: Pair<'a, Rule>, vars: &[Id], formula: &mut Formula<'a>) -> Id {
    match pair.as_rule() {
        Rule::var => {
            let var: VarId = pair
                .clone()
                .into_inner()
                .peek()
                .unwrap()
                .as_str()
                .parse()
                .unwrap();
            let var: usize = var.try_into().unwrap();
            if pair.as_str().starts_with("-") {
                formula.expr(Not(vars[var]))
            } else {
                vars[var]
            }
        }
        Rule::not => {
            let child_id = parse_pair(pair.into_inner().next().unwrap(), vars, formula);
            formula.expr(Not(child_id))
        }
        Rule::and => {
            let child_ids = parse_children(pair, vars, formula);
            formula.expr(And(child_ids))
        }
        Rule::or => {
            let child_ids = parse_children(pair, vars, formula);
            formula.expr(Or(child_ids))
        }
        _ => unreachable!(),
    }
}

impl FormulaParser for SatFormulaParser {
    fn parse_into<'a>(&self, file: &'a String, formula: &mut Formula<'a>) -> (Id, HashSet<VarId>) {
        let mut pairs = SatFormulaParser::parse(Rule::file, file).unwrap();

        let mut var_ids = HashSet::<VarId>::new();
        let mut variable_names = HashMap::<VarId, &str>::new();
        while let Rule::comment = pairs.peek().unwrap().as_rule() {
            let pair = pairs.next().unwrap().into_inner().next().unwrap();
            if let Rule::comment_var = pair.as_rule() {
                let mut pairs = pair.into_inner();
                let var: VarId = pairs.next().unwrap().as_str().parse().unwrap();
                let name = pairs.next().unwrap().as_str().trim();
                debug_assert!(!variable_names.contains_key(&var));
                variable_names.insert(var, name);
            }
        }

        let n: VarId = pairs
            .next()
            .unwrap()
            .into_inner()
            .next()
            .unwrap()
            .as_str()
            .parse()
            .unwrap();
        let mut vars: Vec<Id> = vec![0];
        for i in 1..=n {
            if variable_names.contains_key(&i) {
                let (expr_id, var_id) = formula.var_expr_with_id(variable_names.get(&i).unwrap());
                var_ids.insert(var_id);
                vars.push(expr_id);
                variable_names.remove(&i);
            } else {
                vars.push(formula.add_var_aux_expr());
            }
        }
        debug_assert!(variable_names.is_empty());

        (parse_pair(pairs.next().unwrap(), &vars, formula), var_ids)
    }
}
