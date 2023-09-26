//! Miscellaneous utilities.

use std::{fs, path::Path, io::Read};

pub(crate) mod exec;

pub(crate) fn readable_file(file_name: &str) -> bool {
    Path::new(file_name).exists() || file_name == "-"
}

/// Reads the contents and extension of a file.
pub(crate) fn read_file(file_name: &str) -> (String, Option<String>) {
    let mut file;
    let extension;
    if file_name != "-" {
        file = fs::read_to_string(file_name).unwrap();
        // todo: move to parser.rs
        extension = Path::new(file_name)
            .extension()
            .map_or(None, |e| e.to_str())
            .map(|e| e.to_string());
    } else {
        file = String::new();
        extension = None;
        std::io::stdin().read_to_string(&mut file).unwrap();
    };
    (file, extension)
}