//! Parser for feature-model formulas.

use pest::{
    iterators::{Pair, Pairs},
    pratt_parser::{Assoc, Op, PrattParser},
    Parser,
};
use pest_derive::Parser;

use crate::formula::{Expr::*, Formula, Id};

#[derive(Parser)]
#[grammar = "model.pest"]
struct ModelParser;

// fn parse_children<'a>(pair: Pair<'a, Rule>, formula: &mut Formula<'a>) -> Vec<Id> {
//     pair.into_inner()
//         .map(|pair| parse_pair(pair, formula))
//         .collect()
// }

// fn parse_pair<'a>(pair: Pair<'a, Rule>, formula: &mut Formula<'a>) -> Id {
//     match pair.as_rule() {
//         Rule::var => formula.var(pair.into_inner().next().unwrap().as_str()),
//         Rule::not => {
//             let child_id = parse_pair(pair.into_inner().next().unwrap(), formula);
//             formula.expr(Not(child_id))
//         }
//         Rule::and => {
//             let child_ids = parse_children(pair, formula);
//             formula.expr(And(child_ids))
//         }
//         Rule::or => {
//             let child_ids = parse_children(pair, formula);
//             formula.expr(Or(child_ids))
//         }
//         _ => unreachable!(),
//     }
// }
fn parse_expr<'a>(pairs: Pairs<'a, Rule>, parser: &PrattParser<Rule>, formula: &mut Formula<'a>) -> Id {
    parser
        .map_primary(|primary| match primary.as_rule() {
            Rule::int => formula.var(primary.as_str()),
            Rule::expr => parse_expr(primary.into_inner(), parser, formula),
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::or => formula.expr(Or(vec![lhs, rhs])),
            Rule::and => formula.expr(And(vec![lhs, rhs])),
            _ => unreachable!(),
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::not => formula.expr(Not(rhs)),
            _ => unreachable!(),
        })
        .parse(pairs)
}

impl<'a> From<&'a str> for Formula<'a> {
    fn from(model: &'a str) -> Self {
        let mut formula = Formula::new();
        let mut child_ids = Vec::<Id>::new();

        let parser = PrattParser::new()
            .op(Op::infix(Rule::or, Assoc::Left))
            .op(Op::infix(Rule::and, Assoc::Left))
            .op(Op::prefix(Rule::not));

        for line in model.lines() {
            // let pair = ModelParser::parse(Rule::line, line)
            //     .expect("failed to parse model file")
            //     .next()
            //     .unwrap();

            // match pair.as_rule() {
            //     Rule::EOI => (),
            //     _ => child_ids.push(parse_pair(pair, &mut formula)),
            // }

            parser
                .map_primary(|primary| match primary.as_rule() {
                    Rule::int => dbg!(primary.as_str()),
                    Rule::expr => todo!(),
                    _ => unreachable!(),
                })
                .map_infix(|lhs, op, rhs| match op.as_rule() {
                    Rule::or => todo!(),
                    Rule::and => todo!(),
                    _ => unreachable!(),
                })
                .map_prefix(|op, rhs| match op.as_rule() {
                    Rule::not => todo!(),
                    _ => unreachable!(),
                });
        }

        let root_id = if child_ids.len() == 1 {
            child_ids[0]
        } else {
            formula.expr(And(child_ids))
        }; // todo: maybe move this unary simplification straight into .expr?
        formula.set_root_expr(root_id);
        formula
    }
}
