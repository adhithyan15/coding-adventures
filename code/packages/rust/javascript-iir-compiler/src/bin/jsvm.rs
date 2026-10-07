//! Run the current JavaScript pilot with this repository's native Rust VM.

use std::io::Read;

fn main() {
    let path = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: jsvm <file.js>");
            std::process::exit(2);
        }
    };
    let source = match std::fs::File::open(&path).and_then(|file| {
        let mut source = String::new();
        file.take((javascript_iir_compiler::MAX_SOURCE_BYTES + 1) as u64)
            .read_to_string(&mut source)?;
        Ok(source)
    }) {
        Ok(source) if source.len() <= javascript_iir_compiler::MAX_SOURCE_BYTES => source,
        Ok(_) => {
            eprintln!(
                "{path}: JavaScript source exceeds the {}-byte native pilot limit",
                javascript_iir_compiler::MAX_SOURCE_BYTES
            );
            std::process::exit(1);
        }
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
