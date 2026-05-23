//! JSDoc lexer.
//!
//! Tokenizes the **interior** of a `/** ... */` block comment per CLOC05
//! §"`jsdoc-tokens` outline." The enclosing `/**` and `*/` markers and the
//! per-line `* ` continuation prefixes are expected to be stripped by an
//! upstream comment-extractor stage (deferred follow-up); this lexer is
//! tolerant of leftover `* ` line-starts via a skip pattern as a safety
//! net.
//!
//! The lexer is grammar-driven: the actual token definitions live in
//! [`code/grammars/jsdoc/jsdoc.tokens`](../../../grammars/jsdoc/jsdoc.tokens),
//! compiled to native Rust at build time via `grammar-tools compile-tokens`
//! and embedded as `mod _grammar`.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

/// Construct a JSDoc [`GrammarLexer`] over `source`. The caller is
/// responsible for stripping the surrounding `/** */` markers; see the
/// crate-level docs.
pub fn create_jsdoc_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

/// Convenience: tokenize `source` and return the produced tokens. Panics
/// on lexer error (use [`create_jsdoc_lexer`] + `tokenize()` for callers
/// that need to handle errors explicitly).
pub fn tokenize_jsdoc(source: &str) -> Vec<Token> {
    let mut lexer = create_jsdoc_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("JSDoc tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    /// Helper: collect owned `(type_name, value)` pairs minus the
    /// trailing EOF.
    ///
    /// Token names favor `type_name` when set (the grammar-driven name
    /// from `.tokens`), falling back to a mapping from the built-in
    /// `TokenType` enum for the well-known single-character tokens
    /// (`LBRACE`, `RBRACE`, `LBRACKET`, …). This lets tests assert
    /// against the SCREAMING_SNAKE_CASE names declared in `jsdoc.tokens`
    /// uniformly, without caring whether the lexer used a built-in or a
    /// custom token type.
    fn pairs(tokens: &[Token]) -> Vec<(String, String)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| {
                let name = t.type_name.clone().unwrap_or_else(|| match t.type_ {
                    TokenType::LBrace => "LBRACE".into(),
                    TokenType::RBrace => "RBRACE".into(),
                    TokenType::LBracket => "LBRACKET".into(),
                    TokenType::RBracket => "RBRACKET".into(),
                    TokenType::LParen => "LPAREN".into(),
                    TokenType::RParen => "RPAREN".into(),
                    TokenType::Comma => "COMMA".into(),
                    TokenType::Colon => "COLON".into(),
                    TokenType::Dot => "DOT".into(),
                    TokenType::Bang => "BANG".into(),
                    TokenType::Star => "STAR".into(),
                    TokenType::Equals => "EQUALS".into(),
                    TokenType::Newline => "NEWLINE".into(),
                    TokenType::Name => "NAME".into(),
                    TokenType::Number => "NUMBER".into(),
                    TokenType::String => "STRING".into(),
                    TokenType::Keyword => "KEYWORD".into(),
                    other => format!("{:?}", other),
                });
                (name, t.value.clone())
            })
            .collect()
    }

    #[test]
    fn tokenizes_type_tag() {
        // `@type {number}` is the simplest JSDoc tag — one tag, one
        // type expression with one primitive.
        let tokens = tokenize_jsdoc("@type {number}\n");
        let names: Vec<String> = pairs(&tokens).into_iter().map(|(n, _)| n).collect();
        assert!(names.contains(&"AT_TAG".to_string()));
        assert!(names.contains(&"LBRACE".to_string()));
        assert!(names.contains(&"NAME".to_string()));
        assert!(names.contains(&"RBRACE".to_string()));
        assert!(names.contains(&"NEWLINE".to_string()));
    }

    #[test]
    fn at_tag_value_includes_at_sign() {
        let tokens = tokenize_jsdoc("@param {string} name\n");
        let at = tokens.iter().find(|t| t.type_name.as_deref() == Some("AT_TAG"));
        assert!(at.is_some());
        assert_eq!(at.unwrap().value, "@param");
    }

    #[test]
    fn nominal_type_with_dots() {
        // `Foo.Bar` should produce NAME DOT NAME — three tokens.
        let tokens = tokenize_jsdoc("@type {Foo.Bar}\n");
        let name_dot_name: Vec<String> = pairs(&tokens)
            .into_iter()
            .filter(|(n, _)| n == "NAME" || n == "DOT")
            .map(|(n, _)| n)
            .collect();
        assert_eq!(name_dot_name, vec!["NAME", "DOT", "NAME"]);
    }

    #[test]
    fn nullable_wrapper() {
        // `?Foo` → QUESTION NAME.
        let tokens = tokenize_jsdoc("@type {?Foo}\n");
        let p = pairs(&tokens);
        assert!(p.iter().any(|(n, _)| n == "QUESTION"));
        assert!(p.iter().any(|(n, v)| n == "NAME" && v == "Foo"));
    }

    #[test]
    fn variadic_wrapper() {
        let tokens = tokenize_jsdoc("@param {...string} args\n");
        assert!(pairs(&tokens).iter().any(|(n, _)| n == "ELLIPSIS"));
    }

    #[test]
    fn array_suffix() {
        // `string[]` → NAME LBRACKET RBRACKET.
        let tokens = tokenize_jsdoc("@type {string[]}\n");
        let p = pairs(&tokens);
        assert!(p.iter().any(|(n, v)| n == "NAME" && v == "string"));
        assert!(p.iter().any(|(n, _)| n == "LBRACKET"));
        assert!(p.iter().any(|(n, _)| n == "RBRACKET"));
    }

    #[test]
    fn whitespace_skipped_between_tokens() {
        let compact_tokens = tokenize_jsdoc("@type{number}\n");
        let spaced_tokens = tokenize_jsdoc("@type   {  number  }\n");
        let compact = pairs(&compact_tokens);
        let spaced = pairs(&spaced_tokens);
        // Same logical sequence, different whitespace.
        let names_compact: Vec<&String> = compact.iter().map(|(n, _)| n).collect();
        let names_spaced: Vec<&String> = spaced.iter().map(|(n, _)| n).collect();
        assert_eq!(names_compact, names_spaced);
    }

    #[test]
    fn create_jsdoc_lexer_returns_working_lexer() {
        // Factory path produces a lexer that can drive tokenize() itself.
        let mut lexer = create_jsdoc_lexer("@type {number}\n");
        let tokens = lexer.tokenize().expect("tokenize should succeed");
        assert!(tokens.last().unwrap().type_ == TokenType::Eof);
    }

    #[test]
    fn empty_input_is_eof_only() {
        let tokens = tokenize_jsdoc("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].type_, TokenType::Eof);
    }
}
