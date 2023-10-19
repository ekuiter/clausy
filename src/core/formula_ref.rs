//! Defines a reference to a feature-model formula.

use std::fmt;

use super::{arena::Arena, formula::Formula};

/// A reference to a feature-model formula.
///
/// This is useful whenever we need to pass a formula around and need the containing arena is not available.
pub(crate) struct FormulaRef<'a> {
    pub(crate) formula: &'a Formula,
    pub(crate) arena: &'a Arena,
}

impl<'a> fmt::Display for FormulaRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.arena.format_expr(self.formula.get_root_expr(), f)
    }
}
