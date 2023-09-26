use std::{env, ffi::OsStr, fs, io::Read, path::Path};

use crate::{core::{
    cnf::Cnf,
    formula::{Expr::*, Formula},
}, parser::model::ModelFormulaParser};

mod core;
mod util;
mod parser;
mod tests;

pub fn main(args: &[String]) {
    let mut file;
    let extension;
    if args.len() == 2 {
        // 2 {
        file = fs::read_to_string(&args[1]).expect("could not read feature model");
        // todo: move to parser.rs
        extension = Path::new(&args[1])
            .extension()
            .map_or(None, |e| e.to_str());
    } else {
        file = String::new();
        extension = None;
        std::io::stdin().read_to_string(&mut file).unwrap();
    };

    // todo: implement command parser:
    // e.g., "a.uvl; b.uvl; nnf; cnf tseitin root; root a.uvl -b.uvl; count" would count the number of removed products

    let mut formula = Formula::from((&mut file, extension)).assert_valid(); // todo: parsing takes long for linux
    //let model2 = fs::read_to_string(&args[2]).expect("could not read feature model");
    //let id = parser::sat::parse_model(&model2, &mut formula);
    //let expr = And(vec![formula.get_root_expr(), formula.expr(Not(id))]);
    //let id = formula.expr(expr);
    //formula.set_root_expr(id);

    println!("{}", formula);
    formula = formula.to_nnf().assert_valid();
    // //formula = formula.to_cnf_dist().assert_valid();
    // formula = formula.to_cnf_tseitin().assert_valid();
    // formula = formula.to_nnf().assert_valid();
    // // println!("{}", formula);
    // let cnf = Cnf::from(formula);
    // println!("{}", cnf);
    // dbg!(cnf.count());
    // cnf.assert_count(&model);
}
