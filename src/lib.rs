use std::collections::HashMap;

use crate::{
    core::{
        cnf::Cnf,
        formula::{Expr::*, Formula, Id},
    },
    parser::{parser, FormulaParsee},
    util::{read_file, readable_file},
};

mod core;
mod parser;
mod tests;
mod util;

// doc todo
pub fn main(commands: &[String]) {
    let mut formula = Formula::new();
    let mut cnf = None;
    let mut parsed_files = HashMap::<String, (String, Option<String>)>::new();
    let mut parsed_ids = vec![];

    // todo: allow no arguments at all with reasonable defaults
    for file_name in commands {
        if readable_file(file_name) {
            let (mut file, extension) = read_file(file_name.as_str());
            file = parser(extension.clone()).preprocess(file);
            parsed_files.insert(file_name.clone(), (file, extension));
        }
    }

    for command in commands {
        let mut args = command.split(' ');
        match args.next().unwrap() {
            "print" | "p" => {
                if cnf.is_some() {
                    println!("{}", cnf.as_ref().unwrap());
                } else {
                    println!("{}", formula);
                };
            }
            "nnf" | "n" => formula = formula.to_nnf().assert_valid(),
            "cnf_dist" | "d" => formula = formula.to_cnf_dist().assert_valid(),
            "cnf_tseitin" | "t" => formula = formula.to_cnf_tseitin().assert_valid(),
            "cnf" | "c" => cnf = Some(Cnf::from(&formula)),
            "count" => println!("{}", cnf.as_ref().unwrap().count()),
            "root" => {
                let args: Vec<Id> = args
                    .map(|arg| {
                        let arg: i32 = arg.parse().unwrap();
                        let idx: usize = arg.unsigned_abs().try_into().unwrap();
                        let id: Id = parsed_ids[idx - 1];
                        if arg > 0 {
                            id
                        } else {
                            formula.expr(Not(id))
                        }
                    })
                    .collect();
                let root_id = formula.expr(And(args)); // todo: also allow other operators (use parser?)
                formula.set_root_expr(root_id);
            }
            // todo: when comparing (merging during parse?), set dead variables
            // todo: integrate sat solver for checking tautologies
            file_name => {
                if readable_file(file_name) {
                    let (file, extension) = parsed_files.get(file_name).unwrap();
                    parsed_ids.push(formula.parse(&file, parser(extension.clone())));
                    formula.set_root_expr(*parsed_ids.last().unwrap());
                    formula = formula.assert_valid();
                } else {
                    panic!("command {} invalid", file_name);
                }
            }
        }
    }
}
