//! Parsers for feature-model formula files.

use std::collections::HashSet;

use crate::core::formula::{Arena, Id, VarId, Formula};

use self::{io::IoFormulaParser, model::ModelFormulaParser, sat::SatFormulaParser};

mod io;
mod model;
mod sat;
pub(crate) mod sat_inline;

/// Parses a feature-model formula file into a [Formula] instance.
pub(crate) trait FormulaParser {
    /// Preprocesses a feature-model formula file, if necessary.
    fn preprocess(&self, file: String) -> String {
        file
    }

    /// Parses a feature-model formula file into an existing [Formula].
    ///
    /// Returns the [Id] of the root expression of the parsed formula.
    /// Also returns a set with each [VarId] of a named sub-variable of the parsed formula.
    /// This function does not modify the sub-expressions of the given formula.
    /// That is, after parsing, the formula will hold the given feature-model formula in [Formula::exprs], but not refer to it.
    /// Thus, [Formula::set_root_expr] must be called explicitly with the returned [Id] to make use of the parsed formula.
    fn parse_into(&self, file: &str, arena: &mut Arena) -> Formula;

    // /// Parses a feature-model formula file into a new [Formula].
    // fn parse_new(&'a self, file: &str) -> (Arena, Formula) {
    //     let mut arena = Arena::new();
    //     let formula = self.parse_into(file, &mut arena);
    //     (arena, formula)
    // }
}

/// An object that can parse a feature-model formula file into itself.
///
/// Only implemented for [Formula].
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
// impl<'a, T> From<(&str, T)> for Formula
// where
//     T: FormulaParser,
// {
//     fn from(file_and_parser: (&str, T)) -> Self {
//         let (file, parser) = file_and_parser;
//         parser.parse_new(file).1
//     }
// }

/// Parses a feature-model formula file into an existing formula.
impl FormulaParsee for Arena {
    fn parse(&mut self, file: &str, parser: Box<dyn FormulaParser>) -> Formula {
        parser.parse_into(file, self)
    }
}
