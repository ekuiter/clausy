//! Parser for any file format accepted by FeatureIDE.

use super::{sat::SatFormulaParser, FormulaParser};
use crate::{
    core::{arena::Arena, formula::Formula, file::File},
    util,
};

/// Parses feature-model formula files in any file format accepted by FeatureIDE.
pub(crate) struct IoFormulaParser {
    /// The extension of the parsed file.
    ///
    /// Used by FeatureIDE to determine the file format.
    extension: String,
}

impl IoFormulaParser {
    pub(crate) fn new(extension: String) -> Self {
        IoFormulaParser { extension }
    }
}

impl FormulaParser for IoFormulaParser {
    fn parse_into(&self, file: &str, arena: &mut Arena) -> Formula {
        let sat_file = util::exec::io(file, &self.extension, "sat", &[]);
        let mut formula = SatFormulaParser.parse_into(&sat_file, arena);
        formula.file = Some(File::new(file.to_string(), Some(self.extension.clone())));
        formula
    }
}
