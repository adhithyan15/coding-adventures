//! # mccarthy-lisp-parser build script — compile `mccarthy_lisp.grammar` to Rust.
//!
//! Mirror of `mccarthy-lisp-lexer/build.rs` for the *parser* grammar.
//! Reads `code/grammars/mccarthy_lisp.grammar`, parses it via
//! [`grammar_tools::parser_grammar::parse_parser_grammar`], and runs
//! [`grammar_tools::compiler::compile_parser_grammar`] to emit Rust
//! source defining `pub fn parser_grammar() -> ParserGrammar`.  The
//! output is written to `$OUT_DIR/mccarthy_lisp_parser_grammar.rs`,
//! which `lib.rs` `include!`s and wraps in a `OnceLock`.

use std::env;
use std::fs;
use std::path::PathBuf;

use grammar_tools::compiler::compile_parser_grammar;
use grammar_tools::parser_grammar::parse_parser_grammar;

fn main() {
    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo");
    // code/packages/rust/mccarthy-lisp-parser → ../../../grammars
    let grammar_path = PathBuf::from(&manifest_dir)
        .join("..")
        .join("..")
        .join("..")
        .join("grammars")
        .join("mccarthy_lisp.grammar");
    let grammar_path = grammar_path.canonicalize().unwrap_or_else(|e| {
        panic!("Failed to resolve mccarthy_lisp.grammar path {grammar_path:?}: {e}")
    });

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", grammar_path.display());

    let grammar_text = fs::read_to_string(&grammar_path)
        .unwrap_or_else(|e| panic!("Failed to read {grammar_path:?}: {e}"));
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse {grammar_path:?}: {e}"));

    let rust = compile_parser_grammar(&grammar, "mccarthy_lisp.grammar");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let out_path = PathBuf::from(&out_dir).join("mccarthy_lisp_parser_grammar.rs");
    fs::write(&out_path, rust)
        .unwrap_or_else(|e| panic!("Failed to write {out_path:?}: {e}"));
}
