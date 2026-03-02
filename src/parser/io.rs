//! Parser for any file format accepted by FeatureIDE.

use super::{sat::SatFormulaParser, FormulaParser};
use crate::core::{arena::Arena, file::File, formula::Formula};

/// Parses feature-model formula files in any file format accepted by FeatureIDE.
pub(crate) struct IoFormulaParser;

impl FormulaParser for IoFormulaParser {
    fn parse_into(&self, file: File, arena: &mut Arena) -> Formula {
        let sat_file = file.convert_with_featureide("sat");
        let mut formula = SatFormulaParser.parse_into(sat_file, arena);
        formula.file = Some(file);
        formula
    }
}
