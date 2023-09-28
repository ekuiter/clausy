use std::collections::HashMap;

use parser::sat_inline::SatInlineFormulaParser;

use crate::{
    core::formula::{Expr::*, Formula, Id},
    parser::{parser, FormulaParsee},
    util::{file_exists, read_file},
};

mod core;
mod parser;
mod tests;
mod util;

/// Main entry point.
///
/// Parses and runs each given command in order.
pub fn main(mut commands: Vec<String>) {
    let mut formula = Formula::new();
    let mut clauses = None;
    let mut parsed_files = HashMap::<String, (String, Option<String>)>::new();
    let mut parsed_ids = vec![];

    if commands.is_empty() {
        commands.push("-".to_string());
    }

    if commands.len() == 1 && file_exists(&commands[0]) {
        commands.push("to_nnf".to_string());
        commands.push("to_cnf_dist".to_string());
        commands.push("to_clauses".to_string());
        commands.push("print".to_string());
    }

    for command in &commands {
        if file_exists(command) {
            let (mut file, extension) = read_file(command);
            file = parser(extension.clone()).preprocess(file);
            parsed_files.insert(command.to_string(), (file, extension));
        }
    }

    for command in &commands {
        match command.as_str() {
            "print" => {
                if clauses.is_some() {
                    print!("{}", clauses.as_ref().unwrap());
                } else {
                    println!("{}", formula);
                };
            }
            "to_nnf" => formula = formula.to_nnf(),
            "to_cnf_dist" => formula = formula.to_cnf_dist(),
            "to_cnf_tseitin" => formula = formula.to_cnf_tseitin(),
            "to_clauses" => clauses = Some(formula.to_clauses()),
            "satisfy" => todo!(),
            "tautology" => todo!(),
            "count" => println!("{}", clauses.as_ref().unwrap().count()),
            "enumerate" => todo!(),
            "compare" => todo!(),
            _ => {
                if file_exists(command) {
                    let (file, extension) = parsed_files.get(command).unwrap();
                    parsed_ids.push(formula.parse(&file, parser(extension.clone())));
                    formula.set_root_expr(*parsed_ids.last().unwrap());
                } else if SatInlineFormulaParser::can_parse(command) {
                    let root_id = SatInlineFormulaParser::new(parsed_ids.clone())
                        .parse_into(&command, &mut formula);
                    formula.set_root_expr(root_id);
                } else {
                    unreachable!();
                }
                clauses = None;
            }
        }
        #[cfg(debug_assertions)]
        {
            formula = formula.assert_valid();
            if clauses.is_some() {
                clauses.as_ref().unwrap().assert_valid();
            }
        }
    }
}
