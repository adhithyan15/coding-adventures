//! # mccarthy-lisp-lexer build script — compile `mccarthy_lisp.tokens` to Rust.
//!
//! Reads `code/grammars/mccarthy_lisp.tokens`, parses it via
//! [`grammar_tools::token_grammar::parse_token_grammar`], and runs
//! [`grammar_tools::compiler::compile_token_grammar`] to emit Rust
//! source that reconstructs the parsed `TokenGrammar` as native struct
//! literals.  The output is written to
//! `$OUT_DIR/mccarthy_lisp_token_grammar.rs`, which `lib.rs` `include!`s.
//!
//! This is the exact pattern used by `twig-lexer/build.rs`; see that
//! file for the rationale.  In short:
//!
//! 1. **No runtime file I/O** — the grammar is baked into the binary,
//!    so the crate ships standalone and runs under Miri's filesystem
//!    sandbox.
//! 2. **Build-time validation** — a malformed `.tokens` file fails
//!    `cargo build`, not the first lexer call.
//! 3. **One construction per process** — `lib.rs` wraps the generated
//!    constructor in a `OnceLock`.

use std::env;
use std::fs;
use std::path::PathBuf;

use grammar_tools::compiler::compile_token_grammar;
use grammar_tools::token_grammar::parse_token_grammar;

fn main() {
    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo");
    // code/packages/rust/mccarthy-lisp-lexer → ../../../grammars
    let grammar_path = PathBuf::from(&manifest_dir)
        .join("..")
        .join("..")
        .join("..")
        .join("grammars")
        .join("mccarthy_lisp.tokens");
    let grammar_path = grammar_path.canonicalize().unwrap_or_else(|e| {
        panic!("Failed to resolve mccarthy_lisp.tokens path {grammar_path:?}: {e}")
    });

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", grammar_path.display());

    let grammar_text = fs::read_to_string(&grammar_path)
        .unwrap_or_else(|e| panic!("Failed to read {grammar_path:?}: {e}"));
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse {grammar_path:?}: {e}"));

    let rust = compile_token_grammar(&grammar, "mccarthy_lisp.tokens");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let out_path = PathBuf::from(&out_dir).join("mccarthy_lisp_token_grammar.rs");
    fs::write(&out_path, rust)
        .unwrap_or_else(|e| panic!("Failed to write {out_path:?}: {e}"));
}
