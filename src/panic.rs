//! Custom panic hook for CLI-friendly crash reporting.
//!
//! This module replaces Rust's default panic output with a compact,
//! human-readable error block that is consistent with other `clausy`
//! diagnostics.

use std::io::IsTerminal;

/// Splits panic payloads that include nested error details.
///
/// For payloads like `"...: Os { ... }"` or `"...: Custom { ... }"`,
/// this returns the leading message and the extracted exception suffix.
/// If no known marker is present, the full payload is returned as message.
fn format_panic_message(message: &str) -> (&str, Option<&str>) {
    for marker in [": Os {", ": Custom {"] {
        if let Some(idx) = message.find(marker) {
            return (&message[..idx], Some(&message[idx + 2..]));
        }
    }
    (message, None)
}

#[derive(Clone, Copy)]
struct Theme {
    /// ANSI reset sequence.
    reset: &'static str,
    /// Header color/style (`clausy error`).
    header: &'static str,
    /// Label color/style (`Message`, `Exception`, ...).
    label: &'static str,
    /// Primary message color/style.
    message: &'static str,
    /// Exception detail color/style.
    exception: &'static str,
    /// Location/backtrace hint color/style.
    location: &'static str,
}

impl Theme {
    /// Theme without ANSI escapes (used for non-TTY stderr or `NO_COLOR`).
    fn plain() -> Self {
        Self {
            reset: "",
            header: "",
            label: "",
            message: "",
            exception: "",
            location: "",
        }
    }

    /// Theme with ANSI escapes for improved readability in interactive terminals.
    fn colored() -> Self {
        Self {
            reset: "\x1b[0m",
            header: "\x1b[1;31m",
            label: "\x1b[1;36m",
            message: "\x1b[97m",
            exception: "\x1b[93m",
            location: "\x1b[90m",
        }
    }
}

/// Returns whether panic output should include ANSI colors.
///
/// Color is disabled when:
/// - `NO_COLOR` is set, or
/// - stderr is not connected to a terminal.
fn use_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stderr().is_terminal()
}

/// Returns whether a backtrace should be printed in the panic report.
///
/// This follows Rust conventions and enables backtraces for:
/// - `RUST_BACKTRACE=1`
/// - `RUST_BACKTRACE=full`
fn backtrace_requested() -> bool {
    matches!(
        std::env::var("RUST_BACKTRACE").ok().as_deref(),
        Some("1" | "full")
    )
}

/// Installs a global panic hook with compact, structured output.
///
/// The hook prints:
/// - a fixed header (`clausy error`)
/// - message and extracted exception details
/// - source location if available
/// - an optional backtrace (or a hint to enable one)
pub(crate) fn install_panic_hook() {
    let theme = if use_color() {
        Theme::colored()
    } else {
        Theme::plain()
    };
    std::panic::set_hook(Box::new(move |info| {
        let payload = info
            .payload()
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| info.payload().downcast_ref::<&str>().copied())
            .unwrap_or("panic without message");
        let (message, exception) = format_panic_message(payload);

        // Keep the first line stable so scripts and users can recognize failures quickly.
        eprintln!("{}clausy error{}", theme.header, theme.reset);
        eprintln!(
            "{}Message{}   {}{}{}",
            theme.label, theme.reset, theme.message, message, theme.reset
        );
        if let Some(exception) = exception {
            eprintln!(
                "{}Exception{} {}{}{}",
                theme.label, theme.reset, theme.exception, exception, theme.reset
            );
        } else {
            eprintln!(
                "{}Exception{} {}panic{}",
                theme.label, theme.reset, theme.exception, theme.reset
            );
        }
        if let Some(location) = info.location() {
            eprintln!(
                "{}Location{}  {}{}:{}:{}{}",
                theme.label,
                theme.reset,
                theme.location,
                location.file(),
                location.line(),
                location.column(),
                theme.reset
            );
        } else {
            eprintln!(
                "{}Location{}  {}unknown{}",
                theme.label, theme.reset, theme.location, theme.reset
            );
        }
        if backtrace_requested() {
            // Force capture so we get a trace even in contexts where it is lazily omitted.
            let backtrace = std::backtrace::Backtrace::force_capture();
            eprintln!(
                "{}Backtrace{}\n{}{}{}",
                theme.label, theme.reset, theme.location, backtrace, theme.reset
            );
        } else {
            eprintln!(
                "{}note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace{}",
                theme.location, theme.reset
            );
        }
    }));
}
