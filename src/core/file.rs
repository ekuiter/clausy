//! Defines a feature-model formula file.

use std::collections::HashSet;

use num_bigint::BigUint;

use crate::{
    core::{
        clauses::Clauses,
        var::Var,
    },
    parser::{parser, FormulaParsee},
    util::exec,
};

use super::{arena::Arena, formula::Formula, var::VarId};

/// The contents of a feature-model formula file.
///
/// Every [Formula] may be parsed from an existing input [File].
#[derive(Clone)]
pub(crate) struct File {
    /// The contents of the file the associated formula was originally parsed from.
    pub(crate) contents: String,

    /// The extension of the file the associated formula was originally parsed from, if any.
    pub(crate) extension: Option<String>,
}

impl File {
    /// Creates a new file.
    pub(crate) fn new(contents: String, extension: Option<String>) -> Self {
        Self {
            contents,
            extension,
        }
    }

    /// Counts the number of solutions of the formula this file represents using FeatureIDE.
    ///
    /// The file extension must be given so FeatureIDE can detect the correct format.
    pub(crate) fn count_featureide(&self) -> BigUint {
        exec::d4(&exec::io(
            self.contents.as_str(),
            self.extension.as_ref().unwrap(),
            "dimacs",
            &[],
        ))
    }

    /// Panics if the formula this file represents has a different model count than that of the given clauses.
    ///
    /// Useful for checking the correctness of count-preserving algorithms (e.g., [super::formula::Formula::to_cnf_tseitin]).
    pub(crate) fn assert_count(&self, clauses: &Clauses) {
        assert_eq!(clauses.count(), self.count_featureide());
    }

    /// Slices the formula this file represents such that only the given variables remain.
    ///
    /// Internally, this uses FeatureIDE, so it operates on an intermediate CNF representation created by distributive transformation.
    pub(crate) fn slice_featureide(&self, arena: &mut Arena, var_ids: &HashSet<VarId>) -> Formula {
        let vars = var_ids
            .iter()
            .map(|var_id| {
                let var_id: usize = var_id.unsigned_abs().try_into().unwrap();
                if let Var::Named(name) = &arena.vars[var_id] {
                    exec::name_to_io(name)
                } else {
                    unreachable!()
                }
            })
            .collect::<Vec<String>>();
        let vars = vars.iter().map(|s| &**s).collect::<Vec<&str>>();
        let slice = exec::io(
            &self.contents.as_str(),
            self.extension.as_ref().unwrap(),
            "sat",
            &vars,
        );
        let slice = exec::name_from_io(&slice);
        let formula = arena.parse(&slice, parser(Some("sat".to_string())));
        assert!(var_ids
            .symmetric_difference(&formula.sub_var_ids)
            .next()
            .is_none());
        formula
    }
}
