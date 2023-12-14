//! Defines a feature-model formula file.

use std::{collections::HashSet, io::Read, path::Path, fs};

use num::BigInt;

use crate::{
    core::{clauses::Clauses, var::Var},
    parser::{parser, FormulaParsee},
    util::exec,
};

use super::{arena::Arena, formula::Formula, var::VarId};

/// A feature-model formula file.
///
/// Every [Formula] may be parsed from an existing input [File].
#[derive(Clone)]
pub(crate) struct File {
    /// The name of the file the associated formula was originally parsed from.
    pub(crate) name: String,

    /// The contents of the file the associated formula was originally parsed from.
    pub(crate) contents: String,
}

impl File {
    /// Creates a new file.
    pub(crate) fn new(name: String, contents: String) -> Self {
        Self { name, contents }
    }

    /// Returns whether a file exists at a given path.
    ///
    /// Also allows the special value - for referring to standard input.
    pub(crate) fn exists(file_name: &str) -> bool {
        Path::new(file_name).exists() || file_name.starts_with("-")
    }

    /// Reads the contents and extension of a file.
    pub(crate) fn read(name: &str) -> File {
        let mut contents;
        if name.starts_with("-") {
            contents = String::new();
            std::io::stdin().read_to_string(&mut contents).unwrap();
        } else {
            contents = fs::read_to_string(name).unwrap();
        };
        File::new(name.to_string(), contents)
    }

    /// Returns the extension of this file, if any.
    pub(crate) fn extension(&self) -> Option<String> {
        Path::new(self.name.as_str())
            .extension()
            .map_or(None, |e| e.to_str())
            .map(|e| e.to_string())
    }

    /// Counts the number of solutions of the formula this file represents using FeatureIDE.
    ///
    /// The file extension must be given so FeatureIDE can detect the correct format.
    pub(crate) fn count_featureide(&self) -> BigInt {
        exec::d4(&exec::io(self, "dimacs", &[]).contents)
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
    pub(crate) fn slice_featureide(&self, var_ids: &HashSet<VarId>, arena: &mut Arena) -> Formula {
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
        let slice = exec::io(&self, "dimacs", &[]);
        let slice = exec::io(&slice, "sat", &vars);
        let slice = Self::new("-.sat".to_string(), exec::name_from_io(&slice.contents));
        let mut formula = arena.parse(slice, parser(Some("sat".to_string())));
        formula.sub_var_ids = var_ids.clone();
        formula
    }
}
