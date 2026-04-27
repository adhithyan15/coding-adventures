//! Ruby lexer backed by compiled token grammar.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub fn create_ruby_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

pub fn tokenize_ruby(source: &str) -> Vec<Token> {
    let mut lexer = create_ruby_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Ruby tokenization failed: {e}"))
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
    // Test 1: Simple assignment
    // -----------------------------------------------------------------------

    /// Verify that a basic assignment is tokenized correctly.
    #[test]
    fn test_tokenize_assignment() {
        let tokens = tokenize_ruby("x = 42");
        let pairs = token_pairs(&tokens);

        // Expected: NAME("x"), EQUALS("="), NUMBER("42")
        assert!(pairs.len() >= 3, "Expected at least 3 tokens, got {}", pairs.len());
        assert_eq!(pairs[0].0, TokenType::Name);
        assert_eq!(pairs[0].1, "x");
    }

    // -----------------------------------------------------------------------
    // Test 2: Keywords are recognized
    // -----------------------------------------------------------------------

    /// Ruby keywords should be classified as KEYWORD tokens, not NAME.
    #[test]
    fn test_keywords() {
        let keywords = ["def", "end", "if", "else", "elsif", "unless",
                        "while", "do", "class", "module", "return",
                        "true", "false", "nil"];

        for kw in &keywords {
            let tokens = tokenize_ruby(kw);
            let pairs = token_pairs(&tokens);

            assert_eq!(
                pairs[0].0, TokenType::Keyword,
                "Expected '{}' to be a KEYWORD, got {:?}",
                kw, pairs[0].0
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: Arithmetic operators
    // -----------------------------------------------------------------------

    /// Arithmetic operators should be tokenized correctly.
    #[test]
    fn test_operators() {
        let tokens = tokenize_ruby("a + b - c * d / e");
        let pairs = token_pairs(&tokens);

        let ops: Vec<&str> = pairs.iter()
            .filter(|(_, v)| ["+", "-", "*", "/"].contains(v))
            .map(|(_, v)| *v)
            .collect();

        assert_eq!(ops, vec!["+", "-", "*", "/"]);
    }

    // -----------------------------------------------------------------------
    // Test 4: Comparison operators
    // -----------------------------------------------------------------------

    /// Multi-character comparison operators should be tokenized as single tokens.
    #[test]
    fn test_comparison_operators() {
        let tokens = tokenize_ruby("a == b != c");
        let pairs = token_pairs(&tokens);

        let has_eq = pairs.iter().any(|(_, v)| *v == "==");
        let has_ne = pairs.iter().any(|(_, v)| *v == "!=");

        assert!(has_eq, "Expected '==' token");
        assert!(has_ne, "Expected '!=' token");
    }

    // -----------------------------------------------------------------------
    // Test 5: String literals
    // -----------------------------------------------------------------------

    /// Ruby supports both single-quoted and double-quoted strings.
    #[test]
    fn test_strings() {
        let tokens = tokenize_ruby("x = \"hello world\"");
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token");
    }

    // -----------------------------------------------------------------------
    // Test 6: Number literals
    // -----------------------------------------------------------------------

    /// Ruby supports integer and floating-point numbers.
    #[test]
    fn test_numbers() {
        let tokens = tokenize_ruby("42");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "42");
    }

    // -----------------------------------------------------------------------
    // Test 7: Delimiters
    // -----------------------------------------------------------------------

    /// Delimiter tokens defined in ruby.tokens should be recognized.
    /// Note: ruby.tokens only defines LPAREN, RPAREN, COMMA, and COLON
    /// as delimiters. Brackets, braces, semicolons, and dots are not
    /// in the grammar. Comments are also not handled (no skip: section
    /// in ruby.tokens), so comment-related tests are omitted.
    #[test]
    fn test_delimiters() {
        let tokens = tokenize_ruby("(a, b)");
        let pairs = token_pairs(&tokens);

        let values: Vec<&str> = pairs.iter().map(|(_, v)| *v).collect();
        assert!(values.contains(&"("), "Expected '(' token");
        assert!(values.contains(&")"), "Expected ')' token");
        assert!(values.contains(&","), "Expected ',' token");
    }

    // -----------------------------------------------------------------------
    // Test 9: Whitespace is skipped
    // -----------------------------------------------------------------------

    /// Whitespace between tokens should be consumed without producing tokens.
    #[test]
    fn test_whitespace_skipped() {
        let compact = tokenize_ruby("x=1");
        let spaced = tokenize_ruby("x  =  1");

        let pairs_compact = token_pairs(&compact);
        let pairs_spaced = token_pairs(&spaced);

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
    }

    // -----------------------------------------------------------------------
    // Test 10: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_ruby_lexer` factory function should return a `GrammarLexer`
    /// that can successfully tokenize source code.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_ruby_lexer("42");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 11: Method definition tokens
    // -----------------------------------------------------------------------

    /// A method definition exercises keywords, identifiers, and parentheses.
    #[test]
    fn test_tokenize_method_def() {
        let tokens = tokenize_ruby("def add(a, b)\n  a + b\nend");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "def");

        // Should also find the "end" keyword.
        let has_end = pairs.iter().any(|(t, v)| *t == TokenType::Keyword && *v == "end");
        assert!(has_end, "Expected 'end' keyword");
    }

    // -----------------------------------------------------------------------
    // Test 12: Symbol-like tokens
    // -----------------------------------------------------------------------

    /// Ruby has symbols written as :name. The colon should be tokenized.
    #[test]
    fn test_tokenize_colon() {
        let tokens = tokenize_ruby("x = :hello");
        let pairs = token_pairs(&tokens);

        // The colon should appear as a token.
        let has_colon = pairs.iter().any(|(_, v)| *v == ":");
        assert!(has_colon, "Expected a colon token");
    }
}

