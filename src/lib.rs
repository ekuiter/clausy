use std::{env, ffi::OsStr, fs, io::Read, path::Path};

use crate::core::{
    cnf::CNF,
    formula::{Expr::*, Formula},
};

mod core;
mod util;
mod parser;
mod tests;

pub fn main(args: &[String]) {
    let mut model;
    if args.len() == 3 {
        // 2 {
        model = fs::read_to_string(&args[1]).expect("could not read feature model");
        // todo: move to parser.rs
        let extension = Path::new(&args[1])
            .extension()
            .or(Some(OsStr::new("model")))
            .unwrap()
            .to_str()
            .unwrap();
        if extension != "model" {
            model = util::exec::io(&model, extension, "model");
        }
    } else {
        model = String::new();
        std::io::stdin().read_to_string(&mut model).unwrap();
    };

    // todo: implement command parser:
    // e.g., "a.uvl; b.uvl; nnf; cnf tseitin root; root a.uvl -b.uvl; count" would count the number of removed products

    let mut formula = Formula::from(&model[..]).assert_valid(); // todo: parsing takes long for linux
    let model2 = fs::read_to_string(&args[2]).expect("could not read feature model");
    let id = parser::sat::parse_model(&model2, &mut formula);
    //let expr = And(vec![formula.get_root_expr(), formula.expr(Not(id))]);
    //let id = formula.expr(expr);
    formula.set_root_expr(id);

    println!("{}", formula);
    formula = formula.to_nnf().assert_valid();
    // //formula = formula.to_cnf_dist().assert_valid();
    // formula = formula.to_cnf_tseitin().assert_valid();
    // formula = formula.to_nnf().assert_valid();
    // // println!("{}", formula);
    // let cnf = CNF::from(formula);
    // println!("{}", cnf);
    // dbg!(cnf.count());
    // cnf.assert_count(&model);
}
