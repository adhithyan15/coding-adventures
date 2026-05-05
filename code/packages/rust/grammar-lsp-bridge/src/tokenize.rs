//! Tokenize a source string using the grammar-tools GrammarLexer.
//!
//! ## Implementation plan (LS02 PR A)
//!
//! 1. Construct `grammar_tools::GrammarLexer::from_source(spec.tokens_source)`.
//!    Check the actual grammar-tools public API for the constructor name.
//!    File: code/packages/rust/grammar-tools/src/lib.rs
//!
//! 2. Run the lexer over `source` → Vec<grammar_tools::LexToken>.
//!
//! 3. Map each LexToken to ls00::Token:
//!    - kind: look up LexToken.kind in spec.token_kind_map → Some(lsp_type_string)
//!            or None if unmapped
//!    - text: LexToken.text
//!    - line: LexToken.line (check 0-based vs 1-based)
//!    - col:  LexToken.column (check 0-based vs 1-based)
//!
//! 4. Unrecognised characters from the lexer → emit as Diagnostic errors.
//!
//! 5. Return (Vec<ls00::Token>, Vec<ls00::Diagnostic>).

use crate::LanguageSpec;

/// Tokenize `source` using the lexical grammar in `spec`.
///
/// Returns `(tokens, lex_errors)`. Lex errors are surfaced as diagnostics
/// but do not prevent the parse step from running.
///
/// ## TODO — implement (LS02 PR A)
pub fn run(
    spec: &'static LanguageSpec,
    source: &str,
) -> (Vec<()>, Vec<()>) {
    // TODO: replace () with ls00::Token and ls00::Diagnostic once the
    // ls00 types are imported. See module doc for implementation plan.
    let _ = (spec, source);
    (vec![], vec![])
}
