//! Utilities for handling UVL, XML, and other files.

use crate::core::{
    arena::Arena,
    clauses::Clauses,
    expr::ExprId,
    file::File,
    formula::Formula,
    var::{Var, VarId},
};
use std::{collections::HashSet, fmt::Write};

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
    uvl.split("\nconstraints\n")
        .next()
        .expect("failed to split UVL file while removing constraints")
        .trim()
}

/// Given a UVL file with a feature hierarchy and given features or constraints, creates a merged UVL file.
pub(crate) fn uvl_append(uvl: &str, appendix: &str) -> String {
    format!("{}\n{}", uvl_remove_constraints(uvl), &appendix)
}

/// Appends variables as abstract features to an existing UVL feature hierarchy.
pub(crate) fn uvl_add_vars(label: &str, vars: &[Var]) -> String {
    let mut uvl = String::new();
    writeln!(uvl, "\t\tmandatory").expect("failed to build UVL: writing mandatory section");
    writeln!(uvl, "\t\t\t\"{label}\" {{abstract}}")
        .expect("failed to build UVL: writing variable group label");
    writeln!(uvl, "\t\t\t\toptional").expect("failed to build UVL: writing optional block");
    for var in vars {
        writeln!(uvl, "\t\t\t\t\t\"{var}\" {{abstract}}")
            .expect("failed to build UVL: writing variable entry");
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
    file.contents = uvl_append(&file.contents, &uvl_add_vars(label, &other_vars));
}

/// Expresses a clause representation as a string of UVL features and constraints compatible with FeatureIDE.
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
    writeln!(uvl, "{}", uvl_add_vars("Auxiliary Variables", &vars))
        .expect("failed to build UVL: writing auxiliary variable section");
    writeln!(uvl, "\nconstraints").expect("failed to build UVL: writing constraints header");
    for clause in &clauses.clauses {
        write!(uvl, "\t").expect("failed to build UVL: writing clause indentation");
        for literal in clause {
            let var: usize = literal
                .unsigned_abs()
                .try_into()
                .expect("clause literal index does not fit into usize");
            write!(
                uvl,
                "{}\"{}\" | ",
                if *literal > 0 { "" } else { "!" },
                clauses.vars[var]
            )
            .expect("failed to build UVL: writing clause literal");
        }
        if clause.len() > 0 {
            for _ in 1..=3 {
                uvl.pop();
            }
        } else {
            write!(uvl, "false").expect("failed to build UVL: writing empty-clause marker");
        }
        writeln!(uvl).expect("failed to build UVL: terminating clause line");
    }
    uvl.truncate(uvl.trim_end().len());
    uvl
}

/// Given an XML file with a feature hierarchy and given constraints, creates a merged XML file.
pub(crate) fn xml_with_constraints(xml: &str, constraints: &str) -> String {
    xml.replace("</struct>", &format!("</struct>\n{}", constraints))
}

/// Expresses a clause representation as a string of XML constraints compatible with FeatureIDE.
pub(crate) fn to_xml_string(clauses: &Clauses) -> String {
    let mut xml = "\t<constraints>\n".to_string();
    for clause in &clauses.clauses {
        write!(xml, "\t\t<rule><disj>").expect("failed to build XML: writing rule header");
        for literal in clause {
            let var: usize = literal
                .unsigned_abs()
                .try_into()
                .expect("clause literal index does not fit into usize");
            write!(
                xml,
                "{}<var>{}</var>{}",
                if *literal > 0 { "" } else { "<not>" },
                clauses.vars[var],
                if *literal > 0 { "" } else { "</not>" }
            )
            .expect("failed to build XML: writing clause literal");
        }
        writeln!(xml, "</disj></rule>").expect("failed to build XML: terminating rule");
    }
    xml + "\t</constraints>"
}

/// Writes a UVL file with a given prefix.
pub(crate) fn write_uvl(prefix: String, uvl_features: &String, uvl_constraints: &String) {
    let uvl = uvl_append(uvl_features, uvl_constraints);
    File::new(format!("{prefix}.uvl"), uvl).write();
}

/// Writes an XML file with a given prefix.
pub(crate) fn write_xml(prefix: String, uvl_features: &String, xml_constraints: &String) {
    let uvl = uvl_remove_constraints(uvl_features);
    let xml = xml_with_constraints(
        &File::new("-.uvl".to_string(), uvl.to_string())
            .convert_with_featureide("xml")
            .contents,
        xml_constraints,
    );
    File::new(format!("{prefix}.xml"), xml).write();
}

/// Writes a formula to a `.txt` file.
pub(crate) fn write_formula(path: &str, formula: &Formula, proj_vars: Option<&HashSet<VarId>>, arena: &Arena) {
    std::fs::write(
        &path,
        format!("{}\n{:?}\n{:?}", formula.as_ref(arena), formula.sub_vars(arena), proj_vars),
    )
    .unwrap_or_else(|e| panic!("failed to write formula to '{path}': {e}"));
}

/// Writes variables to a file.
pub(crate) fn write_vars(name: String, arena: &Arena, var_ids: &HashSet<VarId>) {
    let mut f = String::new();
    for id in var_ids {
        let id: usize = (*id)
            .try_into()
            .expect("variable id does not fit into usize");
        writeln!(f, "{}", arena.vars[id]).expect("failed to write variable to output buffer");
    }
    File::new(name, f).write();
}

/// Writes constraints to a file.
pub(crate) fn write_constraints(name: String, arena: &Arena, constraint_ids: &HashSet<ExprId>) {
    let mut f = String::new();
    for id in constraint_ids {
        writeln!(f, "{}", arena.as_formula(*id).as_ref(arena))
            .expect("failed to write constraint to output buffer");
    }
    File::new(name, f).write();
}
