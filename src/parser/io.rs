//! Parser for feature-model formula files in any format accepted by FeatureIDE.


use crate::{core::formula::{Formula, Id}, util};

use super::{FormulaParser, sat::SatFormulaParser};

pub(crate) struct IOFormulaParser {
    extension: &'static str
}

impl FormulaParser for IOFormulaParser {
    fn parse_into<'a>(&self, file: &'a mut String, formula: &mut Formula<'a>) -> Id {
        *file = util::exec::io(&file, self.extension, "sat");
        SatFormulaParser.parse_into(file, formula)
    }
}