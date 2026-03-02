//! Binary entry point for the `clausy` CLI.

/// Panic reporting customization for user-facing crashes.
mod panic;

/// Starts the CLI:
/// 1. install the custom panic hook
/// 2. dispatch to the shell command processor
fn main() {
    panic::install_panic_hook();
    clausy::shell::main();
}
