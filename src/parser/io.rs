//! Parser for any file format accepted by FeatureIDE.

use crate::{core::formula::{Formula, Id}, util};

use super::{FormulaParser, sat::SatFormulaParser};

/// Parses feature-model formula files in any file format accepted by FeatureIDE.
pub(crate) struct IoFormulaParser {
    /// The extension of the parsed file.
    /// 
    /// Used by FeatureIDE to determine the file format.
    extension: String
}

impl IoFormulaParser {
    pub(crate) fn new(extension: String) -> Self {
        IoFormulaParser { extension }
    }
}

impl FormulaParser for IoFormulaParser {
    fn preprocess(&self, file: String) -> String {
        util::exec::io(&file, &self.extension, "sat")
    }

    fn parse_into<'b>(&self, file: &'b String, formula: &mut Formula<'b>) -> Id {
        SatFormulaParser.parse_into(file, formula)
    }
}