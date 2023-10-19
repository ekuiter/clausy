//! Parser for any file format accepted by FeatureIDE.

use crate::{
    core::{arena::Arena, formula::Formula},
    util,
};

use super::{sat::SatFormulaParser, FormulaParser};

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
    fn preprocess(&self, file: String) -> String {
        util::exec::io(&file, &self.extension, "sat", &[])
    }

    fn parse_into(&self, file: &str, arena: &mut Arena) -> Formula {
        SatFormulaParser.parse_into(file, arena)
    }
}
