mod panic;

fn main() {
    panic::install_panic_hook();
    clausy::shell::main();
}
