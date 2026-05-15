//! Redirectable output writer.
//!
//! By default, output goes to stdout. Call [`init`] with a file path to redirect to a file.
//! A path of `-` always means stdout.

use std::fs;
use std::io::{self, Write};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Global output writer, initialized on demand. Defaults to stdout if not initialized.
static OUTPUT: OnceLock<Mutex<Box<dyn Write + Send>>> = OnceLock::new();

/// Redirects output to the given file path. Has no effect for `-` (stdout).
pub(crate) fn init(path: &str) {
    if path == "-" {
        return;
    }
    let writer: Box<dyn Write + Send> = Box::new(
        fs::File::create(path)
            .unwrap_or_else(|e| panic!("failed to create output file '{path}': {e}")),
    );
    OUTPUT
        .set(Mutex::new(writer))
        .unwrap_or_else(|_| panic!("output writer was initialized more than once"));
}

/// Returns the global output writer, defaulting to stdout if not initialized.
pub(crate) fn writer() -> MutexGuard<'static, Box<dyn Write + Send>> {
    OUTPUT
        .get_or_init(|| Mutex::new(Box::new(io::stdout())))
        .lock()
        .unwrap()
}
