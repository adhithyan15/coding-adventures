//! TOML lexer backed by compiled token grammar.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub fn create_toml_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

pub fn tokenize_toml(source: &str) -> Vec<Token> {
    let mut lexer = create_toml_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("TOML tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper: collect token (type_, value, type_name) tuples excluding EOF.
    // -----------------------------------------------------------------------

    /// Extract the (TokenType, value, type_name) tuples from a token stream,
    /// excluding the final EOF token. This makes test assertions more concise.
    ///
    /// Many TOML tokens (BARE_KEY, INTEGER, FLOAT, TRUE, FALSE, date/time types)
    /// map to TokenType::Name with a custom type_name. The type_name is what
    /// distinguishes them.
    fn token_info(tokens: &[Token]) -> Vec<(TokenType, &str, Option<&str>)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str(), t.type_name.as_deref()))
            .collect()
    }

    /// Filter out NEWLINE tokens for tests that don't care about line structure.
    fn non_newline_info(tokens: &[Token]) -> Vec<(TokenType, &str, Option<&str>)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof && t.type_ != TokenType::Newline)
            .map(|t| (t.type_, t.value.as_str(), t.type_name.as_deref()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: Bare key
    // -----------------------------------------------------------------------

    /// Bare keys are the most common key type in TOML: unquoted strings
    /// composed of ASCII letters, digits, dashes, and underscores.
    #[test]
    fn test_tokenize_bare_key() {
        let tokens = tokenize_toml("my-key_123");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("BARE_KEY"));
        assert_eq!(info[0].1, "my-key_123");
    }

    // -----------------------------------------------------------------------
    // Test 2: Basic string (double-quoted)
    // -----------------------------------------------------------------------

    /// Basic strings use double quotes and support escape sequences.
    /// With escapes: none mode, quotes are stripped but escapes are preserved.
    #[test]
    fn test_tokenize_basic_string() {
        let tokens = tokenize_toml("\"hello world\"");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("BASIC_STRING"));
        // Quotes should be stripped — the value is the inner content.
        assert_eq!(info[0].1, "hello world");
    }

    // -----------------------------------------------------------------------
    // Test 3: Literal string (single-quoted)
    // -----------------------------------------------------------------------

    /// Literal strings use single quotes and do NOT process escape sequences.
    /// What you see is what you get.
    #[test]
    fn test_tokenize_literal_string() {
        let tokens = tokenize_toml("'C:\\Users\\Alice'");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("LITERAL_STRING"));
        assert_eq!(info[0].1, "C:\\Users\\Alice");
    }

    // -----------------------------------------------------------------------
    // Test 4: Integer (decimal)
    // -----------------------------------------------------------------------

    /// Decimal integers: positive, negative, with underscore separators.
    #[test]
    fn test_tokenize_integer() {
        let tokens = tokenize_toml("42");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("INTEGER"));
        assert_eq!(info[0].1, "42");
    }

    // -----------------------------------------------------------------------
    // Test 5: Integer with underscores
    // -----------------------------------------------------------------------

    /// TOML allows underscores between digits for readability: 1_000_000.
    #[test]
    fn test_tokenize_integer_underscores() {
        let tokens = tokenize_toml("1_000_000");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("INTEGER"));
        assert_eq!(info[0].1, "1_000_000");
    }

    // -----------------------------------------------------------------------
    // Test 6: Hex, octal, binary integers
    // -----------------------------------------------------------------------

    /// TOML supports hex (0x), octal (0o), and binary (0b) integer literals.
    /// All are aliased to the INTEGER token type.
    #[test]
    fn test_tokenize_prefixed_integers() {
        let tokens_hex = tokenize_toml("0xDEADBEEF");
        let info_hex = non_newline_info(&tokens_hex);
        assert_eq!(info_hex.len(), 1);
        assert_eq!(info_hex[0].2, Some("INTEGER"));
        assert_eq!(info_hex[0].1, "0xDEADBEEF");

        let tokens_oct = tokenize_toml("0o755");
        let info_oct = non_newline_info(&tokens_oct);
        assert_eq!(info_oct.len(), 1);
        assert_eq!(info_oct[0].2, Some("INTEGER"));
        assert_eq!(info_oct[0].1, "0o755");

        let tokens_bin = tokenize_toml("0b11010110");
        let info_bin = non_newline_info(&tokens_bin);
        assert_eq!(info_bin.len(), 1);
        assert_eq!(info_bin[0].2, Some("INTEGER"));
        assert_eq!(info_bin[0].1, "0b11010110");
    }

    // -----------------------------------------------------------------------
    // Test 7: Float (decimal)
    // -----------------------------------------------------------------------

    /// Decimal floats: 3.14, +1.0, -0.5
    #[test]
    fn test_tokenize_float_decimal() {
        let tokens = tokenize_toml("3.14");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("FLOAT"));
        assert_eq!(info[0].1, "3.14");
    }

    // -----------------------------------------------------------------------
    // Test 8: Float (scientific notation)
    // -----------------------------------------------------------------------

    /// Scientific notation: 5e+22, 1e06, -2E-2
    #[test]
    fn test_tokenize_float_scientific() {
        let tokens = tokenize_toml("5e+22");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("FLOAT"));
        assert_eq!(info[0].1, "5e+22");
    }

    // -----------------------------------------------------------------------
    // Test 9: Float special values (inf, nan)
    // -----------------------------------------------------------------------

    /// TOML supports inf and nan as float values, optionally with a sign.
    #[test]
    fn test_tokenize_float_special() {
        let tokens_inf = tokenize_toml("inf");
        let info_inf = non_newline_info(&tokens_inf);
        assert_eq!(info_inf.len(), 1);
        assert_eq!(info_inf[0].2, Some("FLOAT"));
        assert_eq!(info_inf[0].1, "inf");

        let tokens_nan = tokenize_toml("nan");
        let info_nan = non_newline_info(&tokens_nan);
        assert_eq!(info_nan.len(), 1);
        assert_eq!(info_nan[0].2, Some("FLOAT"));
        assert_eq!(info_nan[0].1, "nan");

        let tokens_ninf = tokenize_toml("-inf");
        let info_ninf = non_newline_info(&tokens_ninf);
        assert_eq!(info_ninf.len(), 1);
        assert_eq!(info_ninf[0].2, Some("FLOAT"));
        assert_eq!(info_ninf[0].1, "-inf");
    }

    // -----------------------------------------------------------------------
    // Test 10: Boolean literals
    // -----------------------------------------------------------------------

    /// TOML booleans are `true` and `false`, emitted as their own token types.
    #[test]
    fn test_tokenize_booleans() {
        let tokens_true = tokenize_toml("true");
        let info_true = non_newline_info(&tokens_true);
        assert_eq!(info_true.len(), 1);
        assert_eq!(info_true[0].2, Some("TRUE"));
        assert_eq!(info_true[0].1, "true");

        let tokens_false = tokenize_toml("false");
        let info_false = non_newline_info(&tokens_false);
        assert_eq!(info_false.len(), 1);
        assert_eq!(info_false[0].2, Some("FALSE"));
        assert_eq!(info_false[0].1, "false");
    }

    // -----------------------------------------------------------------------
    // Test 11: Offset datetime
    // -----------------------------------------------------------------------

    /// Full datetime with timezone: 1979-05-27T07:32:00Z
    #[test]
    fn test_tokenize_offset_datetime() {
        let tokens = tokenize_toml("1979-05-27T07:32:00Z");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("OFFSET_DATETIME"));
        assert_eq!(info[0].1, "1979-05-27T07:32:00Z");
    }

    // -----------------------------------------------------------------------
    // Test 12: Local datetime
    // -----------------------------------------------------------------------

    /// Datetime without timezone: 1979-05-27T07:32:00
    #[test]
    fn test_tokenize_local_datetime() {
        let tokens = tokenize_toml("1979-05-27T07:32:00");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("LOCAL_DATETIME"));
        assert_eq!(info[0].1, "1979-05-27T07:32:00");
    }

    // -----------------------------------------------------------------------
    // Test 13: Local date
    // -----------------------------------------------------------------------

    /// Date only: 1979-05-27
    #[test]
    fn test_tokenize_local_date() {
        let tokens = tokenize_toml("1979-05-27");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("LOCAL_DATE"));
        assert_eq!(info[0].1, "1979-05-27");
    }

    // -----------------------------------------------------------------------
    // Test 14: Local time
    // -----------------------------------------------------------------------

    /// Time only: 07:32:00
    #[test]
    fn test_tokenize_local_time() {
        let tokens = tokenize_toml("07:32:00");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("LOCAL_TIME"));
        assert_eq!(info[0].1, "07:32:00");
    }

    // -----------------------------------------------------------------------
    // Test 15: Structural tokens
    // -----------------------------------------------------------------------

    /// TOML structural characters: = . , [ ] { }
    #[test]
    fn test_tokenize_structural_tokens() {
        let tokens = tokenize_toml("= . , [ ] { }");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 7);
        assert_eq!(info[0].0, TokenType::Equals);
        assert_eq!(info[1].0, TokenType::Dot);
        assert_eq!(info[2].0, TokenType::Comma);
        assert_eq!(info[3].0, TokenType::LBracket);
        assert_eq!(info[4].0, TokenType::RBracket);
        assert_eq!(info[5].0, TokenType::LBrace);
        assert_eq!(info[6].0, TokenType::RBrace);
    }

    // -----------------------------------------------------------------------
    // Test 16: Simple key-value pair
    // -----------------------------------------------------------------------

    /// The fundamental TOML construct: key = value
    #[test]
    fn test_tokenize_key_value_pair() {
        let tokens = tokenize_toml("title = \"TOML Example\"");
        let info = non_newline_info(&tokens);

        // Expected: BARE_KEY("title"), EQUALS, BASIC_STRING("TOML Example")
        assert_eq!(info.len(), 3);
        assert_eq!(info[0].2, Some("BARE_KEY"));
        assert_eq!(info[0].1, "title");
        assert_eq!(info[1].0, TokenType::Equals);
        assert_eq!(info[2].2, Some("BASIC_STRING"));
        assert_eq!(info[2].1, "TOML Example");
    }

    // -----------------------------------------------------------------------
    // Test 17: Table header
    // -----------------------------------------------------------------------

    /// Table headers switch the current table context: [server]
    #[test]
    fn test_tokenize_table_header() {
        let tokens = tokenize_toml("[server]");
        let info = non_newline_info(&tokens);

        // Expected: LBRACKET, BARE_KEY("server"), RBRACKET
        assert_eq!(info.len(), 3);
        assert_eq!(info[0].0, TokenType::LBracket);
        assert_eq!(info[1].2, Some("BARE_KEY"));
        assert_eq!(info[1].1, "server");
        assert_eq!(info[2].0, TokenType::RBracket);
    }

    // -----------------------------------------------------------------------
    // Test 18: Array of tables header
    // -----------------------------------------------------------------------

    /// Array-of-tables headers use double brackets: [[products]]
    #[test]
    fn test_tokenize_array_table_header() {
        let tokens = tokenize_toml("[[products]]");
        let info = non_newline_info(&tokens);

        // Expected: LBRACKET, LBRACKET, BARE_KEY("products"), RBRACKET, RBRACKET
        assert_eq!(info.len(), 5);
        assert_eq!(info[0].0, TokenType::LBracket);
        assert_eq!(info[1].0, TokenType::LBracket);
        assert_eq!(info[2].2, Some("BARE_KEY"));
        assert_eq!(info[2].1, "products");
        assert_eq!(info[3].0, TokenType::RBracket);
        assert_eq!(info[4].0, TokenType::RBracket);
    }

    // -----------------------------------------------------------------------
    // Test 19: Dotted key
    // -----------------------------------------------------------------------

    /// Dotted keys create intermediate tables: a.b.c = 1
    #[test]
    fn test_tokenize_dotted_key() {
        let tokens = tokenize_toml("a.b.c = 1");
        let info = non_newline_info(&tokens);

        // Expected: BARE_KEY("a"), DOT, BARE_KEY("b"), DOT, BARE_KEY("c"),
        //           EQUALS, INTEGER("1")
        assert_eq!(info.len(), 7);
        assert_eq!(info[0].2, Some("BARE_KEY"));
        assert_eq!(info[0].1, "a");
        assert_eq!(info[1].0, TokenType::Dot);
        assert_eq!(info[2].2, Some("BARE_KEY"));
        assert_eq!(info[2].1, "b");
        assert_eq!(info[3].0, TokenType::Dot);
        assert_eq!(info[4].2, Some("BARE_KEY"));
        assert_eq!(info[4].1, "c");
        assert_eq!(info[5].0, TokenType::Equals);
        assert_eq!(info[6].2, Some("INTEGER"));
    }

    // -----------------------------------------------------------------------
    // Test 20: Inline table
    // -----------------------------------------------------------------------

    /// Inline tables are compact single-line table definitions.
    #[test]
    fn test_tokenize_inline_table() {
        let tokens = tokenize_toml("point = { x = 1, y = 2 }");
        let info = non_newline_info(&tokens);

        // Expected: BARE_KEY("point"), EQUALS, LBRACE,
        //           BARE_KEY("x"), EQUALS, INTEGER("1"), COMMA,
        //           BARE_KEY("y"), EQUALS, INTEGER("2"),
        //           RBRACE
        assert_eq!(info.len(), 11);
        assert_eq!(info[2].0, TokenType::LBrace);
        assert_eq!(info[10].0, TokenType::RBrace);
    }

    // -----------------------------------------------------------------------
    // Test 21: Array value
    // -----------------------------------------------------------------------

    /// Arrays are comma-separated values in brackets.
    #[test]
    fn test_tokenize_array() {
        let tokens = tokenize_toml("colors = [\"red\", \"green\", \"blue\"]");
        let info = non_newline_info(&tokens);

        // Expected: BARE_KEY, EQUALS, LBRACKET,
        //           BASIC_STRING, COMMA, BASIC_STRING, COMMA, BASIC_STRING,
        //           RBRACKET
        assert_eq!(info.len(), 9);
        assert_eq!(info[2].0, TokenType::LBracket);
        assert_eq!(info[3].2, Some("BASIC_STRING"));
        assert_eq!(info[3].1, "red");
        assert_eq!(info[8].0, TokenType::RBracket);
    }

    // -----------------------------------------------------------------------
    // Test 22: NEWLINE tokens
    // -----------------------------------------------------------------------

    /// TOML is newline-sensitive. The lexer emits NEWLINE tokens between lines.
    #[test]
    fn test_tokenize_newlines() {
        let tokens = tokenize_toml("a = 1\nb = 2");
        let info = token_info(&tokens);

        // Find NEWLINE tokens in the stream.
        let newline_count = info.iter().filter(|(t, _, _)| *t == TokenType::Newline).count();
        assert!(newline_count >= 1, "Expected at least one NEWLINE token");
    }

    // -----------------------------------------------------------------------
    // Test 23: Comments are skipped
    // -----------------------------------------------------------------------

    /// Comments (# to end of line) are consumed silently.
    #[test]
    fn test_comments_skipped() {
        let tokens = tokenize_toml("key = \"value\" # this is a comment");
        let info = non_newline_info(&tokens);

        // The comment should not produce any tokens.
        assert_eq!(info.len(), 3);
        assert_eq!(info[0].2, Some("BARE_KEY"));
        assert_eq!(info[1].0, TokenType::Equals);
        assert_eq!(info[2].2, Some("BASIC_STRING"));
    }

    // -----------------------------------------------------------------------
    // Test 24: Multi-line basic string
    // -----------------------------------------------------------------------

    /// Multi-line basic strings use triple double-quotes: """..."""
    #[test]
    fn test_tokenize_ml_basic_string() {
        let tokens = tokenize_toml("\"\"\"hello\nworld\"\"\"");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("ML_BASIC_STRING"));
    }

    // -----------------------------------------------------------------------
    // Test 25: Multi-line literal string
    // -----------------------------------------------------------------------

    /// Multi-line literal strings use triple single-quotes: '''...'''
    #[test]
    fn test_tokenize_ml_literal_string() {
        let tokens = tokenize_toml("'''hello\nworld'''");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("ML_LITERAL_STRING"));
    }

    // -----------------------------------------------------------------------
    // Test 26: Datetime with fractional seconds
    // -----------------------------------------------------------------------

    /// Datetimes can include fractional seconds: 1979-05-27T07:32:00.999999
    #[test]
    fn test_tokenize_datetime_fractional() {
        let tokens = tokenize_toml("1979-05-27T07:32:00.999Z");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("OFFSET_DATETIME"));
        assert_eq!(info[0].1, "1979-05-27T07:32:00.999Z");
    }

    // -----------------------------------------------------------------------
    // Test 27: Offset datetime with timezone offset
    // -----------------------------------------------------------------------

    /// Offset datetimes can use +HH:MM or -HH:MM instead of Z.
    #[test]
    fn test_tokenize_datetime_offset() {
        let tokens = tokenize_toml("1979-05-27T07:32:00+05:30");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("OFFSET_DATETIME"));
        assert_eq!(info[0].1, "1979-05-27T07:32:00+05:30");
    }

    // -----------------------------------------------------------------------
    // Test 28: Factory function
    // -----------------------------------------------------------------------

    /// The `create_toml_lexer` factory function should return a working lexer.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_toml_lexer("key = 42");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        // Should end with an EOF token.
        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 29: Complete TOML document
    // -----------------------------------------------------------------------

    /// A realistic multi-section TOML document exercises all token types together.
    #[test]
    fn test_tokenize_full_document() {
        let source = "# This is a TOML document
title = \"TOML Example\"

[owner]
name = \"Tom Preston-Werner\"
dob = 1979-05-27T07:32:00Z

[database]
enabled = true
ports = [8001, 8001, 8002]
";
        let tokens = tokenize_toml(source);
        let info = non_newline_info(&tokens);

        // Verify we got a reasonable number of tokens (not zero, not an error).
        assert!(info.len() > 15, "Expected many tokens, got {}", info.len());

        // Verify specific tokens are present.
        let has_bare_key = info.iter().any(|(_, _, tn)| *tn == Some("BARE_KEY"));
        let has_basic_string = info.iter().any(|(_, _, tn)| *tn == Some("BASIC_STRING"));
        let has_offset_datetime = info.iter().any(|(_, _, tn)| *tn == Some("OFFSET_DATETIME"));
        let has_true = info.iter().any(|(_, _, tn)| *tn == Some("TRUE"));
        let has_integer = info.iter().any(|(_, _, tn)| *tn == Some("INTEGER"));

        assert!(has_bare_key, "Expected BARE_KEY tokens");
        assert!(has_basic_string, "Expected BASIC_STRING tokens");
        assert!(has_offset_datetime, "Expected OFFSET_DATETIME token");
        assert!(has_true, "Expected TRUE token");
        assert!(has_integer, "Expected INTEGER tokens");
    }

    // -----------------------------------------------------------------------
    // Test 30: Negative integer
    // -----------------------------------------------------------------------

    /// Negative integers like -17 should be a single INTEGER token.
    #[test]
    fn test_tokenize_negative_integer() {
        let tokens = tokenize_toml("-17");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 1);
        assert_eq!(info[0].2, Some("INTEGER"));
        assert_eq!(info[0].1, "-17");
    }

    // -----------------------------------------------------------------------
    // Test 31: Quoted keys
    // -----------------------------------------------------------------------

    /// Keys can be quoted: "key with spaces" = value
    #[test]
    fn test_tokenize_quoted_key() {
        let tokens = tokenize_toml("\"key with spaces\" = 42");
        let info = non_newline_info(&tokens);

        assert_eq!(info.len(), 3);
        assert_eq!(info[0].2, Some("BASIC_STRING"));
        assert_eq!(info[0].1, "key with spaces");
        assert_eq!(info[1].0, TokenType::Equals);
        assert_eq!(info[2].2, Some("INTEGER"));
    }
}

