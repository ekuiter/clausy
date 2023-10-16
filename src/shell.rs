//! Imperative shell for operating on feature-model formulas.

use std::collections::{HashMap, HashSet};

use crate::core::clauses::Clauses;
use crate::core::formula::{Expr::*, ExprInFormula, Id, Var, VarId};
use crate::parser::sat_inline::SatInlineFormulaParser;

use crate::util::exec;
use crate::{
    core::formula::Formula,
    parser::{parser, FormulaParsee},
    util::{file_exists, read_file},
};

/// Converts a formula into its clause representation, if not done yet.
macro_rules! clauses {
    ($clauses:expr, $formula:expr) => {{
        if $clauses.is_none() {
            $clauses = Some(Clauses::from(&$formula));
        }
        $clauses.as_ref().unwrap()
    }};
}

/// Main entry point.
///
/// Parses and runs each given command in order.
pub fn main(mut commands: Vec<String>) {
    let mut formula = Formula::new();
    let mut clauses = None;
    let mut parsed_file_names = Vec::<String>::new();
    let mut parsed_files = HashMap::<String, (String, Option<String>)>::new();
    let mut parsed_results = vec![];

    if commands.is_empty() {
        commands.push("-".to_string());
    }

    if commands.len() == 1 && file_exists(&commands[0]) {
        commands.push("to_cnf_dist".to_string());
        commands.push("to_clauses".to_string());
        commands.push("print".to_string());
    }

    for command in &commands {
        if file_exists(command) {
            let (mut file, extension) = read_file(command);
            file = parser(extension.clone()).preprocess(file);
            parsed_file_names.push(command.clone());
            parsed_files.insert(command.clone(), (file, extension));
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
            "print_sub_exprs" => {
                for id in formula.sub_exprs() {
                    println!("{}", ExprInFormula(&formula, &id));
                }
            }
            "to_canon" => formula = formula.to_canon(),
            "to_nnf" => formula = formula.to_nnf(),
            "to_cnf_dist" => formula = formula.to_cnf_dist(),
            "to_cnf_tseitin" => formula = formula.to_cnf_tseitin(),
            "to_clauses" => clauses = Some(Clauses::from(&formula)),
            "satisfy" => println!("{}", clauses!(clauses, formula).satisfy().unwrap()),
            "count" => println!("{}", clauses!(clauses, formula).count()),
            "assert_count" => {
                if parsed_files.len() == 1 {
                    let (file, extension) = parsed_files.values().next().unwrap();
                    clauses!(clauses, formula).assert_count(file, &extension.as_ref().unwrap());
                } else {
                    unreachable!();
                }
            }
            "enumerate" => clauses!(clauses, formula).enumerate(),
            "compare" => {
                debug_assert!(parsed_files.len() == 2);
                debug_assert!(parsed_results.len() == 2);
                let (root_id_a, var_ids_a): &(Id, HashSet<VarId>) = &parsed_results[0];
                let (root_id_b, var_ids_b): &(Id, HashSet<VarId>) = &parsed_results[1];
                println!("formula a has {} variables", var_ids_a.len());
                println!("formula b has {} variables", var_ids_b.len());

                let common_var_ids: HashSet<VarId> = var_ids_a
                    .intersection(var_ids_b)
                    .map(|var_id| *var_id)
                    .collect();
                let common_vars = common_var_ids
                    .iter()
                    .map(|var_id| {
                        let var_id: usize = var_id.unsigned_abs().try_into().unwrap();
                        if let Var::Named(name) = &formula.vars[var_id] {
                            name.clone()
                        } else {
                            unreachable!()
                        }
                    })
                    .collect::<Vec<String>>();
                println!("both formulas have {} common variables", common_vars.len());

                let mut get_not_root_id = |root_id, other_var_ids, name| {
                    let mut ids = vec![root_id];
                    ids.extend(formula.named_vars_except(other_var_ids).iter().flat_map(
                        |var_id| {
                            if !common_var_ids.contains(var_id) {
                                let expr = formula.expr(Var(*var_id));
                                Some(formula.expr(Not(expr)))
                            } else {
                                None
                            }
                        },
                    ));
                    println!(
                        "{} variables are exclusive to formula {} and will be sliced",
                        ids.len() - 1,
                        name
                    );
                    let expr = formula.expr(And(ids));
                    formula.expr(Not(expr))
                };
                let not_root_id_a = get_not_root_id(*root_id_a, var_ids_b, "a");
                let not_root_id_b = get_not_root_id(*root_id_b, var_ids_a, "b");

                let (file, extension) = &parsed_files[&parsed_file_names[0]];
                let common_vars = common_vars.iter().map(|s| &**s).collect::<Vec<&str>>();
                let slice_a = exec::io(file, extension.as_ref().unwrap(), "sat", &common_vars);
                let (root_id_slice_a, var_ids_slice_a) =
                    formula.parse(&slice_a, parser(Some("sat".to_string())));
                println!(
                    "the slice of a differs in its supposed variables in {:?}",
                    common_var_ids
                        .symmetric_difference(&var_ids_slice_a)
                        .collect::<HashSet<&VarId>>()
                );

                let (file, extension) = &parsed_files[&parsed_file_names[1]];
                let slice_b = exec::io(file, extension.as_ref().unwrap(), "sat", &common_vars);
                let (root_id_slice_b, var_ids_slice_b) =
                    formula.parse(&slice_b, parser(Some("sat".to_string())));
                println!(
                    "the slice of b differs in its supposed variables in {:?}",
                    common_var_ids
                        .symmetric_difference(&var_ids_slice_b)
                        .collect::<HashSet<&VarId>>()
                );

                let root_id = formula.expr(And(vec![root_id_slice_a, not_root_id_a]));
                formula.set_root_expr(root_id);
                formula = formula.to_cnf_tseitin();
                let c = clauses!(clauses, formula); // set of variables not accurate
                                                    //println!("{}", c);
                dbg!(c.count());
                return;
            }
            _ => {
                if file_exists(command) {
                    let (file, extension) = &parsed_files[command];
                    parsed_results.push(formula.parse(&file, parser(extension.clone())));
                    let (root_id, _) = *parsed_results.last().unwrap();
                    formula.set_root_expr(root_id);
                } else if SatInlineFormulaParser::can_parse(command) {
                    let root_id = SatInlineFormulaParser::new(parsed_results.clone(), true)
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
