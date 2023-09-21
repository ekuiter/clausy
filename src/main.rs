use std::{env, fs, io::Read};

use clausy::{cnf::CNF, formula::Formula};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut model;
    if args.len() == 2 {
        model = fs::read_to_string(&args[1]).expect("could not open feature model");
    } else {
        model = String::new();
        std::io::stdin().read_to_string(&mut model).unwrap();
    };

    let mut formula = Formula::from(&model[..]);
    println!("{}", formula);
    formula = formula.to_nnf();
    println!("{}", formula);
    formula.assert_shared();
    formula = formula.to_cnf_dist();
    formula.assert_shared();
    println!("{}", CNF::from(formula));
}