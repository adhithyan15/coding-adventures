//! Run the current JavaScript pilot with this repository's native Rust VM.

use std::io::{Read, Write};

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
            let mut stdout = std::io::stdout().lock();
            if let Err(write_error) = stdout
                .write_all(error.output.as_bytes())
                .and_then(|_| stdout.flush())
            {
                eprintln!("{path}: cannot write prior output: {write_error}");
                std::process::exit(1);
            }
            drop(stdout);
            eprintln!("{path}: {}", error.message);
            std::process::exit(1);
        }
    }
}
