//! Run LANG81's Perl subset on the repository's native Rust VM.

use std::io::Read;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: plvm <source.pl>");
        std::process::exit(2);
    };
    let limit = coding_adventures_perl_lexer::MAX_SOURCE_BYTES;
    let source = match std::fs::File::open(&path).and_then(|file| {
        let mut source = String::new();
        file.take((limit + 1) as u64).read_to_string(&mut source)?;
        Ok(source)
    }) {
        Ok(source) if source.len() <= limit => source,
        Ok(_) => {
            eprintln!("plvm: Perl source exceeds the {limit}-byte native pilot limit");
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("plvm: {error}");
            std::process::exit(1);
        }
    };
    match perl_iir_compiler::run_source(&source) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("plvm: {error}");
            std::process::exit(1);
        }
    }
}
