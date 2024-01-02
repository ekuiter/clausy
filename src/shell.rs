//! Imperative shell for operating on feature-model formulas.

use crate::core::file::File;
use crate::core::formula::DiffCommand;
use crate::parser::sat_inline::SatInlineFormulaParser;
use crate::{
    core::{arena::Arena, formula::Formula},
    parser::{parser, FormulaParsee},
};

/// Whether to print identifiers of expressions.
///
/// Useful for debugging, but should generally be disabled, as this is expected by [crate::tests].
pub(super) const PRINT_ID: bool = false; // todo: make configurable

/// Prefix for auxiliary variables.
///
/// Auxiliary variables are required by some algorithms on formulas and can be created with [Var::Aux].
pub(super) const VAR_AUX_PREFIX: &str = "_aux_"; // todo: make configurable (also whether aux vars should even be listed)

/// Returns the most recently parsed formula.
macro_rules! formula {
    ($formulas:expr) => {
        $formulas.last_mut().unwrap()
    };
}

/// Converts a formula into its clause representation, if not done yet.
macro_rules! clauses {
    ($clauses:expr, $arena:expr, $formulas:expr) => {{
        if $clauses.is_none() {
            $clauses = Some(formula!($formulas).to_clauses(&$arena));
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
    if commands.is_empty() {
        commands.push("-".to_string());
    }
    if commands.len() == 1 && File::exists(&commands[0]) {
        commands.push("to_cnf_dist".to_string());
        commands.push("to_clauses".to_string());
        commands.push("print".to_string());
    }
    for command in &commands {
        let mut arguments: Vec<&str> = command.split_whitespace().collect();
        let action = arguments[0];
        arguments.remove(0);
        match action {
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
            "to_cnf_tseitin" => {
                formula!(formulas).to_cnf_tseitin(true, &mut arena);
            }
            "to_clauses" => clauses = Some(formula!(formulas).to_clauses(&mut arena)),
            "satisfy" => println!("{}", clauses!(clauses, arena, formulas).satisfy().unwrap()),
            "count" => println!("{}", clauses!(clauses, arena, formulas).count(false).0),
            "assert_count" => {
                let clauses = clauses!(clauses, arena, formulas);
                formula!(formulas)
                    .file
                    .as_ref()
                    .unwrap()
                    .assert_count(clauses);
            }
            "enumerate" => clauses!(clauses, arena, formulas).enumerate(),
            "diff" => {
                assert!(formulas.len() == 2);
                assert!(arguments.len() <= 2);
                let a = &formulas[0];
                let b = &formulas[1];
                let mut arguments = arguments.into_iter();
                let command = match arguments.next() {
                    Some("count") => DiffCommand::Count,
                    Some("strict") => DiffCommand::Strict,
                    Some("weak") | _ => DiffCommand::Weak,
                };
                a.diff(b, command, arguments.next(), &mut arena);
            }
            _ => {
                if File::exists(action) {
                    let file = File::read(action);
                    let extension = file.extension();
                    formulas.push(arena.parse(file, parser(extension)));
                } else if SatInlineFormulaParser::can_parse(command) {
                    formulas.push(
                        // todo: what does this implement? a comparison as in Thüm 2009?
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
