//! Defines a reference to a feature-model formula.

use std::fmt;

use super::{arena::Arena, formula::Formula};

/// A shared reference to a feature-model formula.
///
/// This is useful whenever we need to pass a formula around and also need the containing arena.
pub(crate) struct FormulaRef<'a> {
    pub(crate) arena: &'a Arena,
    pub(crate) formula: &'a Formula,
}

impl<'a> fmt::Display for FormulaRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.arena.format_expr(self.formula.root_id, f)
    }
}
