//! # prolog-lexer — ISO/Core Prolog tokenizer.
//!
//! Thin glue layer around the grammar-driven [`lexer::grammar_lexer::GrammarLexer`].
//! The token vocabulary is loaded from a compiled embedding of
//! `code/grammars/prolog/iso.tokens`. The actual recognition machinery
//! is provided by `grammar-tools` + `lexer`; this crate only wires them
//! together for the Prolog `.tokens` file.
//!
//! ## Architecture
//!
//! ```text
//!    code/grammars/prolog/iso.tokens          (canonical, human-readable)
//!         │
//!         │  cargo run -p prolog-lexer --example regenerate_grammar
//!         ▼
//!    src/_grammar.rs                           (auto-generated embedding)
//!         │
//!         ▼
//!    GrammarLexer                              (from `lexer` crate)
//!         │
//!         ▼
//!    Vec<Token>                                (from `lexer::token`)
//! ```
//!
//! This is the Rust mirror of the Python `iso-prolog-lexer` package,
//! which uses the same pipeline (Python `grammar_tools` +
//! `GrammarLexer`). Both implementations source from the same
//! `iso.tokens` file, so their token streams agree by construction.
//!
//! ## Regenerating the embedded grammar
//!
//! When `iso.tokens` changes, regenerate `src/_grammar.rs`:
//!
//! ```sh
//! cargo run -p prolog-lexer --example regenerate_grammar
//! ```
//!
//! The generated file is checked into the repository to avoid file
//! I/O at startup and to keep this crate's deployment independent of
//! the grammars directory.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

/// Build a `GrammarLexer` configured with the ISO Prolog token grammar.
///
/// The returned lexer is ready to call `.tokenize()` on. Use this when
/// you want fine-grained control (custom hooks, incremental
/// tokenization) instead of the convenience function below.
pub fn create_iso_prolog_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    // GrammarLexer holds a reference to the grammar for its lifetime;
    // here we leak the grammar via a static so that the returned lexer
    // can outlive this function. The grammar is small and the leak is
    // intentional (the data is process-lifetime by construction).
    let grammar: &'static _ = Box::leak(Box::new(grammar));
    GrammarLexer::new(source, grammar)
}

/// Tokenize an ISO/Core Prolog source string.
///
/// Returns a `Vec<Token>` ending in an `Eof` token, matching the
/// convention of every other `*-lexer` crate in this workspace. Errors
/// from the underlying `GrammarLexer` propagate as a panic — this is
/// the same shape the other grammar-driven lexers in this workspace
/// adopt. Callers that need recoverable errors should use
/// `create_iso_prolog_lexer(...).tokenize()` directly and pattern-match
/// the `Result`.
pub fn tokenize_iso_prolog(source: &str) -> Vec<Token> {
    let mut lexer = create_iso_prolog_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("ISO Prolog tokenization failed: {e}"))
}

// ---------------------------------------------------------------------------
// Tests — exercise the surface that downstream parsers and tools will use
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    /// Helper: collect (effective_type_name, value) pairs excluding
    /// the trailing EOF. `effective_type_name()` returns the
    /// grammar's uppercase name (`ATOM`, `VARIABLE`, `LPAREN`, etc.)
    /// whether or not the token mapped to a built-in `TokenType`.
    fn pairs(source: &str) -> Vec<(String, String)> {
        let toks = tokenize_iso_prolog(source);
        toks.into_iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.effective_type_name().to_string(), t.value.clone()))
            .collect()
    }

    #[test]
    fn empty_source_yields_only_eof() {
        let toks = tokenize_iso_prolog("");
        // Always at least EOF.
        assert_eq!(toks.last().map(|t| &t.type_), Some(&TokenType::Eof));
        // Nothing before EOF.
        let non_eof: Vec<_> = toks.iter().filter(|t| t.type_ != TokenType::Eof).collect();
        assert!(non_eof.is_empty());
    }

    #[test]
    fn whitespace_and_comments_are_skipped() {
        let ps = pairs("   % a comment\n   ");
        assert!(ps.is_empty());
    }

    #[test]
    fn simple_fact_tokens_in_order() {
        let ps = pairs("father(homer, bart).");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        // The grammar produces uppercase token names like ATOM, LPAREN, etc.
        // Order should be: ATOM('father'), LPAREN, ATOM('homer'), COMMA, ATOM('bart'), RPAREN, DOT
        assert_eq!(
            names,
            vec!["ATOM", "LPAREN", "ATOM", "COMMA", "ATOM", "RPAREN", "DOT"]
        );
        // Values should match the surface text.
        let values: Vec<&str> = ps.iter().map(|(_, v)| v.as_str()).collect();
        assert_eq!(
            values,
            vec!["father", "(", "homer", ",", "bart", ")", "."]
        );
    }

    #[test]
    fn rule_arrow_is_recognized_as_its_own_token() {
        let ps = pairs("a :- b.");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["ATOM", "RULE", "ATOM", "DOT"]);
    }

    #[test]
    fn query_arrow_is_recognized_as_its_own_token() {
        let ps = pairs("?- foo(X).");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec!["QUERY", "ATOM", "LPAREN", "VARIABLE", "RPAREN", "DOT"]
        );
    }

    #[test]
    fn dcg_arrow_is_three_characters() {
        let ps = pairs("greet --> [hello].");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"DCG"));
    }

    #[test]
    fn integer_and_float_literals() {
        let ps = pairs("42 3.14 2.5e-3");
        // Names produced by the grammar:
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["INTEGER", "FLOAT", "FLOAT"]);
    }

    #[test]
    fn integer_followed_by_dot_is_not_a_float() {
        // 42. is "integer 42, dot" — the grammar's FLOAT pattern requires
        // at least one digit after the dot.
        let ps = pairs("42.");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["INTEGER", "DOT"]);
    }

    #[test]
    fn anon_var_alone() {
        let ps = pairs("_");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["ANON_VAR"]);
    }

    #[test]
    fn underscore_led_variable_is_variable_not_anon() {
        let ps = pairs("_State");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["VARIABLE"]);
    }

    #[test]
    fn uppercase_identifier_is_variable() {
        let ps = pairs("X");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["VARIABLE"]);
    }

    #[test]
    fn lowercase_identifier_is_atom() {
        let ps = pairs("chest_pain");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["ATOM"]);
    }

    #[test]
    fn quoted_atom_token_kind_is_atom_via_alias() {
        // The .tokens grammar declares QUOTED_ATOM -> ATOM, so the
        // emitted token's effective kind name is ATOM.
        let ps = pairs("'Hello World'");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["ATOM"]);
    }

    #[test]
    fn symbolic_atom_via_alias() {
        // ATOM_SYMBOLIC -> ATOM, so e.g. `>=` is reported as ATOM.
        let ps = pairs(">=");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["ATOM"]);
    }

    #[test]
    fn list_brackets_and_pipe() {
        let ps = pairs("[H | T]");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec!["LBRACKET", "VARIABLE", "BAR", "VARIABLE", "RBRACKET"]
        );
    }

    #[test]
    fn cut_is_its_own_token() {
        let ps = pairs("!");
        let names: Vec<&str> = ps.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["CUT"]);
    }

    #[test]
    fn complete_small_program_tokenizes() {
        let src = "\
            father(homer, bart).\n\
            parent(X, Y) :- father(X, Y).\n\
            ?- parent(homer, Who).\n\
        ";
        let toks = tokenize_iso_prolog(src);
        // Sanity: at least the right counts of `RULE`, `QUERY`, `DOT`.
        let count = |label: &str| -> usize {
            toks.iter()
                .filter(|t| t.effective_type_name() == label)
                .count()
        };
        assert_eq!(count("RULE"), 1);
        assert_eq!(count("QUERY"), 1);
        assert_eq!(count("DOT"), 3);
        // The trailing EOF is always present.
        assert_eq!(toks.last().map(|t| &t.type_), Some(&TokenType::Eof));
    }
}
