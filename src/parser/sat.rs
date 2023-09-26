use std::{collections::HashMap, vec};

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::core::formula::{Expr::*, Formula, Id, VarId};

#[derive(Parser)]
#[grammar = "parser/sat.pest"]
struct SatParser;

fn parse_children<'a>(pair: Pair<'a, Rule>, vars: &[Id], formula: &mut Formula<'a>) -> Vec<Id> {
    pair.into_inner()
        .map(|pair| parse_pair(pair, vars, formula))
        .collect()
}

fn parse_pair<'a>(pair: Pair<'a, Rule>, vars: &[Id], formula: &mut Formula<'a>) -> Id {
    match pair.as_rule() {
        Rule::var => {
            let var: VarId = pair.clone().into_inner().peek().unwrap().as_str().parse().unwrap();
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

pub(crate) fn parse_model<'a>(model: &'a str, formula: &mut Formula<'a>) -> Id {
    let mut pairs = SatParser::parse(Rule::file, model).expect("failed to parse sat file");

    let mut variable_names = HashMap::<VarId, &str>::new();
    while let Rule::comment = pairs.peek().unwrap().as_rule() {
        let pair = pairs.next().unwrap().into_inner().next().unwrap();
        if let Rule::comment_var = pair.as_rule() {
            let mut pairs = pair.into_inner();
            let var: VarId = pairs.next().unwrap().as_str().parse().unwrap();
            let name = pairs.next().unwrap().as_str();
            assert!(
                !variable_names.contains_key(&var),
                "named same variable twice"
            );
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
            vars.push(formula.var(variable_names.get(&i).unwrap()));
            variable_names.remove(&i);
        } else {
            vars.push(formula.add_var_aux());
        }
    }
    assert!(variable_names.is_empty(), "named invalid variable");

    parse_pair(pairs.next().unwrap(), &vars, formula)

    //  // todo: maybe move this unary simplification straight into .expr?
    // if child_ids.len() == 1 {
    //     child_ids[0]
    // } else {
    //     formula.expr(And(child_ids))
    // }
}
