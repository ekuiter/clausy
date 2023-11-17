//! Imperative shell for operating on feature-model formulas.

use std::collections::HashSet;

use crate::core::clauses::Clauses;
use crate::core::expr::Expr;
use crate::core::var::{Var, VarId};
use crate::parser::sat_inline::SatInlineFormulaParser;
use crate::util::exec;
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
                let clauses = clauses!(clauses, arena, formulas);
                formula!(formulas)
                    .file
                    .as_ref()
                    .unwrap()
                    .assert_count(clauses);
            }
            "enumerate" => clauses!(clauses, arena, formulas).enumerate(),
            "compare" => {
                debug_assert!(formulas.len() == 2);
                let a = &formulas[0];
                let b = &formulas[1];
                println!("formula a has {} variables", a.sub_var_ids.len());
                println!("formula b has {} variables", b.sub_var_ids.len());

                let common_var_ids: HashSet<VarId> = a
                    .sub_var_ids
                    .intersection(&b.sub_var_ids)
                    .map(|var_id| *var_id)
                    .collect();

                let count_a = a.file.as_ref().unwrap().count_featureide();
                let count_b = b.file.as_ref().unwrap().count_featureide();
                println!("formula a has count {}", count_a);
                println!("formula b has count {}", count_b);

                let slice_a = a
                    .file
                    .as_ref()
                    .unwrap()
                    .slice_featureide(&mut arena, &common_var_ids);
                let slice_b = b
                    .file
                    .as_ref()
                    .unwrap()
                    .slice_featureide(&mut arena, &common_var_ids);

                {
                    let mut arena = arena.clone();
                    let mut slice = slice_a.clone();
                    slice.to_cnf_tseitin(&mut arena);
                    println!(
                        "slice a has count {}",
                        Clauses::from(slice.as_ref(&arena)).count()
                    );
                }
                {
                    let mut arena = arena.clone();
                    let mut slice = slice_b.clone();
                    slice.to_cnf_tseitin(&mut arena);
                    println!(
                        "slice b has count {}",
                        Clauses::from(slice.as_ref(&arena)).count()
                    );
                }

                // todo: do not clone arena, modify root id instead
                {
                    let mut arena = arena.clone();
                    let not = arena.expr(Expr::Not(slice_b.root_id));
                    let root_id = arena.expr(Expr::And(vec![slice_a.root_id, not]));
                    let mut tmp = Formula::new(common_var_ids.clone(), root_id, None);
                    tmp.to_cnf_tseitin(&mut arena);
                    println!("slice b removes {}", Clauses::from(tmp.as_ref(&arena)).count());
                }

                {
                    let mut arena = arena.clone();
                    let not = arena.expr(Expr::Not(slice_a.root_id));
                    let root_id = arena.expr(Expr::And(vec![slice_b.root_id, not]));
                    let mut tmp = Formula::new(common_var_ids.clone(), root_id, None);
                    tmp.to_cnf_tseitin(&mut arena);
                    println!("slice b adds {}", Clauses::from(tmp.as_ref(&arena)).count());
                }
                

                // let not_root_id_a = arena.expr(Expr::Not(a.root_id));
                // let root_id = arena.expr(Expr::And(vec![slice_a.root_id, not_root_id_a]));
                // let mut cmp_a_slice = Formula::new(a.sub_var_ids.clone(), root_id);
                // cmp_a_slice.to_cnf_tseitin(&mut arena);
                // let c = Clauses::from(cmp_a_slice.as_ref(&arena));
                // println!("{}", c.count());

                // let not_root_id_b = arena.expr(Expr::Not(b.root_id));
                // let root_id = arena.expr(Expr::And(vec![slice_b.root_id, not_root_id_b]));
                // let mut cmp_b_slice = Formula::new(b.sub_var_ids.clone(), root_id);
                // cmp_b_slice.to_cnf_tseitin(&mut arena);
                // let c = Clauses::from(cmp_b_slice.as_ref(&arena));
                // println!("{}", c.count());

                // let not = arena.expr(Expr::Not(slice_b.root_id));
                // let root_id = arena.expr(Expr::And(vec![slice_a.root_id, not]));
                // let mut cmp = Formula::new(common_var_ids.clone(), root_id);
                // cmp.to_cnf_tseitin(&mut arena);
                // let c = Clauses::from(cmp.as_ref(&arena));
                // println!("{}", c.count());
                return;
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
