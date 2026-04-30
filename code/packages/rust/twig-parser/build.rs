//! # twig-parser build script — compile twig.grammar into Rust at build time.
//!
//! Mirror of `twig-lexer/build.rs` for the parser grammar.  See that
//! file's docs for the rationale; this script does the same thing
//! with `parse_parser_grammar` + `parser_grammar_to_rust_source`.

use std::env;
use std::fs;
use std::path::PathBuf;

use grammar_tools::codegen::parser_grammar_to_rust_source;
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

    let rust = parser_grammar_to_rust_source(&grammar, "twig_parser_grammar");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let out_path = PathBuf::from(&out_dir).join("twig_parser_grammar.rs");
    fs::write(&out_path, rust)
        .unwrap_or_else(|e| panic!("Failed to write {out_path:?}: {e}"));
}
