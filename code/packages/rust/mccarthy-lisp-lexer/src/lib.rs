//! # `mccarthy-lisp-lexer` — tokenizer for McCarthy's 1960 Lisp.
//!
//! Lisp 1.0 (the language described in John McCarthy's 1960 paper
//! *"Recursive Functions of Symbolic Expressions and Their Computation
//! by Machine, Part I"*) has the simplest tokenizer of any production
//! language ever shipped — six token kinds, no string literals, no
//! operator symbols, no floats:
//!
//! | Grammar token | Source form        | `effective_type_name()` |
//! |---------------|--------------------|-------------------------|
//! | `LPAREN`      | `(`                | `"LPAREN"`              |
//! | `RPAREN`      | `)`                | `"RPAREN"`              |
//! | `QUOTE`       | `'`                | `"QUOTE"`               |
//! | `DOT`         | `.`                | `"DOT"`                 |
//! | `SYMBOL`      | `[A-Z][A-Z0-9-]*`  | `"SYMBOL"`              |
//! | `INTEGER`     | `-?[0-9]+`         | `"INTEGER"`             |
//!
//! Plus whitespace and `;`-to-EOL comments, which are skipped.
//!
//! ## This crate is a *thin wrapper*, not a hand-written lexer
//!
//! Every Lisp 1.0 lexing rule lives in
//! [`code/grammars/mccarthy_lisp.tokens`](../../../grammars/mccarthy_lisp.tokens),
//! which `build.rs` compiles to Rust at build time.  This module just
//! materialises that grammar once (via a `OnceLock`) and hands it to
//! the shared [`lexer::grammar_lexer::GrammarLexer`].  The same pattern
//! is used by `twig-lexer`, `nib-lexer`, and `oct-lexer` — see
//! [`feedback_no_handwritten_lexers_parsers`].  Hand-writing the
//! tokenizer would fork the grammar into a second implementation that
//! could silently drift.
//!
//! ## Why a distinct crate vs reusing `lisp-lexer`
//!
//! The in-tree `lisp-lexer` (and its grammar, `lisp.tokens`) targets a
//! modern Scheme-ish dialect: lowercase symbols, strings, decimals,
//! operator symbols (`+`, `<=`, `null?`).  None of those existed in
//! 1960.  The McCarthy grammar enforces the all-uppercase,
//! integers-only, no-strings dialect at the token level, keeping the
//! downstream `mccarthy-lisp-parser` and `mccarthy-lisp-iir-compiler`
//! honest about which Lisp they consume.
//!
//! ## How the dialect rules fall out of the grammar
//!
//! - **Negative integers** — `INTEGER = /-?[0-9]+/` requires a digit,
//!   so `-1` is one token while a bare `-` matches nothing and is a lex
//!   error.  That *is* McCarthy 1.0's "no operator symbols" rule.
//! - **All-uppercase symbols** — `SYMBOL = /[A-Z][A-Z0-9-]*/` excludes
//!   lowercase, so lowercase source is a lex error.
//!
//! ## Quick start
//!
//! ```
//! use mccarthy_lisp_lexer::tokenize_mccarthy;
//!
//! // The canonical "first Lisp program" — McCarthy 1960 §3.
//! let toks = tokenize_mccarthy("(CAR '(A B C))").expect("tokenize");
//! let names: Vec<&str> = toks
//!     .iter()
//!     .map(|t| t.effective_type_name())
//!     .collect();
//! assert_eq!(
//!     names,
//!     vec![
//!         "LPAREN", "SYMBOL", "QUOTE", "LPAREN",
//!         "SYMBOL", "SYMBOL", "SYMBOL", "RPAREN", "RPAREN", "EOF",
//!     ]
//! );
//! ```

#![warn(missing_docs)]

use std::sync::OnceLock;

use grammar_tools::token_grammar::TokenGrammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

// Re-export the shared token types so callers (notably
// `mccarthy-lisp-parser`) can name them without taking a direct
// dependency on the `lexer` crate.
pub use lexer::token::{LexerError, Token as LispToken, TokenType};

// ---------------------------------------------------------------------------
// Build-time-compiled token grammar
// ---------------------------------------------------------------------------
//
// `build.rs` writes `$OUT_DIR/mccarthy_lisp_token_grammar.rs`, whose
// body defines `pub fn token_grammar() -> TokenGrammar` from native
// struct literals.  We `include!` it inside a private module and cache
// the result in a `OnceLock` so the `Vec`/`HashMap` construction runs
// at most once per process — every `tokenize_mccarthy` call after the
// first is a pointer load.

mod generated_grammar {
    include!(concat!(env!("OUT_DIR"), "/mccarthy_lisp_token_grammar.rs"));
}

static MCCARTHY_TOKEN_GRAMMAR: OnceLock<TokenGrammar> = OnceLock::new();

fn mccarthy_token_grammar() -> &'static TokenGrammar {
    MCCARTHY_TOKEN_GRAMMAR.get_or_init(generated_grammar::token_grammar)
}

/// Borrow the build-time-compiled McCarthy Lisp [`TokenGrammar`].
///
/// Exposed for tooling (LSP servers, syntax-highlight generators) that
/// wants to introspect the token set without re-parsing the canonical
/// `.tokens` file — which is a build-time-only artifact and is not
/// shipped at runtime.
pub fn mccarthy_token_grammar_spec() -> &'static TokenGrammar {
    mccarthy_token_grammar()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Construct a [`GrammarLexer`] over a McCarthy Lisp source string.
///
/// Most callers want [`tokenize_mccarthy`]; reach for this when you
/// need the lexer object itself (e.g. for incremental / streaming use).
pub fn create_mccarthy_lexer(source: &str) -> GrammarLexer<'_> {
    GrammarLexer::new(source, mccarthy_token_grammar())
}

/// Tokenize a McCarthy 1960 Lisp source string.
///
/// Returns the full token stream including the trailing `EOF` token, as
/// produced by the shared [`GrammarLexer`].  Whitespace and `;` line
/// comments are skipped.
///
/// # Errors
///
/// Returns [`LexerError`] on the first byte that matches no token rule —
/// e.g. a lowercase letter, a bare `-`, or any non-Lisp character.  The
/// error carries the 1-based line and column.
pub fn tokenize_mccarthy(source: &str) -> Result<Vec<Token>, LexerError> {
    create_mccarthy_lexer(source).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Token names (minus the trailing EOF) for a source string.
    fn names(src: &str) -> Vec<String> {
        let toks = tokenize_mccarthy(src).expect("tokenize");
        toks.iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.effective_type_name().to_string())
            .collect()
    }

    #[test]
    fn tokenizes_the_canonical_car_example() {
        assert_eq!(
            names("(CAR '(A B C))"),
            vec![
                "LPAREN", "SYMBOL", "QUOTE", "LPAREN", "SYMBOL", "SYMBOL", "SYMBOL", "RPAREN",
                "RPAREN",
            ]
        );
    }

    #[test]
    fn values_are_preserved() {
        let toks = tokenize_mccarthy("(CONS A NIL)").unwrap();
        let values: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(values, vec!["(", "CONS", "A", "NIL", ")"]);
    }

    #[test]
    fn negative_integers_are_one_token() {
        assert_eq!(names("-42"), vec!["INTEGER"]);
        let toks = tokenize_mccarthy("-42").unwrap();
        assert_eq!(toks[0].value, "-42");
    }

    #[test]
    fn dotted_pair_separator() {
        assert_eq!(names("(A . B)"), vec!["LPAREN", "SYMBOL", "DOT", "SYMBOL", "RPAREN"]);
    }

    #[test]
    fn hyphenated_symbol_is_one_token() {
        assert_eq!(names("LIST-OF-3"), vec!["SYMBOL"]);
        assert_eq!(tokenize_mccarthy("LIST-OF-3").unwrap()[0].value, "LIST-OF-3");
    }

    #[test]
    fn comments_and_whitespace_are_skipped() {
        assert_eq!(names("  CAR ; take the head\n  CDR"), vec!["SYMBOL", "SYMBOL"]);
    }

    #[test]
    fn bare_minus_is_a_lex_error() {
        // No operator symbols in Lisp 1.0 — a lone `-` matches no rule.
        assert!(tokenize_mccarthy("(- A B)").is_err());
    }

    #[test]
    fn lowercase_is_a_lex_error() {
        // McCarthy 1960 Lisp is all-uppercase.
        assert!(tokenize_mccarthy("car").is_err());
    }

    #[test]
    fn position_tracking_is_one_based() {
        let toks = tokenize_mccarthy("CAR\n CDR").unwrap();
        assert_eq!((toks[0].line, toks[0].column), (1, 1));
        assert_eq!((toks[1].line, toks[1].column), (2, 2));
    }

    #[test]
    fn empty_source_yields_only_eof() {
        let toks = tokenize_mccarthy("   ; just a comment\n").unwrap();
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].type_, TokenType::Eof);
    }

    #[test]
    fn grammar_spec_is_accessible() {
        let names: Vec<&str> = mccarthy_token_grammar_spec()
            .definitions
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        assert!(names.contains(&"SYMBOL"));
        assert!(names.contains(&"INTEGER"));
    }
}
