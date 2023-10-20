//! Imperative shell for operating on feature-model formulas.

use crate::core::clauses::Clauses;
use crate::parser::sat_inline::SatInlineFormulaParser;
use crate::{
    core::{
        arena::Arena,
        formula::Formula,
    },
    parser::{parser, FormulaParsee},
    util::{file_exists, read_file},
};

/// Whether to print identifiers of expressions.
///
/// Useful for debugging, but should generally be disabled, as this is expected by [crate::tests].
pub(super) const PRINT_ID: bool = false; // todo: make configurable

/// Prefix for auxiliary variables.
///
/// Auxiliary variables are required by some algorithms on formulas and can be created with [Var::Aux].
pub(super) const VAR_AUX_PREFIX: &str = "_aux_"; // todo: make configurable (also whether aux vars should even be listed)

macro_rules! formula {
    ($formulas:expr) => {
        $formulas.last_mut().unwrap()
    };
}

/// Converts a formula into its clause representation, if not done yet.
macro_rules! clauses {
    ($clauses:expr, $arena:expr, $formulas:expr) => {{
        if $clauses.is_none() {
            $clauses = Some(Clauses::from(formula!($formulas).as_ref(&$arena)));
        }
        $clauses.as_ref().unwrap()
    }};
}

/// Main entry point.
///
/// Parses and runs each given command in order.
pub fn main(mut commands: Vec<String>) {
    let mut arena = Arena::new();
    let mut formulas = Vec::<Formula>::new();
    let mut clauses = None;
    let mut parsed_file = None;

    if commands.is_empty() {
        commands.push("-".to_string());
    }

    if commands.len() == 1 && file_exists(&commands[0]) {
        commands.push("to_cnf_dist".to_string());
        commands.push("to_clauses".to_string());
        commands.push("print".to_string());
    }

    for command in &commands {
        match command.as_str() {
            "print" => {
                if clauses.is_some() {
                    print!("{}", clauses.as_ref().unwrap());
                } else {
                    println!("{}", formula!(formulas).as_ref(&arena));
                };
            }
            "print_sub_exprs" => {
                for id in formula!(formulas).sub_exprs(&mut arena) {
                    println!("{}", arena.as_formula(id).as_ref(&arena));
                }
            }
            "to_canon" => formula!(formulas).to_canon(&mut arena),
            "to_nnf" => formula!(formulas).to_nnf(&mut arena),
            "to_cnf_dist" => formula!(formulas).to_cnf_dist(&mut arena),
            "to_cnf_tseitin" => formula!(formulas).to_cnf_tseitin(&mut arena),
            "to_clauses" => clauses = Some(Clauses::from(formula!(formulas).as_ref(&mut arena))),
            "satisfy" => println!("{}", clauses!(clauses, arena, formulas).satisfy().unwrap()),
            "count" => println!("{}", clauses!(clauses, arena, formulas).count()),
            "assert_count" => {
                let (file, extension): &(String, Option<String>) = parsed_file.as_ref().unwrap();
                clauses!(clauses, arena, formulas).assert_count(file, extension.as_ref().unwrap());
            }
            "enumerate" => clauses!(clauses, arena, formulas).enumerate(),
            "compare" => {
                // debug_assert!(parsed_files.len() == 2);
                // debug_assert!(formulas.len() == 2);
                // let (root_id_a, var_ids_a): &(Id, HashSet<VarId>) = &formulas[0];
                // let (root_id_b, var_ids_b): &(Id, HashSet<VarId>) = &formulas[1];
                // println!("formula a has {} variables", var_ids_a.len());
                // println!("formula b has {} variables", var_ids_b.len());

                // let common_var_ids: HashSet<VarId> = var_ids_a
                //     .intersection(var_ids_b)
                //     .map(|var_id| *var_id)
                //     .collect();
                // let common_vars = common_var_ids
                //     .iter()
                //     .map(|var_id| {
                //         let var_id: usize = var_id.unsigned_abs().try_into().unwrap();
                //         if let Var::Named(name) = &arena.vars[var_id] {
                //             name.clone()
                //         } else {
                //             unreachable!()
                //         }
                //     })
                //     .collect::<Vec<String>>();
                // println!("both formulas have {} common variables", common_vars.len());

                // let mut get_not_root_id = |root_id, other_var_ids, name| {
                //     let mut ids = vec![root_id];
                //     ids.extend(arena.filter_vars(other_var_ids).iter().flat_map(
                //         |var_id| {
                //             if !common_var_ids.contains(var_id) {
                //                 let expr = arena.expr(Var(*var_id));
                //                 Some(arena.expr(Not(expr)))
                //             } else {
                //                 None
                //             }
                //         },
                //     ));
                //     println!(
                //         "{} variables are exclusive to formula {} and will be sliced",
                //         ids.len() - 1,
                //         name
                //     );
                //     let expr = arena.expr(And(ids));
                //     arena.expr(Not(expr))
                // };
                // let not_root_id_a = get_not_root_id(*root_id_a, var_ids_b, "a");
                // let not_root_id_b = get_not_root_id(*root_id_b, var_ids_a, "b");

                // let (file, extension) = &parsed_files[&parsed_file_names[0]];
                // let common_vars = common_vars.iter().map(|s| &**s).collect::<Vec<&str>>();
                // let slice_a = exec::io(file, extension.as_ref().unwrap(), "sat", &common_vars);
                // let (root_id_slice_a, var_ids_slice_a) =
                //     arena.parse(&slice_a, parser(Some("sat".to_string())));
                // println!(
                //     "the slice of a differs in its supposed variables in {:?}",
                //     common_var_ids
                //         .symmetric_difference(&var_ids_slice_a)
                //         .collect::<HashSet<&VarId>>()
                // );

                // let (file, extension) = &parsed_files[&parsed_file_names[1]];
                // let slice_b = exec::io(file, extension.as_ref().unwrap(), "sat", &common_vars);
                // let (root_id_slice_b, var_ids_slice_b) =
                //     arena.parse(&slice_b, parser(Some("sat".to_string())));
                // println!(
                //     "the slice of b differs in its supposed variables in {:?}",
                //     common_var_ids
                //         .symmetric_difference(&var_ids_slice_b)
                //         .collect::<HashSet<&VarId>>()
                // );

                // let root_id = arena.expr(And(vec![root_id_slice_a, not_root_id_a]));
                // arena.set_root_expr(root_id);
                // arena = arena.to_cnf_tseitin();
                // let c = clauses!(clauses, arena); // set of variables not accurate
                //                                     //println!("{}", c);
                // dbg!(c.count());
                // return;
            }
            _ => {
                if file_exists(command) {
                    let (mut file, extension) = read_file(command);
                    file = parser(extension.clone()).preprocess(file);
                    formulas.push(arena.parse(&file, parser(extension.clone())));
                    parsed_file = Some((file, extension));
                } else if SatInlineFormulaParser::can_parse(command) {
                    formulas.push(
                        SatInlineFormulaParser::new(&formulas, true)
                            .parse_into(&command, &mut arena),
                    );
                } else {
                    unreachable!();
                }
                clauses = None;
            }
        }
        #[cfg(debug_assertions)]
        {
            if formulas.last().is_some() {
                formulas.last_mut().unwrap().assert_canon(&mut arena);
            }
        }
    }
}
