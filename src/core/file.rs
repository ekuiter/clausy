//! Defines a feature-model formula file.

use std::{collections::HashSet, fs, io::Read, path::Path};

use num::BigInt;

use crate::{
    core::{clauses::Clauses, var::Var},
    parser::{parser, FormulaParsee},
    util::{exec, io},
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
            std::io::stdin()
                .read_to_string(&mut contents)
                .expect("failed to read formula contents from stdin");
        } else {
            contents = fs::read_to_string(name)
                .unwrap_or_else(|err| panic!("failed to read file '{}': {err}", name));
        };
        File::new(name.to_string(), contents)
    }

    /// Writes this file to its destination.
    pub(crate) fn write(&self) {
        fs::write(&self.name, &self.contents)
            .unwrap_or_else(|err| panic!("failed to write file '{}': {err}", self.name));
    }

    /// Returns the extension of this file, if any.
    pub(crate) fn extension(&self) -> Option<String> {
        Path::new(self.name.as_str())
            .extension()
            .map_or(None, |e| e.to_str())
            .map(|e| e.to_string())
    }

    /// Counts the number of solutions of the formula this file represents.
    ///
    /// Uses FeatureIDE's internal distributive CNF transformation directly on this file.
    /// The file extension must be given so FeatureIDE can detect the correct format.
    /// This function is useful as a baseline to ensure the correctness of clausy's transformations.
    /// However, it will not terminate for large formulas due to exponential blowup.
    pub(crate) fn count_with_featureide(&self) -> BigInt {
        exec::sharp_sat(&self.convert_with_featureide("cnf").contents)
    }

    /// Panics if the formula this file represents has a different model count than that of the given clauses.
    ///
    /// Useful for checking the correctness of count-preserving algorithms (e.g., [super::formula::Formula::to_cnf_tseitin]).
    /// Internally calls FeatureIDE as a transformation baseline.
    pub(crate) fn assert_count(&self, clauses: &Clauses) {
        let clausy_count = clauses.count();
        let featureide_count = self.count_with_featureide();
        assert_eq!(
            clausy_count,
            featureide_count,
            "error: clausy counts {} satisfying assignments, but FeatureIDE counts {}",
            clausy_count,
            featureide_count
        );
    }

    /// Converts this file into a given format, if necessary.
    pub(crate) fn convert_with_featureide(&self, output_format: &str) -> File {
        if self
            .extension()
            .filter(|extension| extension == output_format)
            .is_none()
        {
            exec::io(&self, output_format, &[])
        } else {
            self.clone()
        }
    }

    /// Slices the formula this file represents such that only the given variables remain.
    ///
    /// Internally, this uses FeatureIDE's slicing algorithm.
    /// Thus, it operates on an intermediate CNF representation created by distributive transformation.
    /// Consequently, it will not terminate for large formulas due to exponential blowup.
    pub(crate) fn slice_with_featureide(
        &self,
        var_ids: &HashSet<VarId>,
        arena: &mut Arena,
        uvl: bool,
    ) -> (Formula, Option<File>) {
        let vars = var_ids
            .iter()
            .map(|var_id| {
                let var_id_usize: usize = var_id
                    .unsigned_abs()
                    .try_into()
                    .expect("variable id does not fit into usize");
                if let Var::Named(name) = &arena.vars[var_id_usize] {
                    io::name_to_io(name)
                } else {
                    panic!(
                        "FeatureIDE slicing requires named variables, found auxiliary variable id {} while converting file '{}'",
                        var_id,
                        self.name
                    )
                }
            })
            .collect::<Vec<String>>();
        let vars = vars.iter().map(|s| &**s).collect::<Vec<&str>>();
        let slice = exec::io(&self, "cnf", &vars);
        let mut uvl_file = None;
        if uvl {
            uvl_file = Some(exec::io(&self, "uvl", &vars));
        }
        let slice = Self::new("-.cnf".to_string(), io::name_from_io(&slice.contents));
        let mut formula = arena.parse(slice, parser(Some("cnf".to_string())));
        formula.sub_var_ids = var_ids.clone();
        (formula, uvl_file)
    }
}
