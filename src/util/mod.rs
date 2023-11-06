//! Miscellaneous utilities.

use std::{fs, io::Read, path::Path};

pub(super) mod exec;

/// Returns whether a file exists at a given path.
///
/// Also allows the special value - for referring to standard input.
pub(super) fn file_exists(file_name: &str) -> bool {
    Path::new(file_name).exists() || file_name.starts_with("-")
}

/// Reads the contents and extension of a file.
pub(super) fn read_contents(file_name: &str) -> (String, Option<String>) {
    let mut contents;
    if file_name.starts_with("-") {
        contents = String::new();
        std::io::stdin().read_to_string(&mut contents).unwrap();
    } else {
        contents = fs::read_to_string(file_name).unwrap();
    };
    let extension = Path::new(file_name)
        .extension()
        .map_or(None, |e| e.to_str())
        .map(|e| e.to_string());
    (contents, extension)
}
