//! # C lexer — tokenizing the C integer-core subset (SIR27).
//!
//! The lexical layer of the `c-to-semantic-ir` frontend
//! ([SIR27](../../../specs/SIR27-c-to-semantic-ir.md)).  Like every language
//! frontend in this repo it does **not** hand-write tokenization: it loads the
//! compiled `c.tokens` grammar and feeds it to the generic [`GrammarLexer`].
//!
//! ```text
//! c.tokens              (grammar file on disk)
//!    │  grammar-tools    (parses .tokens → TokenGrammar, embedded in _grammar.rs)
//!    ▼
//! lexer::GrammarLexer   (tokenizes source using the TokenGrammar)
//! ```
//!
//! Directive markers (`#` and `##`) are exposed for the PREP01 C dialect.
//! The legacy source-input parser still ignores directive lines until that
//! dialect is connected. Fixed-width type names remain keywords in this subset.
//!
//! Public entry points:
//! - [`create_c_lexer`] — a configured [`GrammarLexer`] for fine control.
//! - [`tokenize_c`] — convenience `&str` → `Vec<Token>` (panics on error).
//! - [`try_tokenize_c`] — the fallible form.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;
use std::collections::HashSet;
use std::sync::OnceLock;

mod _grammar;

/// Create a [`GrammarLexer`] configured for the C subset.
pub fn create_c_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

/// Tokenize C `source` into a `Vec<Token>` (ending in EOF).  Panics on a
/// lexical error; use [`try_tokenize_c`] for the fallible form.
pub fn tokenize_c(source: &str) -> Vec<Token> {
    try_tokenize_c(source).unwrap_or_else(|e| panic!("C tokenization failed: {e}"))
}

/// Tokenize C `source`, returning a human-readable error string on failure.
pub fn try_tokenize_c(source: &str) -> Result<Vec<Token>, String> {
    create_c_lexer(source)
        .tokenize()
        .map_err(|e| format!("{e:?}"))
}

/// Whether the compiled C token grammar reserves `word` as a keyword.
///
/// The set is built once from the same generated grammar used by `try_tokenize_c`.
/// Macro paste can call this for every expansion without rebuilding the lexer.
pub fn is_c_keyword(word: &str) -> bool {
    static KEYWORDS: OnceLock<HashSet<String>> = OnceLock::new();
    KEYWORDS
        .get_or_init(|| {
            let grammar = _grammar::token_grammar();
            grammar
                .keywords
                .into_iter()
                .chain(grammar.reserved_keywords)
                .collect()
        })
        .contains(word)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `effective` type-name of each non-EOF token, for compact assertions.
    fn kinds(src: &str) -> Vec<String> {
        tokenize_c(src)
            .into_iter()
            .filter(|t| t.effective_type_name() != "EOF")
            .map(|t| t.effective_type_name().to_string())
            .collect()
    }

    fn values(src: &str) -> Vec<String> {
        tokenize_c(src)
            .into_iter()
            .filter(|t| t.effective_type_name() != "EOF")
            .map(|t| t.value.clone())
            .collect()
    }

    #[test]
    fn declaration_tokenizes() {
        // `int32_t` is a keyword, `x` a NAME, `=` EQ, the literal INT_LIT.
        assert_eq!(
            values("int32_t x = 5;"),
            vec!["int32_t", "x", "=", "5", ";"]
        );
    }

    #[test]
    fn multi_char_operators_win_over_single() {
        assert_eq!(
            kinds("a == b << c && d"),
            vec!["NAME", "EQ_EQ", "NAME", "SHL", "NAME", "AND_AND", "NAME"]
        );
    }

    #[test]
    fn directives_are_exposed_and_comments_are_skipped() {
        let src = "#include <stdint.h>\n// a line comment\nint x; /* block */ int y;";
        assert_eq!(
            values(src),
            vec!["#", "include", "<", "stdint", ".", "h", ">", "int", "x", ";", "int", "y", ";"]
        );
        assert_eq!(
            kinds("#define JOIN(a,b) a ## b"),
            vec![
                "HASH",
                "NAME",
                "NAME",
                "LPAREN",
                "NAME",
                "COMMA",
                "NAME",
                "RPAREN",
                "NAME",
                "HASH_HASH",
                "NAME"
            ]
        );
    }

    #[test]
    fn integer_literals_with_hex_and_suffix() {
        assert_eq!(
            values("0xFF 100u 10LL 42"),
            vec!["0xFF", "100u", "10LL", "42"]
        );
    }

    #[test]
    fn float_literals_are_recognized_and_do_not_shadow_ints() {
        // Fractions, leading/trailing dots, exponents, and `f` suffix all lex as
        // FLOAT_LIT; a plain integer still lexes as INT_LIT (FLOAT_LIT requires a
        // `.` or an exponent).
        assert_eq!(
            kinds("3.14 .5 10. 1e10 2.5e-3 1.0f 42"),
            vec![
                "FLOAT_LIT",
                "FLOAT_LIT",
                "FLOAT_LIT",
                "FLOAT_LIT",
                "FLOAT_LIT",
                "FLOAT_LIT",
                "INT_LIT"
            ]
        );
    }

    #[test]
    fn float_and_double_are_keywords() {
        for kw in ["float", "double"] {
            assert_eq!(kinds(kw), vec!["KEYWORD"], "for `{kw}`");
        }
    }

    #[test]
    fn char_and_string_literals() {
        let ks = kinds("putchar('A'); printf(\"%d\\n\", x);");
        assert!(ks.contains(&"CHAR_LIT".to_string()), "got: {ks:?}");
        assert!(ks.contains(&"STR_LIT".to_string()), "got: {ks:?}");
    }

    #[test]
    fn stdint_names_are_keywords_not_names() {
        // Each fixed-width type is a keyword token (value == the word), never a
        // NAME — so the grammar sees a type, not an identifier.
        for kw in ["uint8_t", "int64_t", "size_t", "unsigned"] {
            assert_eq!(kinds(kw), vec!["KEYWORD"], "for `{kw}`");
        }
    }

    #[test]
    fn cached_keyword_lookup_agrees_with_the_compiled_c_grammar() {
        for keyword in _grammar::token_grammar().keywords {
            assert!(is_c_keyword(&keyword), "{keyword}");
            assert_eq!(kinds(&keyword), vec!["KEYWORD"], "{keyword}");
        }
        for name in ["preNAME", "int32_tail", "_name"] {
            assert!(!is_c_keyword(name), "{name}");
            assert_eq!(kinds(name), vec!["NAME"], "{name}");
        }
    }
}
