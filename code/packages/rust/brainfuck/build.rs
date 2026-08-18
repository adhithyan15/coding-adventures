//! # brainfuck build script — compile brainfuck.tokens/.grammar into Rust at build time.
//!
//! Reads `code/grammars/brainfuck/brainfuck.tokens` and
//! `code/grammars/brainfuck/brainfuck.grammar`, parses them via
//! `grammar_tools::token_grammar::parse_token_grammar` /
//! `grammar_tools::parser_grammar::parse_parser_grammar`, runs
//! `grammar_tools::compiler::compile_token_grammar` /
//! `compile_parser_grammar` to emit Rust source code that reconstructs
//! the parsed grammars, and writes the results to `$OUT_DIR`.
//!
//! `src/lexer.rs` and `src/parser.rs` each `include!` their generated file
//! inside a private module and wrap the generated constructor in a
//! `OnceLock` so the struct is materialised exactly once per process.
//!
//! ## Why a build.rs
//!
//! Mirrors `twig-lexer/build.rs` / `twig-parser/build.rs` (the established
//! pattern in this repo — see those files for the full rationale). In
//! short: no runtime file I/O (works under Miri's default FS isolation,
//! works in a published crate with no `code/grammars/` tree on disk),
//! build-time validation, and one construction per process instead of one
//! per lexer/parser call.
//!
//! ## Cargo rerun signals
//!
//! `cargo:rerun-if-changed` tells Cargo to re-run this build script
//! whenever `build.rs` itself or either grammar file changes.

use std::env;
use std::fs;
use std::path::PathBuf;

use grammar_tools::compiler::{compile_parser_grammar, compile_token_grammar};
use grammar_tools::parser_grammar::parse_parser_grammar;
use grammar_tools::token_grammar::parse_token_grammar;

fn main() {
    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");

    let grammars_dir = PathBuf::from(&manifest_dir)
        .join("..")
        .join("..")
        .join("..")
        .join("grammars")
        .join("brainfuck");

    println!("cargo:rerun-if-changed=build.rs");

    // --- brainfuck.tokens -> OUT_DIR/brainfuck_token_grammar.rs ---
    let tokens_path = grammars_dir.join("brainfuck.tokens");
    let tokens_path = tokens_path
        .canonicalize()
        .unwrap_or_else(|e| panic!("Failed to resolve brainfuck.tokens path {tokens_path:?}: {e}"));
    println!("cargo:rerun-if-changed={}", tokens_path.display());

    let tokens_text = fs::read_to_string(&tokens_path)
        .unwrap_or_else(|e| panic!("Failed to read {tokens_path:?}: {e}"));
    let token_grammar = parse_token_grammar(&tokens_text)
        .unwrap_or_else(|e| panic!("Failed to parse {tokens_path:?}: {e}"));

    let token_rust = compile_token_grammar(&token_grammar, "brainfuck.tokens");
    let token_out_path = PathBuf::from(&out_dir).join("brainfuck_token_grammar.rs");
    fs::write(&token_out_path, token_rust)
        .unwrap_or_else(|e| panic!("Failed to write {token_out_path:?}: {e}"));

    // --- brainfuck.grammar -> OUT_DIR/brainfuck_parser_grammar.rs ---
    let grammar_path = grammars_dir.join("brainfuck.grammar");
    let grammar_path = grammar_path.canonicalize().unwrap_or_else(|e| {
        panic!("Failed to resolve brainfuck.grammar path {grammar_path:?}: {e}")
    });
    println!("cargo:rerun-if-changed={}", grammar_path.display());

    let grammar_text = fs::read_to_string(&grammar_path)
        .unwrap_or_else(|e| panic!("Failed to read {grammar_path:?}: {e}"));
    let parser_grammar = parse_parser_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse {grammar_path:?}: {e}"));

    let parser_rust = compile_parser_grammar(&parser_grammar, "brainfuck.grammar");
    let parser_out_path = PathBuf::from(&out_dir).join("brainfuck_parser_grammar.rs");
    fs::write(&parser_out_path, parser_rust)
        .unwrap_or_else(|e| panic!("Failed to write {parser_out_path:?}: {e}"));
}
