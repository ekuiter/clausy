//! Parsers for feature-model formula files.

use crate::core::formula::{Formula, Id};

pub(crate) mod model;
pub(crate) mod sat;
pub(crate) mod io;

/// Parses a feature-model formula file into a [Formula] instance.
trait FormulaParser {
    /// Parses a feature-model formula file into an existing [Formula].
    /// 
    /// Returns the [Id] of the root expression of the parsed formula.
    /// This function does not modify the sub-expressions of the given formula.
    /// That is, after parsing, the formula will hold the given feature-model formula in [Formula::exprs], but not refer to it.
    /// Thus, [Formula::set_root_expr] must be called explicitly with the returned [Id] to make use of the parsed formula.
    fn parse_into<'a>(&self, file: &'a str, formula: &mut Formula<'a>) -> Id;

    /// Parses a feature-model formula file into a new [Formula].
    fn parse_new(&self, file: &str) -> Formula {
        let mut formula = Formula::new();
        formula.set_root_expr(self.parse_into(file, &mut formula));
        formula
    }
}

/// Creates a feature-model formula from a feature-model formula file and parser.
impl<'a, T> From<(&'a str, T)> for Formula<'a> where T: FormulaParser {
    fn from(file_and_parser: (&'a str, T)) -> Self {
        let (file, parser) = file_and_parser;
        parser.parse_new(file)
    }
}