//! CSS lexer backed by compiled token grammar.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub fn create_css_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

pub fn tokenize_css(source: &str) -> Vec<Token> {
    let mut lexer = create_css_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("CSS tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper: collect token (type_, value) pairs excluding EOF.
    // -----------------------------------------------------------------------

    fn token_pairs(tokens: &[Token]) -> Vec<(TokenType, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: Simple selector and property
    // -----------------------------------------------------------------------

    /// A basic CSS rule with a type selector and a single property.
    #[test]
    fn test_tokenize_simple_rule() {
        let tokens = tokenize_css("body { color: red; }");
        let pairs = token_pairs(&tokens);

        // Should produce tokens for: body, {, color, :, red, ;, }
        assert!(pairs.len() >= 7, "Expected at least 7 tokens, got {}", pairs.len());
    }

    // -----------------------------------------------------------------------
    // Test 2: Numeric values with units (DIMENSION tokens)
    // -----------------------------------------------------------------------

    /// CSS dimension tokens like `16px`, `1.5em`, `100%` should be tokenized
    /// as single tokens, not split into number + identifier.
    #[test]
    fn test_tokenize_dimensions() {
        let tokens = tokenize_css("h1 { font-size: 16px; }");
        let pairs = token_pairs(&tokens);

        // Look for a token containing "16px" — it should be a single token.
        let has_dimension = pairs.iter().any(|(_, v)| *v == "16px");
        assert!(has_dimension, "Expected a single '16px' dimension token");
    }

    // -----------------------------------------------------------------------
    // Test 3: Hash tokens (colors and IDs)
    // -----------------------------------------------------------------------

    /// The `#` character introduces HASH tokens in CSS, used for both
    /// hex colors (#ff0000) and ID selectors (#main).
    #[test]
    fn test_tokenize_hash() {
        let tokens = tokenize_css("#main { color: #ff0000; }");
        let pairs = token_pairs(&tokens);

        // Should find hash tokens for #main and #ff0000.
        let hash_count = pairs.iter().filter(|(_, v)| v.starts_with('#')).count();
        assert!(hash_count >= 2, "Expected at least 2 hash tokens, got {}", hash_count);
    }

    // -----------------------------------------------------------------------
    // Test 4: String literals
    // -----------------------------------------------------------------------

    /// CSS supports both single-quoted and double-quoted strings, commonly
    /// used in `content` properties and `url()` functions.
    #[test]
    fn test_tokenize_strings() {
        let tokens = tokenize_css("a::after { content: \"hello\"; }");
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token");
    }

    // -----------------------------------------------------------------------
    // Test 5: At-keywords
    // -----------------------------------------------------------------------

    /// CSS at-rules like @media, @import, @keyframes begin with an
    /// AT_KEYWORD token.
    #[test]
    fn test_tokenize_at_keyword() {
        let tokens = tokenize_css("@media screen { }");
        let pairs = token_pairs(&tokens);

        let has_at = pairs.iter().any(|(_, v)| v.starts_with('@'));
        assert!(has_at, "Expected an at-keyword token");
    }

    // -----------------------------------------------------------------------
    // Test 6: Whitespace is skipped
    // -----------------------------------------------------------------------

    /// Whitespace between tokens should be consumed without producing tokens.
    #[test]
    fn test_whitespace_skipped() {
        let compact = tokenize_css("a{b:c}");
        let spaced = tokenize_css("a  {  b  :  c  }");

        let pairs_compact = token_pairs(&compact);
        let pairs_spaced = token_pairs(&spaced);

        assert_eq!(
            pairs_compact.len(), pairs_spaced.len(),
            "Whitespace should not affect token count"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: Comments are skipped
    // -----------------------------------------------------------------------

    /// CSS comments (/* ... */) should be consumed without producing tokens.
    #[test]
    fn test_comments_skipped() {
        let tokens = tokenize_css("a /* comment */ { color: red; }");
        let pairs = token_pairs(&tokens);

        let has_comment = pairs.iter().any(|(_, v)| v.contains("comment"));
        assert!(!has_comment, "Comments should not produce tokens");
    }

    // -----------------------------------------------------------------------
    // Test 8: Braces and delimiters
    // -----------------------------------------------------------------------

    /// CSS structural delimiters: { } ( ) [ ] : ; ,
    #[test]
    fn test_tokenize_delimiters() {
        let tokens = tokenize_css("a { b: c; }");
        let pairs = token_pairs(&tokens);

        let values: Vec<&str> = pairs.iter().map(|(_, v)| *v).collect();
        assert!(values.contains(&"{"));
        assert!(values.contains(&"}"));
        assert!(values.contains(&":"));
        assert!(values.contains(&";"));
    }

    // -----------------------------------------------------------------------
    // Test 9: Multiple selectors
    // -----------------------------------------------------------------------

    /// CSS selectors can be separated by commas, and compound selectors
    /// can include class selectors (.) and combinators.
    #[test]
    fn test_tokenize_multiple_selectors() {
        let tokens = tokenize_css("h1, h2, h3 { margin: 0; }");
        let pairs = token_pairs(&tokens);

        // Should have comma tokens separating selectors.
        let comma_count = pairs.iter().filter(|(_, v)| *v == ",").count();
        assert_eq!(comma_count, 2, "Expected 2 commas separating 3 selectors");
    }

    // -----------------------------------------------------------------------
    // Test 10: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_css_lexer` factory function should return a `GrammarLexer`
    /// that can successfully tokenize source code.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_css_lexer("a { }");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }
}

