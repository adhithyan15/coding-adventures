//! Regenerate `src/_grammar.rs` from `code/grammars/axiom/axiom.grammar`.
//!
//! Run with:
//!
//! ```sh
//! cargo run -p coding-adventures-axiom-parser --example regenerate_grammar
//! ```
//!
//! This reads the canonical `axiom.grammar` (EBNF) and writes the compiled
//! Rust embedding to `src/_grammar.rs`. The output is checked into the
//! repository so downstream users do not need file I/O at startup — see
//! this crate's own `src/lib.rs` doc comment. Mirrors `prolog-parser`'s own
//! `examples/regenerate_grammar.rs`, adapted to this crate's grammar file.
//!
//! After editing `code/grammars/axiom/axiom.grammar`, re-run this example
//! and commit the regenerated `src/_grammar.rs` alongside the `.grammar`
//! change — the two must never drift (lessons.md's own "a `.grammar`/
//! `.tokens` edit is not live" entry).

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
    let grammar_path = grammars_dir.join("axiom").join("axiom.grammar");

    let source = fs::read_to_string(&grammar_path)
        .map_err(|e| format!("could not read {}: {}", grammar_path.display(), e))?;
    let grammar = parse_parser_grammar(&source)?;

    let rust_source = compile_parser_grammar(&grammar, "axiom.grammar");
    let out = crate_root.join("src").join("_grammar.rs");
    fs::write(&out, rust_source)?;

    println!("wrote {} from {}", out.display(), grammar_path.display());
    Ok(())
}
