//! Utilities for handling UVL, XML, and other files.

use crate::core::{
    arena::Arena,
    clauses::Clauses,
    expr::ExprId,
    file::File,
    var::{Var, VarId},
};
use std::{collections::HashSet, fmt::Write};

use super::exec;

/// Transforms a given name into a form compatible with FeatureIDE.
pub(crate) fn name_to_io(str: &str) -> String {
    str.replace("=", "__EQUALS__")
        .replace(":", "__COLON__")
        .replace(".", "__DOT__")
        .replace(",", "__COMMA__")
        .replace("/", "__SLASH__")
        .replace("\\", "__BACKSLASH__")
        .replace(" ", "__SPACE__")
        .replace("-", "__DASH__")
}

/// Retrieves a name from a given form compatible with FeatureIDE.
pub(crate) fn name_from_io(str: &str) -> String {
    str.replace("__EQUALS__", "=")
        .replace("__COLON__", ":")
        .replace("__DOT__", ".")
        .replace("__COMMA__", ",")
        .replace("__SLASH__", "/")
        .replace("__BACKSLASH__", "\\")
        .replace("__SPACE__", " ")
        .replace("__DASH__", "-")
}

/// Given a UVL file, only returns its feature hierarchy, omitting its constraints.
pub(crate) fn uvl_remove_constraints(uvl: &str) -> &str {
    uvl.split("\nconstraints\n").nth(0).unwrap().trim()
}

/// Given a UVL file with a feature hierarchy and given constraints, creates a merged UVL file.
pub(crate) fn uvl_with_constraints(uvl: &str, constraints: &str) -> String {
    format!("{}\n{}", uvl_remove_constraints(uvl), &constraints)
}

/// Appends variables as abstract features to an existing UVL feature hierarchy.
pub(crate) fn uvl_add_vars(label: &str, vars: &[Var]) -> String {
    let mut uvl = String::new();
    writeln!(uvl, "\t\tmandatory").unwrap();
    writeln!(uvl, "\t\t\t\"{label}\" {{abstract}}").unwrap();
    writeln!(uvl, "\t\t\t\toptional").unwrap();
    for var in vars {
        writeln!(uvl, "\t\t\t\t\t\"{var}\" {{abstract}}").unwrap();
    }
    uvl
}

/// Appends variables by their identifiers to an existing UVL file.
pub(crate) fn uvl_file_add_vars(
    file: &mut File,
    label: &str,
    var_ids: &HashSet<i32>,
    arena: &mut Arena,
) {
    let other_vars: Vec<Var> = arena
        .vars(|var_id, _| var_ids.contains(&var_id))
        .into_iter()
        .map(|(_, var)| var)
        .collect();
    file.contents = uvl_with_constraints(&file.contents, &uvl_add_vars(label, &other_vars));
}

/// Expresses a clause representation as a string of UVL features and constraints.
///
/// This string is to be appended to an existing UVL feature hierarchy.
pub(crate) fn to_uvl_string(clauses: &Clauses) -> String {
    let mut uvl = String::new();
    let vars: Vec<Var> = clauses.vars[1..]
        .iter()
        .filter(|var| match var {
            Var::Named(_) => false,
            Var::Aux(_) => true,
        })
        .map(|var| var.clone())
        .collect();
    writeln!(uvl, "{}", uvl_add_vars("Auxiliary Variables", &vars)).unwrap();
    writeln!(uvl, "\nconstraints").unwrap();
    for clause in &clauses.clauses {
        write!(uvl, "\t").unwrap();
        for literal in clause {
            let var: usize = literal.unsigned_abs().try_into().unwrap();
            write!(
                uvl,
                "{}\"{}\" | ",
                if *literal > 0 { "" } else { "!" },
                clauses.vars[var]
            )
            .unwrap();
        }
        if clause.len() > 0 {
            for _ in 1..=3 {
                uvl.pop();
            }
        } else {
            write!(uvl, "false").unwrap();
        }
        writeln!(uvl).unwrap();
    }
    uvl.truncate(uvl.trim_end().len());
    uvl
}

/// Given a XML file with a feature hierarchy and given constraints, creates a merged XML file.
pub(crate) fn xml_with_constraints(xml: &str, constraints: &str) -> String {
    xml.replace("</struct>", &format!("</struct>\n{}", constraints))
}

/// Expresses a clause representation as a string of XML constraints.
pub(crate) fn to_xml_string(clauses: &Clauses) -> String {
    let mut xml = "\t<constraints>\n".to_string();
    for clause in &clauses.clauses {
        write!(xml, "\t\t<rule><disj>").unwrap();
        for literal in clause {
            let var: usize = literal.unsigned_abs().try_into().unwrap();
            write!(
                xml,
                "{}<var>{}</var>{}",
                if *literal > 0 { "" } else { "<not>" },
                clauses.vars[var],
                if *literal > 0 { "" } else { "</not>" }
            )
            .unwrap();
        }
        writeln!(xml, "</disj></rule>").unwrap();
    }
    xml + "\t</constraints>"
}

/// Writes a UVL and XML file with a given prefix.
pub(crate) fn write_uvl_and_xml(
    prefix: String,
    uvl_features: &String,
    uvl_constraints: &String,
    xml_constraints: &String,
) {
    let uvl = uvl_with_constraints(uvl_features, uvl_constraints);
    let xml = xml_with_constraints(
        &exec::io(
            &File::new(
                "-.uvl".to_string(),
                uvl_remove_constraints(&uvl).to_string(),
            ),
            "xml",
            &[],
        )
        .contents,
        &xml_constraints,
    );
    File::new(format!("{prefix}.uvl"), uvl).write();
    File::new(format!("{prefix}.xml"), xml).write();
}

/// Writes variables to a file.
pub(crate) fn write_vars(name: String, arena: &Arena, var_ids: &HashSet<VarId>) {
    let mut f = String::new();
    for id in var_ids {
        let id: usize = (*id).try_into().unwrap();
        writeln!(f, "{}", arena.vars[id]).unwrap();
    }
    File::new(name, f).write();
}

/// Writes constraints to a file.
pub(crate) fn write_constraints(name: String, arena: &Arena, constraint_ids: &HashSet<ExprId>) {
    let mut f = String::new();
    for id in constraint_ids {
        writeln!(f, "{}", arena.as_formula(*id).as_ref(arena)).unwrap();
    }
    File::new(name, f).write();
}
