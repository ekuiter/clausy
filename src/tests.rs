//! Unit tests for parser and core formula/arena behavior.

use std::collections::HashSet;

use crate::{
    core::{
        arena::Arena,
        expr::Expr::*,
        file::File,
        formula::Formula,
        var::{Var, VarId},
    },
    parser::{parser, sat_inline::SatInlineFormulaParser, FormulaParsee},
};

fn parse_into(arena: &mut Arena, name: &str, contents: &str, extension: &str) -> Formula {
    arena.parse(
        File::new(name.to_string(), contents.to_string()),
        parser(Some(extension.to_string())),
    )
}

fn parse_new(name: &str, contents: &str, extension: &str) -> (Arena, Formula) {
    let mut arena = Arena::new();
    let formula = parse_into(&mut arena, name, contents, extension);
    (arena, formula)
}

fn formula_string(formula: &Formula, arena: &Arena) -> String {
    formula.as_ref(arena).to_string()
}

fn parse_model_new(name: &str, contents: &str) -> (Arena, Formula) {
    parse_new(name, contents, "model")
}

fn parse_sat_new(name: &str, contents: &str) -> (Arena, Formula) {
    parse_new(name, contents, "sat")
}

fn parse_cnf_new(name: &str, contents: &str) -> (Arena, Formula) {
    parse_new(name, contents, "cnf")
}

#[test]
// Checks canonicalization for commutative operators.
// We expect `Or(a, b)` and `Or(b, a)` to resolve to the same expression id because the arena sorts/deduplicates operands.
fn arena_expr_deduplicates_commutative_operands() {
    let mut arena = Arena::new();
    let a = arena.var_expr("a".to_string());
    let b = arena.var_expr("b".to_string());
    let or_ab = arena.expr(Or(vec![a, b]));
    let or_ba = arena.expr(Or(vec![b, a]));

    assert_eq!(or_ab, or_ba);
}

#[test]
// Checks simplification of double negation.
// We expect `Not(Not(a))` to collapse to `a` because the arena removes double negation in `simp_expr`.
fn arena_expr_simplifies_double_negation() {
    let mut arena = Arena::new();
    let a = arena.var_expr("a".to_string());
    let not_a = arena.expr(Not(a));
    let not_not_a = arena.expr(Not(not_a));

    assert_eq!(a, not_not_a);
}

#[test]
// Checks SAT comment-variable mapping.
// We expect DIMACS ids to be parsed as named variables because comment lines starting with `c` bind ids to names.
fn sat_parser_uses_comment_variable_names() {
    let sat = "c 1 feature_a\nc 2 feature_b\np sat 2 *(1 -(2))";
    let (arena, formula) = parse_sat_new("test.sat", sat);

    assert_eq!(
        formula_string(&formula, &arena),
        "And(feature_a, Not(feature_b))"
    );
}

#[test]
// Checks CNF parser structure for clauses with positive/negative literals.
// We expect an `And` with one disjunction and one unit clause because each CNF line becomes one clause.
fn cnf_parser_builds_expected_formula_structure() {
    let cnf = "c 1 a\nc 2 b\np cnf 2 2\n1 -2 0\n2 0\n";
    let (arena, formula) = parse_cnf_new("test.cnf", cnf);
    let rendered = formula_string(&formula, &arena);

    assert_eq!(rendered, "And(b, Or(a, Not(b)))");
}

#[test]
// Checks CNF parser compatibility with projection comments after clauses.
// We expect trailing `c p show ...` lines to be ignored and clause parsing to remain unchanged.
fn cnf_parser_accepts_trailing_projection_comment_line() {
    let cnf = "p cnf 2 2\n1 -2 0\n2 0\nc p show 1 2 0\n";
    let (arena, formula) = parse_cnf_new("test.dimacs", cnf);
    let rendered = formula_string(&formula, &arena);

    assert_eq!(rendered, "And(2, Or(1, Not(2)))");
}

#[test]
// Checks model parser support for `<unsupported>` (introduced by torte on KClause extraction).
// We expect it to survive as a placeholder variable so unsupported constructs remain represented unchanged.
fn model_parser_handles_unsupported_literal() {
    let model = "# comment\n(def(a)&<unsupported>)\n";
    let (arena, formula) = parse_model_new("test.model", model);

    assert_eq!(formula_string(&formula, &arena), "And(a, <unsupported>)");
}

#[test]
// Checks inline SAT parser references (and simplifies) previous formulas correctly.
// We expect `+(1 -2)` to combine formula #1 and negated formula #2, yielding `Or(a, b)`.
fn sat_inline_parser_can_reference_previous_formulas() {
    let mut arena = Arena::new();
    let f1 = parse_into(&mut arena, "a.sat", "c 1 a\np sat 1 1", "sat");
    let f2 = parse_into(&mut arena, "b.sat", "c 1 b\np sat 1 -(1)", "sat");

    let formulas = vec![f1.clone(), f2.clone()];
    let inline =
        SatInlineFormulaParser::new(&formulas, None).parse_into(&"+(1 -2)".to_string(), &mut arena);

    assert_eq!(formula_string(&inline, &arena), "Or(a, b)");
}

#[test]
// Checks NNF conversion pushes negation to leaves.
// We expect `-(*(1 2))` to become `Or(Not(a), Not(b))` by De Morgan.
fn to_nnf_pushes_negations_to_leaves() {
    let sat = "c 1 a\nc 2 b\np sat 2 -(*(1 2))";
    let (mut arena, mut formula) = parse_sat_new("nnf.sat", sat);

    formula.to_nnf(&mut arena);

    assert_eq!(formula_string(&formula, &arena), "Or(Not(a), Not(b))");
}

#[test]
// Checks distributive CNF conversion.
// We expect `Or(And(a,b), c)` to become `And(Or(a,c), Or(b,c))` because OR is distributed over AND.
fn to_cnf_dist_distributes_or_over_and() {
    let sat = "c 1 a\nc 2 b\nc 3 c\np sat 3 +(*(1 2) 3)";
    let (mut arena, mut formula) = parse_sat_new("cnf_dist.sat", sat);

    formula.to_cnf_dist(&mut arena);

    assert_eq!(formula_string(&formula, &arena), "And(Or(a, c), Or(b, c))");
}

#[test]
// Checks Tseitin CNF introduces auxiliary variables.
// We expect more sub-variables and at least one `Var::Aux` because complex subexpressions get fresh helper vars.
fn to_cnf_tseitin_adds_auxiliary_variables() {
    let sat = "c 1 a\nc 2 b\nc 3 c\np sat 3 +(*(1 2) 3)";
    let (mut arena, mut formula) = parse_sat_new("cnf_tseitin.sat", sat);
    let before = formula.sub_var_ids.len();

    formula.to_cnf_tseitin(true, &mut arena);

    assert!(formula.sub_var_ids.len() > before);
    let has_aux = formula.sub_var_ids.iter().any(|id| {
        let idx: usize = (*id)
            .try_into()
            .expect("sub-variable id should fit into usize");
        matches!(arena.vars[idx], Var::Aux(_))
    });
    assert!(has_aux);
}

#[test]
// Checks selective constraint removal.
// We expect constraints mentioning `b` to be dropped, leaving only `a`, because `remove_constraints` filters by variable usage.
fn remove_constraints_drops_constraints_with_removed_variables() {
    let sat = "c 1 a\nc 2 b\np sat 2 *(+(1 2) 1)";
    let (mut arena, formula) = parse_sat_new("remove.sat", sat);

    let b_id: VarId = arena
        .get_var_named("b".to_string())
        .expect("named variable 'b' should exist");

    let mut remove_ids = HashSet::new();
    remove_ids.insert(b_id);
    let reduced = formula.remove_constraints(&remove_ids, &mut arena);

    assert_eq!(formula_string(&reduced, &arena), "a");
}

#[test]
// Checks diff partitioning on variables and constraints.
// We expect one common and one side-specific element per side because formulas share one part and differ in one part.
fn diff_vars_and_constraints_report_expected_partition() {
    let mut arena = Arena::new();
    let a = parse_into(
        &mut arena,
        "left.sat",
        "c 1 a\nc 2 b\np sat 2 *(1 +(1 2))",
        "sat",
    );
    let b = parse_into(
        &mut arena,
        "right.sat",
        "c 1 a\nc 2 c\np sat 2 *(1 +(1 2))",
        "sat",
    );

    let (common_vars, left_vars, right_vars) = a.diff_vars(&b);
    assert_eq!(common_vars.len(), 1);
    assert_eq!(left_vars.len(), 1);
    assert_eq!(right_vars.len(), 1);

    let (common_constraints, left_constraints, right_constraints) =
        a.diff_constraints(&b, &mut arena);
    assert_eq!(common_constraints.len(), 1);
    assert_eq!(left_constraints.len(), 1);
    assert_eq!(right_constraints.len(), 1);
}

#[test]
// Checks DIMACS serialization basics.
// We expect a CNF header and `0`-terminated clause lines because DIMACS requires explicit clause termination.
fn clauses_render_dimacs_with_zero_terminated_clauses() {
    let sat = "c 1 a\nc 2 b\np sat 2 *(+(1 -2) 2)";
    let (arena, formula) = parse_sat_new("clauses.sat", sat);
    let dimacs = formula.to_clauses(&arena).to_string();

    assert!(dimacs.contains("p cnf 2 2"));

    let clause_lines: Vec<&str> = dimacs
        .lines()
        .filter(|line| !line.starts_with("c ") && !line.starts_with("p "))
        .collect();
    assert_eq!(clause_lines.len(), 2);
    assert!(clause_lines.iter().all(|line| line.ends_with(" 0")));
}

#[test]
#[should_panic]
// Checks proto-CNF guard rejects non-conjunctive roots.
// We expect panic for a literal root because proto-CNF requires a non-empty top-level conjunction.
fn assert_proto_cnf_panics_on_literal_formula() {
    let (arena, formula) = parse_sat_new("literal.sat", "c 1 a\np sat 1 1");
    formula.assert_proto_cnf(&arena);
}

#[test]
#[should_panic]
// Checks proto-CNF guard rejects empty conjunctions.
// We expect panic because an empty `And` is disallowed by the precondition check.
fn assert_proto_cnf_panics_on_empty_conjunction() {
    let mut arena = Arena::new();
    let root = arena.expr(And(vec![]));
    let formula = Formula::new(arena.var_ids(), root, None);
    formula.assert_proto_cnf(&arena);
}

#[test]
// Checks parser regression scenario with comments/spacing/nesting.
// We expect all components to be preserved semantically despite formatting noise.
fn parser_simple_legacy_scenario() {
    let model = "# comment
            (def(x)|def(y))
            # coaoeu
            def( ab )
            !def(n)
            (def( abc)& !(def(x)|def(y))   & def( bb ))";
    let (arena, formula) = parse_model_new("legacy.model", model);
    let rendered = formula_string(&formula, &arena);
    assert_eq!(
        rendered,
        "And(Or(x, y), ab, Not(n), And(Not(Or(x, y)), abc, bb))"
    );
}

#[test]
// Checks NNF base case.
// We expect `!def(a)` to remain `Not(a)` because negation is already at leaf level.
fn nnf_not_a() {
    let (mut arena, mut formula) = parse_model_new("not_a.model", "!def(a)");
    formula.to_nnf(&mut arena);
    assert_eq!(formula_string(&formula, &arena), "Not(a)");
}

#[test]
// Checks NNF double-negation case.
// We expect `!!def(a)` to simplify to `a` due to negation normalization.
fn nnf_not_not_a() {
    let (mut arena, mut formula) = parse_model_new("not_not_a.model", "!!def(a)");
    formula.to_nnf(&mut arena);
    assert_eq!(formula_string(&formula, &arena), "a");
}

#[test]
// Checks NNF idempotence check.
// We expect identical output after a second `to_nnf` call because NNF transform should be stable.
fn nnf_complex_is_idempotent() {
    let model = "((!def(a))&(((def(c)|!def(a)))|((def(a))&(def(c)|!(def(a)|def(b))))))";
    let (mut arena, mut formula) = parse_model_new("nnf_complex.model", model);
    formula.to_nnf(&mut arena);
    let once = formula_string(&formula, &arena);
    formula.to_nnf(&mut arena);
    let twice = formula_string(&formula, &arena);
    assert_eq!(once, twice);
}

#[test]
// Checks shared-subexpression case for NNF.
// We expect successful conversion and printable output.
fn nnf_shared_expression_no_panic() {
    let model = "((((!(def(a))&def(a)))&(!(!(def(a))&def(a))))&((!(!(def(a))&def(a)))&(!((def(a))&def(a)))))";
    let (mut arena, mut formula) = parse_model_new("nnf_shared.model", model);
    formula.to_nnf(&mut arena);
    let printed = formula_string(&formula, &arena);
    assert_eq!(printed, "And(Not(a), Or())");
}

#[test]
// Checks simple distributive CNF shape.
// We expect `And(a, Or(a, b))` after CNF conversion because the input is equivalent to that normal form.
fn cnf_dist_simple_legacy_shape() {
    let sat = "c 1 a\nc 2 b\np sat 2 *(1 +(1 2))";
    let (mut arena, mut formula) = parse_sat_new("cnf_simple.sat", sat);
    formula.to_cnf_dist(&mut arena);
    assert_eq!(formula_string(&formula, &arena), "And(a, Or(a, b))");
}

#[test]
// Checks CNF idempotence check.
// We expect unchanged output after reapplying `to_cnf_dist` because the formula is already in canonical CNF.
fn cnf_dist_idempotent() {
    let sat = "c 1 a\nc 2 b\nc 3 c\np sat 3 *(+(1 2) +(1 3))";
    let (mut arena, mut formula) = parse_sat_new("cnf_idempotent.sat", sat);
    formula.to_cnf_dist(&mut arena);
    let once = formula_string(&formula, &arena);
    formula.to_cnf_dist(&mut arena);
    let twice = formula_string(&formula, &arena);
    assert_eq!(once, twice);
}

#[test]
// Checks shared-subexpression case for CNF distribution.
// We expect no panic and non-empty output because distribution handles shared nodes and simplification safely.
fn cnf_dist_shared_expression_no_panic() {
    let sat = "c 1 a\np sat 1 *(-(1) -(-(1)))";
    let (mut arena, mut formula) = parse_sat_new("cnf_shared.sat", sat);
    formula.to_cnf_dist(&mut arena);
    let printed = formula_string(&formula, &arena);
    assert_eq!(printed, "Or()");
}

#[test]
// Checks CNF rendering.
// We expect DIMACS metadata and clause terminators to ensure serialized output remains solver-compatible.
fn cnf_legacy_simple_line_shape() {
    let sat =
        "c 1 x\nc 2 y\nc 3 ab\nc 4 n\nc 5 abc\nc 6 bb\np sat 6 *(+(1 2) 3 -(4) *(5 -(+(1 2)) 6))";
    let (mut arena, mut formula) = parse_sat_new("legacy_cnf.sat", sat);
    formula.to_cnf_dist(&mut arena);
    let dimacs = formula.to_clauses(&arena).to_string();
    assert_eq!(
        dimacs,
        "c 1 x\nc 2 y\nc 3 ab\nc 4 n\nc 5 abc\nc 6 bb\np cnf 6 7\n-1 0\n-2 0\n3 0\n-4 0\n5 0\n6 0\n1 2 0\n"
    );
}

#[test]
// Checks canonicalization flattens nested same-kind operators.
// We expect `And(And(a, b), c)` to become `And(a, b, c)` after `to_canon`.
fn to_canon_flattens_nested_ands() {
    let mut arena = Arena::new();
    let a = arena.var_expr("a".to_string());
    let b = arena.var_expr("b".to_string());
    let c = arena.var_expr("c".to_string());
    let inner = arena.expr(And(vec![a, b]));
    let root = arena.expr(And(vec![inner, c]));
    let mut formula = Formula::new(arena.var_ids(), root, None);

    formula.to_canon(&mut arena);

    assert_eq!(formula_string(&formula, &arena), "And(a, b, c)");
}

#[test]
// Checks forcing foreign variables as assumptions while excluding selected vars.
// We expect all non-sub vars except exclusions to be added as literals and included in sub_var_ids.
fn force_foreign_vars_respects_exclusions() {
    let mut arena = Arena::new();
    let a_formula = parse_into(&mut arena, "a.sat", "c 1 a\np sat 1 1", "sat");
    let _b_expr = arena.var_expr("b".to_string());
    let _c_expr = arena.var_expr("c".to_string());
    let b_id = arena
        .get_var_named("b".to_string())
        .expect("named variable 'b' should exist");
    let c_id = arena
        .get_var_named("c".to_string())
        .expect("named variable 'c' should exist");

    let mut exclude_ids = HashSet::new();
    exclude_ids.insert(c_id);
    let forced = a_formula.force_foreign_vars(false, &exclude_ids, &mut arena);

    assert_eq!(formula_string(&forced, &arena), "And(a, Not(b))");
    assert!(forced.sub_var_ids.contains(&b_id));
    assert!(!forced.sub_var_ids.contains(&c_id));
}

#[test]
// Checks variable-set operations and boolean composition helpers.
// We expect disjoint vars to be partitioned correctly and composition operators to build expected formulas.
fn vars_set_ops_and_formula_composition_behave_as_expected() {
    let mut arena = Arena::new();
    let left = parse_into(&mut arena, "left.sat", "c 1 a\np sat 1 1", "sat");
    let right = parse_into(&mut arena, "right.sat", "c 1 b\np sat 1 1", "sat");

    let common = left.common_vars(&right);
    let all = left.all_vars(&right);
    let left_only = left.except_vars(&right);
    let right_only = right.except_vars(&left);
    assert!(common.is_empty());
    assert_eq!(all.len(), 2);
    assert_eq!(left_only.len(), 1);
    assert_eq!(right_only.len(), 1);

    let conjunction = left.and(&right, &mut arena);
    let implication = left.implies(&right, &mut arena);
    assert_eq!(formula_string(&conjunction, &arena), "And(a, b)");
    assert_eq!(formula_string(&implication, &arena), "And(a, Not(b))");
}

#[test]
// Checks clause conversion edge-case for contradiction with no variables.
// We expect one empty clause (`0`) so the result is unsatisfiable in DIMACS.
fn clauses_render_empty_contradiction_without_variables() {
    let mut arena = Arena::new();
    let root = arena.expr(Or(vec![]));
    let formula = Formula::new(HashSet::new(), root, None);

    let dimacs = formula.to_clauses(&arena).to_string();

    assert_eq!(dimacs, "p cnf 0 1\n0\n");
}

#[test]
// Checks clause conversion edge-case for tautology with no variables.
// We expect a DIMACS header with zero vars/clauses and no clause lines.
fn clauses_render_empty_tautology_without_variables() {
    let mut arena = Arena::new();
    let root = arena.expr(And(vec![]));
    let formula = Formula::new(HashSet::new(), root, None);

    let dimacs = formula.to_clauses(&arena).to_string();

    assert_eq!(dimacs, "p cnf 0 0\n");
}
