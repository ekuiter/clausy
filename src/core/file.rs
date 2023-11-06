//! Defines a feature-model formula file.

use crate::{core::clauses::Clauses, util::exec};

/// The contents of a feature-model formula file.
///
/// Every [Formula] may be parsed from an existing input [File].
pub(crate) struct File {
    /// The contents of the file the associated formula was originally parsed from.
    pub(crate) contents: String,

    /// The extension of the file the associated formula was originally parsed from, if any.
    pub(crate) extension: Option<String>,
}

impl File {
    /// Creates a new file.
    pub(crate) fn new(
        contents: String,
        extension: Option<String>,
    ) -> Self {
        Self {
            contents,
            extension,
        }
    }

    /// Counts the number of solutions of a file using FeatureIDE.
    ///
    /// The file extension must be given so FeatureIDE can detect the correct format.
    fn count_featureide(&self) -> String {
        exec::d4(&exec::io(
            self.contents.as_str(),
            self.extension.as_ref().unwrap().as_str(),
            "dimacs",
            &[],
        ))
    }

    /// Panics if this file has a different model count than that of the given clauses.
    ///
    /// Useful for checking the correctness of count-preserving algorithms (e.g., [super::formula::Formula::to_cnf_tseitin]).
    pub(crate) fn assert_count(&self, clauses: &Clauses) {
        assert_eq!(
            clauses.count(),
            self.count_featureide()
        );
    }
}
