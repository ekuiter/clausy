//! Defines an ordered list of feature-model formulas.

use super::{arena::Arena, formula::Formula, formula_ref::FormulaRef};

/// An ordered list of feature-model formulas.
///
/// This is useful for parsing and manipulating multiple related formulas at once while leveraging structural sharing.
pub(crate) struct FormulaVec {
    pub(crate) arena: Arena,
    pub(crate) formulas: Vec<Formula>,
}

impl FormulaVec {
    pub(crate) fn new() -> Self {
        FormulaVec {
            arena: Arena::new(),
            formulas: vec![],
        }
    }

    pub(crate) fn last(&self) -> &Formula {
        self.formulas.last().unwrap()
    }

    pub(crate) fn last_mut(&mut self) -> &mut Formula {
        self.formulas.last_mut().unwrap()
    }

    pub(crate) fn last_ref(&self) -> FormulaRef {
        self.last().as_ref(&self.arena)
    }

    // pub(crate) fn last_ref_mut(&mut self) -> FormulaMutRef {
    //     self.last_mut().as_mut(&mut self.arena)
    // }
}
