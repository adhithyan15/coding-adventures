//! # twig-parser build script — compile twig.grammar into Rust at build time.
//!
//! Mirror of `twig-lexer/build.rs` for the parser grammar.  Uses
//! `grammar_tools::compiler::compile_parser_grammar` to emit Rust
//! source defining `pub fn parser_grammar() -> ParserGrammar`.
//! The lib.rs wraps that in a `OnceLock<ParserGrammar>` so the
//! struct is materialised once per process.

use std::env;
use std::fs;
use std::path::PathBuf;

use grammar_tools::compiler::compile_parser_grammar;
use grammar_tools::parser_grammar::parse_parser_grammar;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set by cargo");
    let grammar_path = PathBuf::from(&manifest_dir)
        .join("..").join("..").join("..")
        .join("grammars").join("twig.grammar");
    let grammar_path = grammar_path
        .canonicalize()
        .unwrap_or_else(|e| panic!("Failed to resolve twig.grammar path {grammar_path:?}: {e}"));

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", grammar_path.display());

    let grammar_text = fs::read_to_string(&grammar_path)
        .unwrap_or_else(|e| panic!("Failed to read {grammar_path:?}: {e}"));
    let grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse {grammar_path:?}: {e}"));

    let rust = compile_parser_grammar(&grammar, "twig.grammar");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let out_path = PathBuf::from(&out_dir).join("twig_parser_grammar.rs");
    fs::write(&out_path, rust)
        .unwrap_or_else(|e| panic!("Failed to write {out_path:?}: {e}"));
}
