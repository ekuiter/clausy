use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::formula::{Expr::*, Formula, Id};

#[derive(Parser)]
#[grammar = "model.pest"]
struct ModelParser;

// could also be implemented iteratively
fn parse_pair<'a>(pair: Pair<Rule>, formula: &mut Formula) -> Id {
    match pair.as_rule() {
        Rule::var => formula.get_var(pair.into_inner().next().unwrap().as_str()),
        Rule::not => {
            let child_id = parse_pair(pair.into_inner().next().unwrap(), formula);
            formula.add_expr(Not(child_id))
        }
        Rule::and => {
            let child_ids: Vec<u32> = pair
                .into_inner()
                .map(|pair| parse_pair(pair, formula))
                .collect();
            formula.add_expr(And(child_ids))
        }
        Rule::or => {
            let child_ids: Vec<u32> = pair
                .into_inner()
                .map(|pair| parse_pair(pair, formula))
                .collect();
            formula.add_expr(Or(child_ids))
        }
        _ => unreachable!(),
    }
}

impl<'a> From<&'a str> for Formula<'a> {
    fn from(model_string: &'a str) -> Self {
        // could also be parsed line by line and not entire file to save space (variables need to be stored, though)
        let pairs = ModelParser::parse(Rule::file, model_string).expect("failed to parse model file");
        let mut formula = Formula::new();
        let mut child_ids = Vec::<u32>::new();

        let mut vars: Vec<&str> = pairs
            .clone()
            .find_tagged("var")
            .map(|pair| pair.as_str())
            .collect();
        vars.sort();
        vars.dedup();
        for var in vars {
            formula.add_var(var);
        }

        for pair in pairs {
            if let Rule::EOI = pair.as_rule() {
            } else {
                child_ids.push(parse_pair(pair, &mut formula));
            }
        }
        let root_id = formula.add_expr(And(child_ids));
        formula.set_root_expr(root_id);

        formula
    }
}
