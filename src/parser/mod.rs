//! Parsers for feature-model formula files.

use self::{io::IoFormulaParser, model::ModelFormulaParser, sat::SatFormulaParser};
use crate::core::arena::Arena;
use crate::core::formula::Formula;

mod io;
mod model;
mod sat;
pub(crate) mod sat_inline;

/// Parses a feature-model formula file into an [Arena].
pub(crate) trait FormulaParser {
    /// Parses a feature-model formula file into an existing [Arena].
    ///
    /// Returns the parsed [Formula].
    /// Does not modify the sub-expressions of any other formula in the arena.
    fn parse_into(&self, file: &str, arena: &mut Arena) -> Formula;

    /// Parses a feature-model formula file into a new [Arena].
    fn parse_new(&self, file: &str) -> (Arena, Formula) {
        let mut arena = Arena::new();
        let formula = self.parse_into(file, &mut arena);
        (arena, formula)
    }
}

/// An object that can parse a feature-model formula file into itself.
///
/// Only implemented for [Arena].
pub(crate) trait FormulaParsee {
    /// Parses a feature-model formula into this object.
    fn parse(&mut self, file: &str, parser: Box<dyn FormulaParser>) -> Formula;
}

/// Returns the appropriate parser for a file extension.
pub(crate) fn parser(extension: Option<String>) -> Box<dyn FormulaParser> {
    match extension {
        Some(extension) => match extension.as_str() {
            "sat" => Box::new(SatFormulaParser),
            "model" => Box::new(ModelFormulaParser),
            _ => Box::new(IoFormulaParser::new(extension)),
        },
        None => Box::new(SatFormulaParser),
    }
}

/// Creates a feature-model formula from a feature-model formula file and parser.
impl<'a, T> From<(&str, T)> for Formula
where
    T: FormulaParser,
{
    fn from(file_and_parser: (&str, T)) -> Self {
        let (file, parser) = file_and_parser;
        parser.parse_new(file).1
    }
}

impl FormulaParsee for Arena {
    fn parse(&mut self, file: &str, parser: Box<dyn FormulaParser>) -> Formula {
        parser.parse_into(file, self)
    }
}
