//! Run the supported Python float pilot on the native Rust LANG VM.

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: pyvm <source.py>");
        std::process::exit(2);
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("pyvm: {error}");
            std::process::exit(1);
        }
    };
    match python_iir_compiler::run_source(&source) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("pyvm: {error}");
            std::process::exit(1);
        }
    }
}
