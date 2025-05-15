use delta as lib;

fn main() {
    if let Err(e) = lib::main() {
        eprintln!("{}", e);
    };
}
