//! # twig-lexer — thin wrapper over the generic `GrammarLexer`.
//!
//! Twig's tokens are defined in `code/grammars/twig.tokens` (the canonical
//! grammar file shared with the Python implementation and any future
//! language-frontend port).  This crate is the Rust binding to that file —
//! it loads the grammar at runtime via [`grammar_tools::token_grammar::parse_token_grammar`]
//! and constructs a [`lexer::grammar_lexer::GrammarLexer`] over it.
//!
//! The same pattern is used by every other Rust language frontend in this
//! repo (brainfuck, dartmouth-basic, …); see
//! [`code/packages/rust/brainfuck/src/lexer.rs`](../brainfuck/src/lexer.rs)
//! for the canonical reference.
//!
//! ## Why a wrapper, not a hand-written tokenizer?
//!
//! - **Single source of truth.**  Every Twig implementation (Python, Rust,
//!   any future Ruby/Go/etc.) reads the same `twig.tokens` file.
//!   Hand-writing the lexer would fork the grammar into a second
//!   implementation that could drift silently.
//! - **Shared infrastructure tests.**  `grammar-tools` and `lexer` have
//!   their own test suites; a wrapper inherits that coverage for free.
//! - **Less code, fewer bugs.**  The token grammar is data; the lexer
//!   logic is the same for every language.
//!
//! ## Token kinds produced (per `twig.tokens`)
//!
//! | Grammar name | `Token.type_`  | `Token.type_name`     |
//! |--------------|----------------|-----------------------|
//! | `LPAREN`     | `LParen`       | `None`                |
//! | `RPAREN`     | `RParen`       | `None`                |
//! | `QUOTE`      | `Name`         | `Some("QUOTE")`       |
//! | `BOOL_TRUE`  | `Name`         | `Some("BOOL_TRUE")`   |
//! | `BOOL_FALSE` | `Name`         | `Some("BOOL_FALSE")`  |
//! | `INTEGER`    | `Number`       | `None`                |
//! | `KEYWORD`    | `Keyword`      | `Some("KEYWORD")`     |
//! | `NAME`       | `Name`         | `None`                |
//! |              | `Eof`          | `None`                |
//!
//! Whitespace and `;`-to-end-of-line comments are silently skipped — they
//! never appear in the output.  Position tracking is 1-indexed.
//!
//! ## Example
//!
//! ```no_run
//! use twig_lexer::tokenize_twig;
//!
//! let tokens = tokenize_twig("(define x 42)").unwrap();
//! // [LParen, Keyword("define"), Name("x"), Integer("42"), RParen, Eof]
//! assert_eq!(tokens.len(), 6);
//! ```

use std::sync::OnceLock;

use grammar_tools::token_grammar::TokenGrammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

// Re-export the lexer error type so callers can propagate it without
// dragging the `lexer` crate into their dependencies just for the type.
pub use lexer::token::LexerError;

// ---------------------------------------------------------------------------
// Generated grammar (build.rs → grammar-tools::compiler → Rust)
// ---------------------------------------------------------------------------
//
// Earlier drafts called `std::fs::read_to_string` on
// `code/grammars/twig.tokens` at every `create_twig_lexer` call.  That
// had three problems:
//
//   1. Runtime file dependency — the deployed crate needed the
//      grammar file at a known path on the host filesystem.
//   2. Per-call parse cost — `parse_token_grammar` ran on every
//      `create_twig_lexer`.
//   3. Miri sandbox incompatibility — Miri's default isolation mode
//      blocks file-system access; running the test suite under Miri
//      required `-Zmiri-disable-isolation`, defeating the sandbox.
//
// The fix: `build.rs` invokes
// [`grammar_tools::compiler::compile_token_grammar`] at build time
// to emit Rust source code that materialises the parsed
// `TokenGrammar` as native struct literals.  The generated file
// defines a `pub fn token_grammar() -> TokenGrammar` that constructs
// the grammar from those literals.  We `include!` it inside a private
// module and wrap the constructor in a `OnceLock<TokenGrammar>` so
// the struct is materialised exactly **once** per process — every
// `create_twig_lexer` call after the first is a pointer load.
//
// The choice of `grammar_tools::compiler` (existing, shared) over
// the custom `codegen` module an earlier draft of this PR added is
// intentional: there is one canonical grammar-to-Rust compiler in
// the repo, and twig joins it.

mod generated_grammar {
    // The build.rs writes this file; its contents define
    // `pub fn token_grammar() -> TokenGrammar`.  The generated
    // file already includes its own `use` statements for the
    // grammar struct types and `HashMap`, so this module is
    // intentionally empty other than the include.
    include!(concat!(env!("OUT_DIR"), "/twig_token_grammar.rs"));
}

/// One-time-materialised Twig token grammar.
///
/// `OnceLock` ensures the generated `token_grammar()` constructor
/// runs at most once — even though it produces struct literals
/// (no parsing), constructing a `Vec` + `HashMap` on every lexer
/// call would still be wasteful.
static TWIG_TOKEN_GRAMMAR: OnceLock<TokenGrammar> = OnceLock::new();

fn twig_token_grammar() -> &'static TokenGrammar {
    TWIG_TOKEN_GRAMMAR.get_or_init(generated_grammar::token_grammar)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a [`GrammarLexer`] configured for Twig source.
///
/// Uses the build-time-compiled Twig token grammar — no filesystem
/// access at runtime.  Use this when you want access to the lexer
/// object itself (e.g. for incremental tokenisation or custom
/// error handling); otherwise reach for [`tokenize_twig`].
pub fn create_twig_lexer(source: &str) -> GrammarLexer<'_> {
    // GrammarLexer borrows the grammar; the build.rs-generated
    // static reference is `'static` so the lexer can outlive any
    // local scope.  One parsed grammar shared across all calls —
    // zero per-call allocation, zero file I/O.
    GrammarLexer::new(source, twig_token_grammar())
}

/// Tokenise Twig source into a `Vec<Token>` ending with EOF.
///
/// Whitespace and `;`-to-end-of-line comments are silently consumed.
/// Position tracking on each token is 1-indexed `(line, column)`.
///
/// # Errors
///
/// Returns a [`LexerError`] if the source contains a character outside
/// every token's valid set — most often a stray `@`, `~`, `:`, an
/// ASCII control character, or non-ASCII Unicode.  Callers handling
/// untrusted input MUST handle this `Err` case; the previous panicking
/// version was a DoS vector (single-character adversarial inputs would
/// abort the process).
///
/// # Panics
///
/// Panics only if the grammar file is missing/malformed (broken
/// checkout, not a runtime input issue).
pub fn tokenize_twig(source: &str) -> Result<Vec<Token>, LexerError> {
    let mut lex = create_twig_lexer(source);
    lex.tokenize()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
//
// Tests assert on the `effective_type_name()` of each token because the
// GrammarLexer maps grammar token names to a mix of [`TokenType`] enum
// variants (LPAREN, RPAREN, Number, Name, Keyword) and string `type_name`
// fields (BOOL_TRUE, BOOL_FALSE, QUOTE).  `effective_type_name` papers
// over that split.

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    fn type_names(source: &str) -> Vec<String> {
        tokenize_twig(source)
            .unwrap_or_else(|e| panic!("test source must lex: {e}"))
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.effective_type_name().to_string())
            .collect()
    }

    fn values(source: &str) -> Vec<String> {
        tokenize_twig(source)
            .unwrap_or_else(|e| panic!("test source must lex: {e}"))
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.value.clone())
            .collect()
    }

    fn lex_unwrap(source: &str) -> Vec<lexer::token::Token> {
        tokenize_twig(source).unwrap_or_else(|e| panic!("test source must lex: {e}"))
    }

    // -- Empty input is just EOF --

    #[test]
    fn empty_source_only_eof() {
        let toks = lex_unwrap("");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].type_, TokenType::Eof);
    }

    // -- Punctuation --

    #[test]
    fn parens_and_quote() {
        assert_eq!(type_names("(')"), vec!["LPAREN", "QUOTE", "RPAREN"]);
    }

    // -- Atoms --

    #[test]
    fn integer_literal() {
        assert_eq!(type_names("42"), vec!["INTEGER"]);
        assert_eq!(values("42"), vec!["42"]);
    }

    #[test]
    fn negative_integer_literal() {
        assert_eq!(type_names("-7"), vec!["INTEGER"]);
        assert_eq!(values("-7"), vec!["-7"]);
    }

    #[test]
    fn bool_true_and_false_distinct() {
        assert_eq!(type_names("#t"), vec!["BOOL_TRUE"]);
        assert_eq!(type_names("#f"), vec!["BOOL_FALSE"]);
    }

    #[test]
    fn name_token() {
        assert_eq!(type_names("foo"), vec!["NAME"]);
        assert_eq!(values("foo"), vec!["foo"]);
    }

    #[test]
    fn operator_names_lex_as_name() {
        for op in ["+", "-", "*", "/", "=", "<", ">", "null?", "pair?"] {
            assert_eq!(type_names(op), vec!["NAME"], "{op} should lex as NAME");
        }
    }

    // -- Keyword promotion --

    #[test]
    fn keywords_promoted_from_name_to_keyword() {
        for kw in ["define", "lambda", "let", "if", "begin", "quote", "nil"] {
            let toks = lex_unwrap(kw);
            assert_eq!(
                toks[0].type_,
                TokenType::Keyword,
                "{kw} should be promoted to KEYWORD"
            );
            assert_eq!(toks[0].value, kw);
        }
    }

    #[test]
    fn keyword_inside_form_keeps_its_kind() {
        let toks = lex_unwrap("(define x 42)");
        assert_eq!(toks[1].type_, TokenType::Keyword);
        assert_eq!(toks[1].value, "define");
    }

    // -- Disambiguation: bare `-` is a NAME --

    #[test]
    fn bare_minus_lexes_as_name() {
        let toks = lex_unwrap("(- 3 1)");
        let kinds: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| t.effective_type_name())
            .collect();
        assert_eq!(kinds, vec!["LPAREN", "NAME", "INTEGER", "INTEGER", "RPAREN"]);
        assert_eq!(toks[1].value, "-");
    }

    // -- Comments + whitespace --

    #[test]
    fn comment_to_eol_skipped() {
        assert_eq!(type_names("; comment\n42"), vec!["INTEGER"]);
    }

    #[test]
    fn whitespace_skipped() {
        assert_eq!(type_names("   42   "), vec!["INTEGER"]);
    }

    // -- Position tracking --

    #[test]
    fn first_token_is_at_1_1() {
        let toks = lex_unwrap("foo");
        assert_eq!((toks[0].line, toks[0].column), (1, 1));
    }

    #[test]
    fn newline_resets_column_and_advances_line() {
        let toks = lex_unwrap("a\nb");
        assert_eq!((toks[0].line, toks[0].column), (1, 1));
        assert_eq!((toks[1].line, toks[1].column), (2, 1));
    }

    // -- Realistic shapes --

    #[test]
    fn factorial_tokenises_cleanly() {
        let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))";
        let kinds = type_names(src);
        // Sanity: contains `define` and `if` keywords plus a sprinkle of NAMEs/NUMBERs.
        assert!(kinds.contains(&"KEYWORD".to_string()));
        assert!(kinds.contains(&"NAME".to_string()));
        assert!(kinds.contains(&"INTEGER".to_string()));
    }

    #[test]
    fn quoted_symbol_tokenises() {
        let src = "(eq? 'foo)";
        let kinds = type_names(src);
        assert_eq!(kinds, vec!["LPAREN", "NAME", "QUOTE", "NAME", "RPAREN"]);
    }

    // -- Adversarial input returns Err, not a panic --

    #[test]
    fn stray_at_sign_returns_err() {
        // `@` is not in any twig.tokens pattern; a panicking lexer
        // would let an attacker abort the process by submitting a
        // single byte.  We require a structured error.
        assert!(tokenize_twig("@").is_err());
    }

    #[test]
    fn non_ascii_unicode_returns_err() {
        assert!(tokenize_twig("(+ 1 €)").is_err());
    }
}
