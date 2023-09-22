use std::{
    env, fs,
    io::{Read, Write},
    process::{Command, Stdio},
};

use clausy::{cnf::CNF, formula::Formula};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut model;
    if args.len() == 2 {
        model = fs::read_to_string(&args[1]).expect("could not read feature model");
    } else {
        model = String::new();
        std::io::stdin().read_to_string(&mut model).unwrap();
    };

    let mut formula = Formula::from(&model[..]).assert_valid();
    //println!("{}", formula);
    formula = formula.to_nnf().assert_valid();
    formula = formula.to_cnf_dist().assert_valid();
    formula = formula.to_cnf_tseitin().assert_valid();
    formula = formula.to_nnf().assert_valid();
    println!("{}", formula);
    let cnf = CNF::from(formula);
    println!("{}", cnf);
    cnf.assert_count(&model);
}
