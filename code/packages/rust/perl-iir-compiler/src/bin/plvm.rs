//! Run the bounded Perl subset on the repository's native Rust VM.
//!
//! ```text
//! plvm [--release <inventory-id>] <source.pl>
//! ```
//!
//! Without `--release` the program compiles against `DEFAULT_RELEASE`. With
//! it, against exactly that row of `code/grammars/perl/releases.csv`; a
//! pending or unknown ID is an error, never a fallback.

use std::io::Read;

fn usage() -> ! {
    eprintln!("usage: plvm [--release <inventory-id>] <source.pl>");
    std::process::exit(2);
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (release, path) = match (args.next(), args.next(), args.next(), args.next()) {
        (Some(path), None, None, None) if path != "--release" => (None, path),
        (Some(flag), Some(release), Some(path), None) if flag == "--release" => {
            (Some(release), path)
        }
        _ => usage(),
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
    match perl_iir_compiler::run_source_at(&source, release.as_deref()) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("plvm: {error}");
            std::process::exit(1);
        }
    }
}
