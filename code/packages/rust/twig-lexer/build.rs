//! # twig-lexer build script — compile twig.tokens into Rust at build time.
//!
//! Reads `code/grammars/twig.tokens`, parses it via
//! `grammar_tools::token_grammar::parse_token_grammar`, runs
//! `grammar_tools::compiler::compile_token_grammar` to emit Rust
//! source code that reconstructs the parsed `TokenGrammar`, and
//! writes the result to `$OUT_DIR/twig_token_grammar.rs`.
//!
//! `lib.rs` `include!`s the generated file inside a private module
//! and wraps the generated `token_grammar()` function in a
//! `OnceLock<TokenGrammar>` so the struct is materialised exactly
//! once per process — not on every `create_twig_lexer` call.
//!
//! ## Why a build.rs
//!
//! 1. **No runtime file I/O** — the grammar is baked into the
//!    binary.  Critical for Miri (which sandboxes FS access) and
//!    for shipping the crate as a standalone library.
//! 2. **Build-time validation** — if `twig.tokens` is malformed,
//!    the error fires at `cargo build` time, not at first lexer
//!    call.
//! 3. **One construction per process, not per call** — the
//!    OnceLock wrapper caches the parsed grammar; the generated
//!    `token_grammar()` body runs at most once.
//!
//! ## Cargo rerun signals
//!
//! `cargo:rerun-if-changed` tells Cargo to re-run this build
//! script whenever `build.rs` itself or the grammar file changes.

use std::env;
use std::fs;
use std::path::PathBuf;

use grammar_tools::compiler::compile_token_grammar;
use grammar_tools::token_grammar::parse_token_grammar;

fn main() {
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

    // `compile_token_grammar` produces Rust source that defines:
    //
    //     pub fn token_grammar() -> TokenGrammar { /* struct literal */ }
    //
    // The generated function constructs a fresh `TokenGrammar`
    // each call.  We wrap it in a `OnceLock` at the consumer
    // (lib.rs) so the construction runs exactly once per process.
    let rust = compile_token_grammar(&grammar, "twig.tokens");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let out_path = PathBuf::from(&out_dir).join("twig_token_grammar.rs");
    fs::write(&out_path, rust)
        .unwrap_or_else(|e| panic!("Failed to write {out_path:?}: {e}"));
}
