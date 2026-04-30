//! # twig-lexer build script — compile twig.tokens into Rust at build time.
//!
//! Reads `code/grammars/twig.tokens`, parses it via
//! `grammar_tools::token_grammar::parse_token_grammar`, runs
//! `grammar_tools::codegen::token_grammar_to_rust_source` to emit Rust
//! source code that reconstructs the parsed `TokenGrammar`, and writes
//! the result to `$OUT_DIR/twig_token_grammar.rs`.
//!
//! `lib.rs` `include!`s the generated file, exposing
//! `pub fn twig_token_grammar() -> &'static TokenGrammar`.
//!
//! ## Why a build.rs
//!
//! 1. **No runtime file I/O** — the grammar is baked into the binary.
//!    Critical for Miri (which sandboxes FS access) and for shipping
//!    the crate as a standalone library that doesn't ship a separate
//!    grammar file.
//! 2. **Build-time validation** — if `twig.tokens` is malformed, the
//!    error fires at `cargo build` time, not at the first lexer call.
//! 3. **One parse per build, not per lexer instance** — even with
//!    `OnceLock` caching, "parse once at startup" beats "parse at
//!    build time" for both speed and determinism.
//!
//! ## Cargo rerun signals
//!
//! `cargo:rerun-if-changed` tells Cargo to re-run this build script
//! whenever:
//!   - `build.rs` itself changes (Cargo's default; explicit here for clarity)
//!   - the grammar source file changes
//! Without these, edits to `twig.tokens` would not trigger a rebuild
//! of this crate, and downstream consumers would see stale grammar.

use std::env;
use std::fs;
use std::path::PathBuf;

use grammar_tools::codegen::token_grammar_to_rust_source;
use grammar_tools::token_grammar::parse_token_grammar;

fn main() {
    // Locate the grammar file.  CARGO_MANIFEST_DIR points at this
    // crate's directory; the grammars folder is three levels up at
    // `code/grammars/twig.tokens` per the repo layout.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set by cargo");
    let grammar_path = PathBuf::from(&manifest_dir)
        .join("..").join("..").join("..")
        .join("grammars").join("twig.tokens");
    let grammar_path = grammar_path
        .canonicalize()
        .unwrap_or_else(|e| panic!("Failed to resolve twig.tokens path {grammar_path:?}: {e}"));

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", grammar_path.display());

    let grammar_text = fs::read_to_string(&grammar_path)
        .unwrap_or_else(|e| panic!("Failed to read {grammar_path:?}: {e}"));
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse {grammar_path:?}: {e}"));

    // Generate the Rust source that reconstructs `grammar` and writes
    // it to OUT_DIR per the standard build-script convention.
    let rust = token_grammar_to_rust_source(&grammar, "twig_token_grammar");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let out_path = PathBuf::from(&out_dir).join("twig_token_grammar.rs");
    fs::write(&out_path, rust)
        .unwrap_or_else(|e| panic!("Failed to write {out_path:?}: {e}"));
}
