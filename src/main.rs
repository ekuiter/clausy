use std::{fs, env};

use cnfrust::{formula::Formula, cnf::CNF};

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_name = if args.len() == 2 { &args[1] } else { "test.model" };
    let model_string = fs::read_to_string(file_name).expect("could not open model file");
    let mut formula = Formula::from(&model_string[..]);
    //println!("{}", formula);
    formula = formula.to_nnf();
    //println!("{}", formula);
    formula = formula.to_cnf_dist();
    //println!("{}", formula);
    println!("{}", CNF::from(&formula));
}