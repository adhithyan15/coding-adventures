//! Regenerate `src/_grammar.rs` from `code/grammars/prolog/iso.tokens`.
//!
//! Run with:
//!
//! ```sh
//! cargo run -p prolog-lexer --example regenerate_grammar
//! ```
//!
//! This reads the canonical ISO Prolog token grammar and writes the
//! compiled Rust embedding to `src/_grammar.rs`. The output is checked
//! into the repository so downstream users do not need file I/O at
//! startup.

use std::fs;
use std::path::PathBuf;

use grammar_tools::compiler::compile_token_grammar;
use grammar_tools::token_grammar::{parse_token_grammar, TokenGrammar};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Walk up from the crate root to find code/grammars/prolog/iso.tokens.
    // Layout: <repo>/code/packages/rust/prolog-lexer/ → <repo>/code/grammars/prolog/iso.tokens
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let grammars_dir = crate_root
        .ancestors()
        .nth(3)
        .ok_or("could not locate the repo's `code/` parent")?
        .join("grammars");
    let tokens_path = grammars_dir.join("prolog").join("iso.tokens");

    let source = fs::read_to_string(&tokens_path)
        .map_err(|e| format!("could not read {}: {}", tokens_path.display(), e))?;
    let grammar = parse_token_grammar(&source)?;
    let grammar = patch_for_rust_regex(grammar);

    let rust_source = compile_token_grammar(&grammar, "iso.tokens");
    let out = crate_root.join("src").join("_grammar.rs");
    fs::write(&out, rust_source)?;

    println!(
        "wrote {} from {}",
        out.display(),
        tokens_path.display()
    );
    Ok(())
}

/// Apply Rust-specific transformations to the parsed grammar before
/// compiling.
///
/// The canonical `iso.tokens` is shared with the Python lexer, which
/// uses Python's `re` module — that module supports negative
/// look-ahead (e.g., `_(?![A-Za-z0-9_])` for `ANON_VAR`). The Rust
/// `regex` crate does **not** support look-around (a deliberate design
/// choice for guaranteed linear-time matching).
///
/// To keep the canonical grammar pristine and have the Rust crate
/// behave identically, we apply two local transformations after
/// parsing:
///
/// 1. **Strip the look-ahead from `ANON_VAR`**: the pattern becomes
///    just `_` (matching a single underscore).
/// 2. **Reorder so `VARIABLE` is tried before `ANON_VAR`**: the
///    `GrammarLexer` uses first-match-wins. With this order, `_State`
///    matches `VARIABLE` (because `VARIABLE`'s `_[A-Za-z0-9_]+`
///    alternative consumes the whole identifier), and `_` alone
///    matches `ANON_VAR` (because `VARIABLE` requires at least one
///    continuation char after the underscore). The result is
///    semantically identical to the look-ahead version.
///
/// If `iso.tokens` ever drops look-around (e.g., the canonical grammar
/// is harmonized across implementations), this function can be deleted
/// without behavioural change.
fn patch_for_rust_regex(mut grammar: TokenGrammar) -> TokenGrammar {
    // 1. Strip look-ahead from ANON_VAR.
    for def in grammar.definitions.iter_mut() {
        if def.name == "ANON_VAR" {
            def.pattern = "_".to_string();
        }
    }

    // 2. Reorder: VARIABLE before ANON_VAR. Find each index; if both
    //    exist and VARIABLE is after ANON_VAR, swap them.
    let anon_idx = grammar
        .definitions
        .iter()
        .position(|d| d.name == "ANON_VAR");
    let var_idx = grammar
        .definitions
        .iter()
        .position(|d| d.name == "VARIABLE");
    if let (Some(a), Some(v)) = (anon_idx, var_idx) {
        if a < v {
            grammar.definitions.swap(a, v);
        }
    }

    grammar
}
