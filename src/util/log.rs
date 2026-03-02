//! Optional runtime logging.
//!
//! This module provides two lightweight helpers:
//! - [`log`] for structured informational lines prefixed with `c `
//! - [`scope`] for automatic duration logging via an RAII guard
//!
//! All log output is written to stderr, while command payloads stay on stdout.
//! This keeps machine-readable stdout stable and allows users to disable logs
//! entirely via `--quiet`.

use crate::shell::options;
use std::time::Instant;

/// Emits one optional informational log line.
///
/// The line is prefixed with `c ` and written to stderr.
/// This prefix is ignored by typical solver-adjacent tooling.
/// Log output can be easily distinguished from other output by distinguishing stderr from stdout.
/// Thus, piping the a resulting CNF into a file will only capture the CNF, not the log output.
/// Alternatively, log output can be disabled completely by using the `--quiet` flag.
pub(crate) fn log(msg: &str) {
    if options().output.quiet {
        return;
    }
    eprintln!("c {msg}");
}

/// RAII timer that logs duration when dropped.
///
/// Prefer creating instances via [`scope`] rather than constructing directly.
pub(crate) struct ScopeTimer {
    module: String,
    operation: String,
    started_at: Instant,
}

impl Drop for ScopeTimer {
    /// Emits one timing line on scope exit.
    ///
    /// Example shape:
    /// `c [SHELL] transform to_cnf_dist duration=2.143 ms (2143 us)`
    fn drop(&mut self) {
        let elapsed = self.started_at.elapsed();
        let elapsed_us = elapsed.as_micros();
        let elapsed_readable = format_elapsed(elapsed.as_secs_f64());
        log(&format!(
            "[{}] {} duration={} ({elapsed_us} us)",
            self.module, self.operation, elapsed_readable
        ));
    }
}

/// Starts measuring elapsed time for `operation` in `module`.
///
/// The returned guard logs automatically when it goes out of scope.
/// This guarantees timing output even when the scope exits early.
pub(crate) fn scope(module: &str, operation: &str) -> ScopeTimer {
    ScopeTimer {
        module: module.to_string(),
        operation: operation.to_string(),
        started_at: Instant::now(),
    }
}

/// Converts seconds to a compact human-readable duration string.
///
/// Output ranges:
/// - `< 1 ms` as microseconds (`us`)
/// - `< 1 s` as milliseconds (`ms`)
/// - `< 60 s` as seconds (`s`)
/// - `>= 60 s` as minutes + seconds (`Xm Y.YYYs`)
fn format_elapsed(seconds: f64) -> String {
    if seconds < 0.001 {
        return format!("{:.0} us", seconds * 1_000_000.0);
    }
    if seconds < 1.0 {
        return format!("{:.3} ms", seconds * 1000.0);
    }
    if seconds < 60.0 {
        return format!("{seconds:.6} s");
    }
    let minutes = (seconds / 60.0).floor() as u64;
    let remaining_seconds = seconds - (minutes as f64 * 60.0);
    format!("{minutes}m {remaining_seconds:.3}s")
}
