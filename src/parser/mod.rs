//! Parsers for feature-model formula files.

use crate::core::formula::{Formula, Id};

use self::{model::ModelFormulaParser, sat::SatFormulaParser, io::IoFormulaParser};

pub(crate) mod io;
pub(crate) mod model;
pub(crate) mod sat;

/// Parses a feature-model formula file into a [Formula] instance.
trait FormulaParser {
    /// Parses a feature-model formula file into an existing [Formula].
    ///
    /// Returns the [Id] of the root expression of the parsed formula.
    /// This function does not modify the sub-expressions of the given formula.
    /// That is, after parsing, the formula will hold the given feature-model formula in [Formula::exprs], but not refer to it.
    /// Thus, [Formula::set_root_expr] must be called explicitly with the returned [Id] to make use of the parsed formula.
    fn parse_into<'a>(&self, file: &'a mut String, formula: &mut Formula<'a>) -> Id;

    /// Parses a feature-model formula file into a new [Formula].
    fn parse_new<'a>(&self, file: &'a mut String) -> Formula<'a> {
        let mut formula = Formula::new();
        let root_id = self.parse_into(file, &mut formula);
        formula.set_root_expr(root_id);
        formula
    }
}

/// Creates a feature-model formula from a feature-model formula file and parser.
impl<'a, T> From<(&'a mut String, T)> for Formula<'a>
where
    T: FormulaParser,
{
    fn from(file_and_parser: (&'a mut String, T)) -> Self {
        let (file, parser) = file_and_parser;
        parser.parse_new(file)
    }
}

fn get_parser<'a>(extension: Option<&'a str>) -> Box<dyn FormulaParser + 'a> {
    match extension {
        Some(extension) => match extension {
            "sat" => Box::new(SatFormulaParser),
            "model" => Box::new(ModelFormulaParser),
            _ => Box::new(IoFormulaParser::new(extension)),
        },
        None => Box::new(SatFormulaParser),
    }
}

impl<'a> From<(&'a mut String, Option<&str>)> for Formula<'a> {
    fn from(file_and_extension: (&'a mut String, Option<&str>)) -> Self {
        let (file, extension) = file_and_extension;
        get_parser(extension).parse_new(file)
    }
}
