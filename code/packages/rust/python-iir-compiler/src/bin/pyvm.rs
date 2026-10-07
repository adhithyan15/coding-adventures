//! Run the supported Python float pilot on the native Rust LANG VM.

use std::io::Read;

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
            eprintln!("pyvm: {error}");
            std::process::exit(1);
        }
    }
}
