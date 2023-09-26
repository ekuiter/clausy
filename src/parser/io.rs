//! Parser for any file format accepted by FeatureIDE.

use crate::{core::formula::{Formula, Id}, util};

use super::{FormulaParser, sat::SatFormulaParser};

/// Parses feature-model formula files in any file format accepted by FeatureIDE.
pub(crate) struct IoFormulaParser<'a> {
    /// The extension of the parsed file.
    /// 
    /// Used by FeatureIDE to determine the file format.
    extension: &'a str
}

impl<'a> IoFormulaParser<'a> {
    pub(crate) fn new(extension: &'a str) -> Self {
        IoFormulaParser { extension }
    }
}

impl<'a> FormulaParser for IoFormulaParser<'a> {
    fn parse_into<'b>(&self, file: &'b mut String, formula: &mut Formula<'b>) -> Id {
        *file = util::exec::io(&file, self.extension, "sat");
        SatFormulaParser.parse_into(file, formula)
    }
}