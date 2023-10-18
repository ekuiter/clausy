//! Defines variables in an arena.

use std::fmt;

use crate::shell::VAR_AUX_PREFIX;

/// Identifier type for variables.
///
/// Serves as an index into [Formula::vars].
/// We also use this type to represent literals in [crate::core::clauses::Clauses], therefore we use a signed type.
/// Also, we do not expect too many variables, so a 32-bit integer should suffice.
pub(crate) type VarId = i32;

/// A variable in an arena.
///
/// Variables can either be named or auxiliary.
/// Named variables refer to a string, which represents their name.
/// Some algorithms on formulas (e.g., [Formula::to_cnf_tseitin]) require creating new, auxiliary variables.
/// As these variables are anonymous and have no designated meaning in the feature-modeling domain, we assign them arbitrary numbers.
/// To avoid creating unnecessary strings, we store these as native numbers.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub(crate) enum Var {
    /// A named variable.
    Named(String),

    /// An auxiliary variable.
    Aux(u32),
}

/// Displays a formula.
impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Var::Named(name) => write!(f, "{name}"),
            Var::Aux(id) => write!(f, "{}{id}", VAR_AUX_PREFIX),
        }
    }
}