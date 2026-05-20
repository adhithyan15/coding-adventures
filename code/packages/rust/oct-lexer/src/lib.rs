//! # Oct lexer — OCT02 phase 1.
//!
//! Tokenizes Oct source text using the grammar-driven Rust lexer.  Thin
//! wrapper around the generic `GrammarLexer` over the auto-generated
//! token grammar in `_grammar.rs` (compiled from
//! `code/grammars/oct.tokens` via the `grammar-tools` CLI).
//!
//! Oct is the typed 8-bit systems language originally targeting the
//! Intel 8008.  This crate is the first step of OCT02 — bringing Oct's
//! frontend into Rust so the LANG VM AOT chain can compile `.oct`
//! files in V2 phases (3 + 4).  Phase 2 (`oct-type-checker`) and
//! phase 3 (`oct-iir-compiler`) follow in subsequent PRs.
//!
//! ## Usage
//!
//! ```
//! use coding_adventures_oct_lexer::tokenize_oct;
//!
//! let tokens = tokenize_oct("fn main() { let x: u8 = 5; }");
//! assert!(tokens.iter().any(|t| t.value == "fn"));
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use grammar_tools::token_grammar::TokenGrammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

fn grammar() -> TokenGrammar {
    _grammar::token_grammar()
}

/// Create a `GrammarLexer` over an Oct source string.  Most callers
/// want [`tokenize_oct`] instead; this is the lower-level entry point.
pub fn create_oct_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize an Oct source string.  Panics on lex errors — V2 will
/// expose a `Result`-returning variant once the type-checker is wired.
pub fn tokenize_oct(source: &str) -> Vec<Token> {
    let grammar = grammar();
    let mut lexer = GrammarLexer::new(source, &grammar);
    let mut tokens = lexer
        .tokenize()
        .unwrap_or_else(|err| panic!("Oct tokenization failed: {err}"));

    // Promote `KEYWORD` tokens so their `type_name` matches the keyword
    // value — same convention as Nib's lexer.  This lets the grammar's
    // quoted-keyword rules (`"fn"`, `"let"`, etc.) match by type name.
    for token in &mut tokens {
        if token.type_name.as_deref() == Some("KEYWORD") {
            token.type_name = Some(token.value.clone());
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_simple_function() {
        let tokens = tokenize_oct("fn main() { let x: u8 = 5; }");
        assert!(tokens.iter().any(|t| t.value == "fn"));
        assert!(tokens.iter().any(|t| t.value == "main"));
        assert!(tokens.iter().any(|t| t.value == "5"));
        assert!(tokens.iter().any(|t| t.value == "u8"));
    }

    #[test]
    fn keyword_type_names_are_promoted() {
        let tokens = tokenize_oct("fn main() {}");
        let fn_tok = tokens.iter().find(|t| t.value == "fn").unwrap();
        assert_eq!(fn_tok.type_name.as_deref(), Some("fn"));
    }

    #[test]
    fn tokenizes_intrinsics() {
        // Oct's 8008 intrinsics: `out`, `in`, `carry` etc. all show up
        // as KEYWORD tokens with their literal value.
        let tokens = tokenize_oct("fn t() { out(1, 0); let c: bool = carry(); }");
        assert!(tokens.iter().any(|t| t.value == "out"));
        assert!(tokens.iter().any(|t| t.value == "carry"));
    }

    #[test]
    fn tokenizes_arithmetic_and_relops() {
        let tokens = tokenize_oct("fn t() { let x: u8 = 1 + 2; if x == 3 { x = x - 1; } }");
        assert!(tokens.iter().any(|t| t.value == "+"));
        assert!(tokens.iter().any(|t| t.value == "=="));
        assert!(tokens.iter().any(|t| t.value == "-"));
    }

    #[test]
    fn tokenizes_loop_and_break() {
        let tokens = tokenize_oct("fn t() { loop { break; } }");
        assert!(tokens.iter().any(|t| t.value == "loop"));
        assert!(tokens.iter().any(|t| t.value == "break"));
    }
}
