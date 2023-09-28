use std::env;

fn main() {
    let mut args: Vec<String> = env::args().collect();
    args.remove(0);
    clausy::shell::main(args);
}
