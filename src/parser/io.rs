//! Parser for feature-model formula files in any format accepted by FeatureIDE.

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::core::formula::{Expr::*, Formula, Id};

use super::FormulaParser;

struct IOFormulaParser;

impl FormulaParser for ModelFormulaParser {
    fn parse_into<'a>(&self, model: &'a str, formula: &mut Formula<'a>) -> Id {
    //     let mut model;
    // if args.len() == 3 {
    //     // 2 {
    //     model = fs::read_to_string(&args[1]).expect("could not read feature model");
    //     // todo: move to parser.rs
    //     let extension = Path::new(&args[1])
    //         .extension()
    //         .or(Some(OsStr::new("model")))
    //         .unwrap()
    //         .to_str()
    //         .unwrap();
    //     if extension != "model" {
    //         model = util::exec::io(&model, extension, "model");
    //     }
        util::exec::io(&model, extension, "model")
    }
}
