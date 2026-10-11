//! Run the supported Python float pilot on the native Rust LANG VM.

use std::io::{Read, Write};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: pyvm <source.py>");
        std::process::exit(2);
    };
    let source = match std::fs::File::open(&path).and_then(|file| {
        let mut source = String::new();
        file.take((python_iir_compiler::MAX_SOURCE_BYTES + 1) as u64)
            .read_to_string(&mut source)?;
        Ok(source)
    }) {
        Ok(source) if source.len() <= python_iir_compiler::MAX_SOURCE_BYTES => source,
        Ok(_) => {
            eprintln!(
                "pyvm: Python source exceeds the {}-byte native pilot limit",
                python_iir_compiler::MAX_SOURCE_BYTES
            );
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("pyvm: {error}");
            std::process::exit(1);
        }
    };
    match python_iir_compiler::run_source(&source) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            // The VM may have completed earlier print calls before the failing
            // instruction. Flush their stdout before reporting the error.
            let mut stdout = std::io::stdout().lock();
            if let Err(write_error) = stdout
                .write_all(error.output.as_bytes())
                .and_then(|_| stdout.flush())
            {
                eprintln!("pyvm: cannot write prior output: {write_error}");
                std::process::exit(1);
            }
            drop(stdout);
            eprintln!("pyvm: {}", error.message);
            std::process::exit(1);
        }
    }
}
