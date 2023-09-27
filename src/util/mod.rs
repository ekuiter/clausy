//! Miscellaneous utilities.

use std::{fs, io::Read, path::Path};

pub(crate) mod exec;

/// Returns whether a file exists at a given path.
///
/// Also allows the special value - for referring to standard input.
pub(crate) fn file_exists(file_name: &str) -> bool {
    Path::new(file_name).exists() || file_name.starts_with("-")
}

/// Reads the contents and extension of a file.
pub(crate) fn read_file(file_name: &str) -> (String, Option<String>) {
    let mut file;
    if file_name.starts_with("-") {
        file = String::new();
        std::io::stdin().read_to_string(&mut file).unwrap();
    } else {
        file = fs::read_to_string(file_name).unwrap();
    };
    let extension = Path::new(file_name)
        .extension()
        .map_or(None, |e| e.to_str())
        .map(|e| e.to_string());
    (file, extension)
}
