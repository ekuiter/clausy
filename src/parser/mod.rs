//! Parsers for feature-model formula files.

use crate::core::formula::{Formula, Id};

use self::{io::IoFormulaParser, model::ModelFormulaParser, sat::SatFormulaParser};

mod io;
mod model;
mod sat;

/// Parses a feature-model formula file into a [Formula] instance.
pub(crate) trait FormulaParser {
    /// Preprocesses a feature-model formula file, if necessary.
    fn preprocess(&self, file: String) -> String {
        file
    }

    /// Parses a feature-model formula file into an existing [Formula].
    ///
    /// Returns the [Id] of the root expression of the parsed formula.
    /// This function does not modify the sub-expressions of the given formula.
    /// That is, after parsing, the formula will hold the given feature-model formula in [Formula::exprs], but not refer to it.
    /// Thus, [Formula::set_root_expr] must be called explicitly with the returned [Id] to make use of the parsed formula.
    fn parse_into<'a>(&self, file: &'a String, formula: &mut Formula<'a>) -> Id;

    /// Parses a feature-model formula file into a new [Formula].
    fn parse_new<'a>(&self, file: &'a String) -> Formula<'a> {
        let mut formula = Formula::new();
        let root_id = self.parse_into(file, &mut formula);
        formula.set_root_expr(root_id);
        formula
    }
}

/// An object that can parse a feature-model formula file into itself.
///
/// Only implemented for [Formula].
pub(crate) trait FormulaParsee<'a> {
    fn parse(&mut self, file: &'a String, parser: Box<dyn FormulaParser>) -> Id;
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
impl<'a, T> From<(&'a String, T)> for Formula<'a>
where
    T: FormulaParser,
{
    fn from(file_and_parser: (&'a String, T)) -> Self {
        let (file, parser) = file_and_parser;
        parser.parse_new(file)
    }
}

/// Parses a feature-model formula file into an existing formula.
impl<'a> FormulaParsee<'a> for Formula<'a> {
    fn parse(&mut self, file: &'a String, parser: Box<dyn FormulaParser>) -> Id {
        parser.parse_into(file, self)
    }
}
