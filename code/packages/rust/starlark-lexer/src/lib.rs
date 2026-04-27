//! Starlark lexer backed by compiled token grammar.

use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

mod _grammar;

pub fn create_starlark_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar = _grammar::token_grammar();
    GrammarLexer::new(source, &grammar)
}

pub fn tokenize_starlark(source: &str) -> Vec<Token> {
    let mut lexer = create_starlark_lexer(source);
    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Starlark tokenization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper: collect token types (excluding EOF) for easier assertions.
    // -----------------------------------------------------------------------

    /// Extract the (type, value) pairs from a token stream, excluding
    /// the final EOF token. This makes test assertions more concise.
    fn token_pairs(tokens: &[Token]) -> Vec<(TokenType, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: Simple arithmetic expression
    // -----------------------------------------------------------------------

    /// Verify that a basic assignment with arithmetic operators is tokenized
    /// correctly. This tests NAME, EQUALS, INT, and PLUS tokens.
    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize_starlark("x = 1 + 2\n");
        let pairs = token_pairs(&tokens);

        // Expected tokens (in indentation mode, the lexer emits NEWLINE
        // at the end of each logical line):
        //   NAME("x"), EQUALS("="), INT("1"), PLUS("+"), INT("2"), NEWLINE
        assert_eq!(pairs.len(), 6);
        assert_eq!(pairs[0].0, TokenType::Name);
        assert_eq!(pairs[0].1, "x");
        assert_eq!(pairs[1].0, TokenType::Equals);
        assert_eq!(pairs[3].0, TokenType::Plus);
    }

    // -----------------------------------------------------------------------
    // Test 2: Keywords are recognized
    // -----------------------------------------------------------------------

    /// Starlark keywords (def, return, if, else, for, in, etc.) should be
    /// classified as KEYWORD tokens, not NAME tokens. The lexer achieves
    /// this by first matching the NAME pattern, then checking the keyword
    /// list and promoting matching names to KEYWORD.
    #[test]
    fn test_keywords() {
        let keywords = ["def", "return", "if", "else", "for", "in", "and",
                        "or", "not", "pass", "break", "continue", "lambda",
                        "load", "elif", "True", "False", "None"];

        for kw in &keywords {
            // Wrap each keyword in a newline so indentation mode is happy.
            let source = format!("{kw}\n");
            let tokens = tokenize_starlark(&source);

            assert_eq!(
                tokens[0].type_, TokenType::Keyword,
                "Expected '{}' to be a KEYWORD, got {:?}",
                kw, tokens[0].type_
            );
            assert_eq!(tokens[0].value, *kw);
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: Reserved keywords cause errors
    // -----------------------------------------------------------------------

    /// Reserved keywords like `class`, `import`, `while` are not allowed in
    /// Starlark. The lexer should panic (via our unwrap) when it encounters
    /// them. This test uses `#[should_panic]` to verify the error.
    #[test]
    #[should_panic(expected = "Reserved keyword")]
    fn test_reserved_keyword_error() {
        // "class" is reserved in Starlark — using it should trigger a
        // LexerError with "Reserved keyword" in the message, which our
        // tokenize_starlark function turns into a panic.
        tokenize_starlark("class\n");
    }

    // -----------------------------------------------------------------------
    // Test 4: Indentation produces INDENT/DEDENT tokens
    // -----------------------------------------------------------------------

    /// In indentation mode, the lexer tracks leading whitespace and emits
    /// INDENT when indentation increases and DEDENT when it decreases.
    /// This is how Python and Starlark represent block structure without
    /// braces.
    #[test]
    fn test_indentation() {
        let source = "def f():\n    return 1\n";
        let tokens = tokenize_starlark(source);

        // The token stream should include INDENT and DEDENT:
        //   KEYWORD("def"), NAME("f"), LPAREN, RPAREN, COLON, NEWLINE,
        //   INDENT, KEYWORD("return"), INT("1"), NEWLINE, DEDENT, EOF
        let has_indent = tokens.iter().any(|t| t.type_ == TokenType::Indent);
        let has_dedent = tokens.iter().any(|t| t.type_ == TokenType::Dedent);
        let has_newline = tokens.iter().any(|t| t.type_ == TokenType::Newline);

        assert!(has_indent, "Expected INDENT token in output");
        assert!(has_dedent, "Expected DEDENT token in output");
        assert!(has_newline, "Expected NEWLINE token in output");

        // Count: exactly one INDENT and one DEDENT for a single-level block.
        let indent_count = tokens.iter().filter(|t| t.type_ == TokenType::Indent).count();
        let dedent_count = tokens.iter().filter(|t| t.type_ == TokenType::Dedent).count();
        assert_eq!(indent_count, 1, "Expected exactly 1 INDENT");
        assert_eq!(dedent_count, 1, "Expected exactly 1 DEDENT");
    }

    // -----------------------------------------------------------------------
    // Test 5: Brackets suppress NEWLINE/INDENT/DEDENT
    // -----------------------------------------------------------------------

    /// When inside brackets (parentheses, square brackets, or curly braces),
    /// the lexer suppresses NEWLINE, INDENT, and DEDENT tokens. This allows
    /// multi-line function calls and list/dict literals:
    ///
    /// ```starlark
    /// cc_library(
    ///     name = "foo",
    ///     srcs = ["bar.cc"],
    /// )
    /// ```
    ///
    /// Without bracket suppression, the newline after `(` would trigger
    /// indentation tracking, producing spurious INDENT/DEDENT tokens.
    #[test]
    fn test_bracket_suppression() {
        let source = "f(\n    x,\n    y\n)\n";
        let tokens = tokenize_starlark(source);

        // Inside the parentheses, no INDENT/DEDENT should be emitted.
        let indent_count = tokens.iter().filter(|t| t.type_ == TokenType::Indent).count();
        assert_eq!(indent_count, 0, "Expected no INDENT inside brackets");

        // NEWLINE inside brackets should also be suppressed.
        // The only NEWLINE should be after the closing ')'.
        let newline_count = tokens.iter().filter(|t| t.type_ == TokenType::Newline).count();
        assert_eq!(newline_count, 1, "Expected exactly 1 NEWLINE (after closing paren)");
    }

    // -----------------------------------------------------------------------
    // Test 6: Multi-character operators
    // -----------------------------------------------------------------------

    /// Starlark has several two-character operators that must be tokenized
    /// as single tokens, not split into two single-character tokens.
    /// For example, `**` must be DOUBLE_STAR, not STAR + STAR.
    #[test]
    fn test_operators() {
        let source = "a ** b // c == d != e <= f >= g\n";
        let tokens = tokenize_starlark(source);
        let pairs = token_pairs(&tokens);

        // Extract just the operators by filtering out identifiers (NAME tokens
        // without a type_name, i.e. actual variable names) and NEWLINE.
        //
        // Note: Multi-character operators like ** and // have custom type names
        // (DOUBLE_STAR, FLOOR_DIV) but map to TokenType::Name as a fallback.
        // We distinguish them from real identifiers by checking that the value
        // is NOT a simple alphabetic name.
        let ops: Vec<&str> = pairs.iter()
            .filter(|(_, v)| {
                // Keep everything that is not a simple alphabetic identifier
                // and not a NEWLINE escape.
                !v.chars().all(|c| c.is_alphabetic() || c == '_') && *v != "\\n"
            })
            .map(|(_, v)| *v)
            .collect();

        assert_eq!(ops, vec!["**", "//", "==", "!=", "<=", ">="]);
    }

    // -----------------------------------------------------------------------
    // Test 7: String literals
    // -----------------------------------------------------------------------

    /// Starlark supports double-quoted string literals. The lexer should
    /// strip the quotes and process escape sequences.
    #[test]
    fn test_strings() {
        let source = "x = \"hello world\"\n";
        let tokens = tokenize_starlark(source);

        // Find the STRING token.
        let string_token = tokens.iter().find(|t| t.type_ == TokenType::String
            || t.type_name.as_deref() == Some("STRING"));

        assert!(string_token.is_some(), "Expected a STRING token");
        let st = string_token.unwrap();
        assert_eq!(st.value, "hello world");
    }

    // -----------------------------------------------------------------------
    // Test 8: Comments are skipped
    // -----------------------------------------------------------------------

    /// Comments in Starlark start with `#` and run to the end of the line.
    /// They should be consumed by the lexer without producing tokens.
    #[test]
    fn test_comments_skipped() {
        let source = "x = 1  # this is a comment\n";
        let tokens = tokenize_starlark(source);

        // The comment should not appear in the token stream. We should see:
        //   NAME("x"), EQUALS("="), INT("1"), NEWLINE, EOF
        let has_comment = tokens.iter().any(|t| t.value.contains("comment") || t.value.contains("#"));
        assert!(!has_comment, "Comments should not produce tokens");

        // Verify the token count: x, =, 1, NEWLINE, EOF = 5 tokens.
        assert_eq!(tokens.len(), 5);
    }

    // -----------------------------------------------------------------------
    // Test 9: Float literals
    // -----------------------------------------------------------------------

    /// Starlark supports floating-point literals like `3.14`, `1e10`, `.5`,
    /// and `1.5e-3`. These should be tokenized as FLOAT tokens.
    #[test]
    fn test_float_literals() {
        // Use a simple source with just the float values to avoid
        // counting ambiguity from other number tokens.
        let source = "3.14\n";
        let tokens = tokenize_starlark(source);

        // Find tokens with FLOAT type name (since FLOAT is a custom type,
        // it will have type_name set to "FLOAT").
        let floats: Vec<&Token> = tokens.iter()
            .filter(|t| t.type_name.as_deref() == Some("FLOAT"))
            .collect();

        assert_eq!(floats.len(), 1, "Expected 1 FLOAT token");
        assert_eq!(floats[0].value, "3.14");

        // Also test scientific notation.
        let source2 = "1e10\n";
        let tokens2 = tokenize_starlark(source2);

        let floats2: Vec<&Token> = tokens2.iter()
            .filter(|t| t.type_name.as_deref() == Some("FLOAT"))
            .collect();

        assert_eq!(floats2.len(), 1, "Expected 1 FLOAT token for scientific notation");
        assert_eq!(floats2[0].value, "1e10");
    }

    // -----------------------------------------------------------------------
    // Test 10: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_starlark_lexer` factory function should return a
    /// `GrammarLexer` that can successfully tokenize source code. This
    /// tests the factory function independently of `tokenize_starlark`.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_starlark_lexer("42\n");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        // Should produce at minimum: INT("42"), NEWLINE, EOF
        assert!(tokens.len() >= 3);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 11: Single-quoted strings
    // -----------------------------------------------------------------------

    /// Starlark supports both single-quoted and double-quoted strings.
    /// Both should produce STRING tokens.
    #[test]
    fn test_single_quoted_strings() {
        let source = "x = 'hello'\n";
        let tokens = tokenize_starlark(source);

        let string_token = tokens.iter().find(|t| t.type_ == TokenType::String
            || t.type_name.as_deref() == Some("STRING"));

        assert!(string_token.is_some(), "Expected a STRING token for single-quoted string");
        assert_eq!(string_token.unwrap().value, "hello");
    }

    // -----------------------------------------------------------------------
    // Test 12: Augmented assignment operators
    // -----------------------------------------------------------------------

    /// Starlark supports augmented assignment operators like `+=`, `-=`,
    /// `*=`, etc. These are two-character tokens that must not be split.
    #[test]
    fn test_augmented_assignment_operators() {
        let source = "x += 1\n";
        let tokens = tokenize_starlark(source);

        // Look for the += token.
        let plus_eq = tokens.iter().find(|t| t.value == "+=");
        assert!(plus_eq.is_some(), "Expected '+=' token");
    }

    // -----------------------------------------------------------------------
    // Test 13: Delimiters
    // -----------------------------------------------------------------------

    /// All delimiter tokens should be recognized: ( ) [ ] { } , : ; .
    #[test]
    fn test_delimiters() {
        let source = "()[]{},:.;\n";
        let tokens = tokenize_starlark(source);

        let values: Vec<&str> = tokens.iter()
            .filter(|t| t.type_ != TokenType::Eof && t.type_ != TokenType::Newline)
            .map(|t| t.value.as_str())
            .collect();

        assert!(values.contains(&"("));
        assert!(values.contains(&")"));
        assert!(values.contains(&"["));
        assert!(values.contains(&"]"));
        assert!(values.contains(&"{"));
        assert!(values.contains(&"}"));
        assert!(values.contains(&","));
        assert!(values.contains(&":"));
        assert!(values.contains(&"."));
        assert!(values.contains(&";"));
    }

    // -----------------------------------------------------------------------
    // Test 14: Integer literals including hex and octal
    // -----------------------------------------------------------------------

    /// Starlark supports decimal, hexadecimal (0x), and octal (0o) integers.
    #[test]
    fn test_integer_literals() {
        let source = "a = 42\nb = 0xFF\nc = 0o77\n";
        let tokens = tokenize_starlark(source);

        // Find INT tokens (INT is a custom type name since it is not in
        // the standard TokenType enum).
        let ints: Vec<&Token> = tokens.iter()
            .filter(|t| t.type_name.as_deref() == Some("INT") || t.value == "42")
            .collect();

        assert!(ints.len() >= 3, "Expected at least 3 integer tokens, got {}", ints.len());
    }

    // -----------------------------------------------------------------------
    // Test 15: Nested indentation
    // -----------------------------------------------------------------------

    /// Multiple levels of indentation should produce multiple INDENT tokens
    /// on the way in and multiple DEDENT tokens on the way out.
    #[test]
    fn test_nested_indentation() {
        let source = "if True:\n    if True:\n        x = 1\n";
        let tokens = tokenize_starlark(source);

        let indent_count = tokens.iter().filter(|t| t.type_ == TokenType::Indent).count();
        let dedent_count = tokens.iter().filter(|t| t.type_ == TokenType::Dedent).count();

        assert_eq!(indent_count, 2, "Expected 2 INDENT tokens for nested blocks");
        assert_eq!(dedent_count, 2, "Expected 2 DEDENT tokens for nested blocks");
    }
}

