//! Run the current JavaScript pilot with this repository's native Rust VM.

fn main() {
    let path = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: jsvm <file.js>");
            std::process::exit(2);
        }
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{path}: {error}");
            std::process::exit(1);
        }
    };
    match javascript_iir_compiler::run_source(&source) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("{path}: {error}");
            std::process::exit(1);
        }
    }
}
