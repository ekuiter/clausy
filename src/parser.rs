use std::fs;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::{
    cnf::CNF,
    formula::{Expr::*, Formula, Id},
};

#[derive(Parser)]
#[grammar = "model.pest"]
pub struct ModelParser;

// optimizations: implement iteratively, parse line by line and not entire file
fn parse<'a>(pair: Pair<Rule>, formula: &mut Formula) -> Id {
    match pair.as_rule() {
        Rule::var => formula.get_var(pair.into_inner().next().unwrap().as_str()),
        Rule::not => {
            let child_id = parse(pair.into_inner().next().unwrap(), formula);
            formula.add_expr(Not(child_id))
        }
        Rule::and => {
            let child_ids: Vec<u32> = pair.into_inner().map(|pair| parse(pair, formula)).collect();
            formula.add_expr(And(child_ids))
        }
        Rule::or => {
            let child_ids: Vec<u32> = pair.into_inner().map(|pair| parse(pair, formula)).collect();
            formula.add_expr(Or(child_ids))
        }
        _ => unreachable!(),
    }
}

pub fn test() {
    let file = fs::read_to_string("test.model").unwrap();
    let pairs = ModelParser::parse(Rule::file, &file).unwrap();
    let mut child_ids = Vec::<u32>::new();
    let mut formula = Formula::new();

    let mut vars: Vec<&str> = pairs
        .clone()
        .find_tagged("var")
        .map(|pair| pair.as_str())
        .collect();
    vars.sort();
    vars.dedup();
    for var in dbg!(vars) {
        formula.add_var(var);
    }

    for pair in pairs {
        if let Rule::EOI = pair.as_rule() {
        } else {
            child_ids.push(parse(pair, &mut formula));
        }
    }
    let root_id = formula.add_expr(And(child_ids));
    formula.set_root_expr(root_id);
    println!("{}", formula);
    formula = formula.to_nnf().to_cnf_dist();
    println!("{}", formula);
    println!("{}", CNF::new(&formula));
}
