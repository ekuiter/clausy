//! clausy transforms feature-model formulas into conjunctive normal form (CNF) for subsequent analysis.
//! 
//! * For information on installation and usage, visit [github.com/ekuiter/clausy](https://github.com/ekuiter/clausy/).
//! * clausy should to be called from a binary crate via [shell::main], use in library crates is not intended.
//! * As a starting point in this documentation, see [core::formula::Formula] for important algorithms.

#![allow(rustdoc::private_intra_doc_links)]

pub mod shell;
mod core;
mod parser;
mod tests;
mod util;