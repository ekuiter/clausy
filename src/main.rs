use std::{
    env, fs,
    io::Read, path::Path, ffi::OsStr,
};

use clausy::{cnf::CNF, formula::Formula, exec};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut model;
    if args.len() == 2 {
        model = fs::read_to_string(&args[1]).expect("could not read feature model");
        // todo: move to parser.rs
        let extension = Path::new(&args[1]).extension().or(Some(OsStr::new("model"))).unwrap().to_str().unwrap();
        if extension != "model" {
            model = exec::io(&model, extension, "model");
        }
    } else {
        model = String::new();
        std::io::stdin().read_to_string(&mut model).unwrap();
    };

    // todo: implement command parser:
    // e.g., "a.uvl; b.uvl; nnf; cnf tseitin root; root a.uvl -b.uvl; count" would count the number of removed products

    let mut formula = Formula::from(&model[..]).assert_valid(); // todo: parsing takes long for linux
    println!("{}", formula);
    formula = formula.to_nnf().assert_valid();
    //formula = formula.to_cnf_dist().assert_valid();
    formula = formula.to_cnf_tseitin().assert_valid();
    formula = formula.to_nnf().assert_valid();
    // println!("{}", formula);
    let cnf = CNF::from(formula);
    println!("{}", cnf);
    // cnf.assert_count(&model);
}
