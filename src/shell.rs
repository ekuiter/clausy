//! Imperative shell for operating on feature-model formulas.

use std::ops::Div;
use std::str::FromStr;

use num::BigRational;
use num::{bigint::ToBigInt, BigInt, ToPrimitive};

use crate::parser::sat_inline::SatInlineFormulaParser;
use crate::{
    core::{arena::Arena, formula::Formula},
    parser::{parser, FormulaParsee},
    util::{file_exists, read_contents},
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

    if commands.len() == 1 && file_exists(&commands[0]) {
        commands.push("to_cnf_dist".to_string());
        commands.push("to_clauses".to_string());
        commands.push("print".to_string());
    }

    for command in &commands {
        let parts: Vec<&str> = command.split_whitespace().collect();
        match parts[0] {
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
            "count" => println!("{}", clauses!(clauses, arena, formulas).count()),
            "assert_count" => {
                let clauses = clauses!(clauses, arena, formulas);
                formula!(formulas)
                    .file
                    .as_ref()
                    .unwrap()
                    .assert_count(clauses);
            }
            "enumerate" => clauses!(clauses, arena, formulas).enumerate(),
            "count_diff" => {
                debug_assert!(formulas.len() == 2);
                debug_assert!(parts.len() == 2);
                let a = &formulas[0];
                let b = &formulas[1];
                let (a2_to_a, a_vars, common, removed, added, b_vars, b2_to_b) =
                    a.count_diff(b, true, &mut arena);
                let all = &common + &removed + &added;
                let common_ratio = BigRational::new(common.clone(), all.clone()).to_f64().unwrap();
                let removed_ratio = BigRational::new(removed.clone(), all.clone()).to_f64().unwrap();
                let added_ratio = BigRational::new(added.clone(), all.clone()).to_f64().unwrap();
                match parts[1] {
                    "csv" => println!("{a2_to_a},{a_vars},{common},{removed},{added},{b_vars},{b2_to_b},{},{},{}", common_ratio, removed_ratio, added_ratio),
                    "bc" => println!("(((#+{a2_to_a})/2^{a_vars})-{removed}+{added})*2^{b_vars}-{b2_to_b}# | sed 's/#/<left model count>/' | bc"),
                    count_a => {
                        let count_a = BigInt::from_str(count_a).unwrap();
                        let two = 2.to_bigint().unwrap();
                        println!(
                            "{}",
                            (((&count_a + &a2_to_a) / two.pow(a_vars)) - &removed + &added)
                                * two.pow(b_vars)
                                - &b2_to_b
                        );
                    }
                }
            }
            _ => {
                if file_exists(command) {
                    let (file, extension) = read_contents(command);
                    formulas.push(arena.parse(&file, parser(extension.clone())));
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
