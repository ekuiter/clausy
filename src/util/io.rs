//! Utilities for handling UVL and XML files.

use crate::core::{clauses::Clauses, var::Var};
use std::fmt::Write;

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

/// Expresses a clause representation as a string of UVL features and constraints.
///
/// This string is to be appended to an existing UVL feature hierarchy.
pub(crate) fn to_uvl_string(clauses: &Clauses) -> String {
    let mut uvl = "".to_string();
    write!(uvl, "\t\tmandatory\n").unwrap();
    write!(uvl, "\t\t\t\"Auxiliary Variables\" {{abstract}}\n").unwrap();
    write!(uvl, "\t\t\t\toptional\n").unwrap();
    for (i, var) in clauses.vars.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if let Var::Aux(_) = var {
            write!(uvl, "\t\t\t\t\t\"{var}\" {{abstract}}\n").unwrap();
        }
    }
    write!(uvl, "\nconstraints\n").unwrap();
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
        write!(uvl, "\n").unwrap();
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
        write!(xml, "</disj></rule>\n").unwrap();
    }
    xml + "\t</constraints>"
}
