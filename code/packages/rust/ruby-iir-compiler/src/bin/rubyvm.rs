//! Execute a Ruby source file through the native IIR interpreter pilot.

use std::io::Read;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: rubyvm <file.rb>");
        std::process::exit(2);
    };
    let source = match std::fs::File::open(&path).and_then(|file| {
        let mut source = String::new();
        file.take((ruby_iir_compiler::MAX_SOURCE_BYTES + 1) as u64)
            .read_to_string(&mut source)?;
        Ok(source)
    }) {
        Ok(source) if source.len() <= ruby_iir_compiler::MAX_SOURCE_BYTES => source,
        Ok(_) => {
            eprintln!("{path}: Ruby source exceeds native pilot limit");
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("{path}: {error}");
            std::process::exit(1);
        }
    };
    match ruby_iir_compiler::run_source(&source) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("{path}: {error}");
            std::process::exit(1);
        }
    }
}
