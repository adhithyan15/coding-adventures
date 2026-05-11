//! Regenerate `src/_grammar.rs` from `code/grammars/prolog/iso.grammar`.
//!
//! Run with:
//!
//! ```sh
//! cargo run -p prolog-parser --example regenerate_grammar
//! ```
//!
//! This reads the canonical ISO Prolog parser grammar (EBNF) and writes
//! the compiled Rust embedding to `src/_grammar.rs`. The output is
//! checked into the repository so downstream users do not need file
//! I/O at startup.

use std::fs;
use std::path::PathBuf;

use grammar_tools::compiler::compile_parser_grammar;
use grammar_tools::parser_grammar::parse_parser_grammar;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let grammars_dir = crate_root
        .ancestors()
        .nth(3)
        .ok_or("could not locate the repo's `code/` parent")?
        .join("grammars");
    let grammar_path = grammars_dir.join("prolog").join("iso.grammar");

    let source = fs::read_to_string(&grammar_path)
        .map_err(|e| format!("could not read {}: {}", grammar_path.display(), e))?;
    let grammar = parse_parser_grammar(&source)?;

    let rust_source = compile_parser_grammar(&grammar, "iso.grammar");
    let out = crate_root.join("src").join("_grammar.rs");
    fs::write(&out, rust_source)?;

    println!(
        "wrote {} from {}",
        out.display(),
        grammar_path.display()
    );
    Ok(())
}
