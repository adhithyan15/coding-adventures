//! # ECMAScript 5 (2009) Lexer — tokenizing ES5 JavaScript source code.
//!
//! [ECMAScript 5](https://www.ecma-international.org/publications-and-standards/standards/ecma-262/)
//! landed a full decade after ES3 (ES4 was abandoned after years of debate).
//! The syntactic changes in ES5 are modest — the real innovations were strict
//! mode semantics, native JSON support, and property descriptors.
//!
//! This crate provides a lexer (tokenizer) specifically for ES5 JavaScript.
//! It loads the `es5.tokens` grammar file and feeds it to the generic
//! [`GrammarLexer`] from the `lexer` crate.
//!
//! # What ES5 adds over ES3
//!
//! - `debugger` keyword (moved from future-reserved to keyword)
//! - Getter/setter syntax in object literals: `{ get x() {}, set x(v) {} }`
//! - String line continuation (backslash before newline)
//! - Trailing commas in object literals
//!
//! # What ES5 does NOT have
//!
//! - No `let`/`const` (added in ES2015)
//! - No class syntax (added in ES2015)
//! - No arrow functions (added in ES2015)
//! - No template literals (added in ES2015)
//! - No modules (added in ES2015)
//!
//! # Architecture
//!
//! ```text
//! es5.tokens           (grammar file on disk)
//!        |
//!        v
//! grammar-tools        (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer  (tokenizes source using TokenGrammar)
//! ```

use std::fs;

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `es5.tokens` grammar file.
///
/// ```text
/// code/
///   grammars/
///     ecmascript/
///       es5.tokens            <-- this is what we want
///   packages/
///     rust/
///       ecmascript-es5-lexer/
///         Cargo.toml          <-- CARGO_MANIFEST_DIR points here
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/ecmascript/es5.tokens")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for ECMAScript 5 source code.
///
/// This function:
/// 1. Reads the `es5.tokens` grammar file from disk.
/// 2. Parses it into a `TokenGrammar` using `grammar-tools`.
/// 3. Constructs a `GrammarLexer` with the grammar and the given source.
///
/// # Panics
///
/// Panics if the grammar file cannot be read or parsed.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ecmascript_es5_lexer::create_es5_lexer;
///
/// let mut lexer = create_es5_lexer("var x = 42;");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// ```
pub fn create_es5_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read es5.tokens: {e}"));

    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse es5.tokens: {e}"));

    GrammarLexer::new(source, &grammar)
}

/// Tokenize ECMAScript 5 source code into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, and tokenization in one call.
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed, or if the source
/// contains an unexpected character.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ecmascript_es5_lexer::tokenize_es5;
///
/// let tokens = tokenize_es5("debugger;");
/// ```
pub fn tokenize_es5(source: &str) -> Vec<Token> {
    let mut lexer = create_es5_lexer(source);

    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("ES5 tokenization failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    fn token_pairs(tokens: &[Token]) -> Vec<(TokenType, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: Simple variable declaration
    // -----------------------------------------------------------------------

    #[test]
    fn test_tokenize_var_declaration() {
        let tokens = tokenize_es5("var x = 42;");
        let pairs = token_pairs(&tokens);

        assert!(pairs.len() >= 5, "Expected at least 5 tokens, got {}", pairs.len());
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "var");
    }

    // -----------------------------------------------------------------------
    // Test 2: debugger is a KEYWORD in ES5
    // -----------------------------------------------------------------------

    /// The key lexical change in ES5: `debugger` moves from future-reserved
    /// (ES3) to a full keyword. The `debugger` statement acts as a breakpoint
    /// — when a debugger is attached, execution pauses.
    #[test]
    fn test_debugger_keyword() {
        let tokens = tokenize_es5("debugger;");
        let pairs = token_pairs(&tokens);

        assert_eq!(
            pairs[0].0, TokenType::Keyword,
            "Expected 'debugger' to be KEYWORD in ES5, got {:?}",
            pairs[0].0
        );
        assert_eq!(pairs[0].1, "debugger");
    }

    // -----------------------------------------------------------------------
    // Test 3: Strict equality (inherited from ES3)
    // -----------------------------------------------------------------------

    /// ES5 retains `===` and `!==` from ES3.
    #[test]
    fn test_strict_equality() {
        let tokens = tokenize_es5("a === b !== c;");
        let pairs = token_pairs(&tokens);

        let has_triple_eq = pairs.iter().any(|(_, v)| *v == "===");
        let has_strict_neq = pairs.iter().any(|(_, v)| *v == "!==");

        assert!(has_triple_eq, "Expected '===' token in ES5");
        assert!(has_strict_neq, "Expected '!==' token in ES5");
    }

    // -----------------------------------------------------------------------
    // Test 4: try/catch/finally/throw still keywords
    // -----------------------------------------------------------------------

    /// ES5 retains all ES3 error-handling keywords.
    #[test]
    fn test_try_catch_keywords() {
        let error_keywords = ["try", "catch", "finally", "throw"];

        for kw in &error_keywords {
            let source = format!("{kw};");
            let tokens = tokenize_es5(&source);
            let pairs = token_pairs(&tokens);

            assert_eq!(
                pairs[0].0, TokenType::Keyword,
                "Expected '{}' to be a KEYWORD in ES5, got {:?}",
                kw, pairs[0].0
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 5: All ES3 keywords still present
    // -----------------------------------------------------------------------

    #[test]
    fn test_es3_keywords_still_present() {
        let es3_keywords = [
            "break", "case", "catch", "continue", "default", "delete",
            "do", "else", "finally", "for", "function", "if", "in",
            "instanceof", "new", "return", "switch", "this", "throw",
            "try", "typeof", "var", "void", "while", "with",
            "true", "false", "null",
        ];

        for kw in &es3_keywords {
            let source = format!("{kw};");
            let tokens = tokenize_es5(&source);
            let pairs = token_pairs(&tokens);

            assert_eq!(
                pairs[0].0, TokenType::Keyword,
                "Expected '{}' to be a KEYWORD in ES5, got {:?}",
                kw, pairs[0].0
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 6: Factory function
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_lexer() {
        let mut lexer = create_es5_lexer("42;");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 7: Comments are skipped
    // -----------------------------------------------------------------------

    #[test]
    fn test_comments_skipped() {
        let tokens = tokenize_es5("var x = 1; // comment\nvar y = 2;");
        let pairs = token_pairs(&tokens);

        let names: Vec<&str> = pairs.iter()
            .filter(|(t, _)| *t == TokenType::Name)
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    // -----------------------------------------------------------------------
    // Test 8: String literals
    // -----------------------------------------------------------------------

    #[test]
    fn test_strings() {
        let tokens = tokenize_es5("var s = \"hello world\";");
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token");
    }

    // -----------------------------------------------------------------------
    // Test 9: Delimiters
    // -----------------------------------------------------------------------

    #[test]
    fn test_delimiters() {
        let tokens = tokenize_es5("(){}[];,");
        let pairs = token_pairs(&tokens);

        let values: Vec<&str> = pairs.iter().map(|(_, v)| *v).collect();
        assert!(values.contains(&"("));
        assert!(values.contains(&")"));
        assert!(values.contains(&"{"));
        assert!(values.contains(&"}"));
    }

    // -----------------------------------------------------------------------
    // Test 10: let and const are NOT keywords in ES5
    // -----------------------------------------------------------------------

    /// `let` and `const` are NOT keywords in ES5 — they are future-reserved
    /// words. The lexer rejects them as identifiers, which is the correct
    /// behavior for reserved words. `let` is NOT reserved in ES5 non-strict
    /// mode, so it should be a plain NAME.
    #[test]
    fn test_let_is_name_in_es5() {
        // `let` is NOT in the reserved section of es5.tokens (only reserved
        // in strict mode, which is a semantic check), so it should be NAME.
        let tokens = tokenize_es5("let;");
        let pairs = token_pairs(&tokens);

        assert_eq!(
            pairs[0].0, TokenType::Name,
            "Expected 'let' to be NAME in ES5, got {:?}",
            pairs[0].0
        );
    }

    /// `const` IS a future-reserved word in ES5 (in the reserved section
    /// of es5.tokens). The lexer correctly rejects it as an identifier.
    #[test]
    fn test_const_is_reserved_in_es5() {
        let result = std::panic::catch_unwind(|| {
            tokenize_es5("const;")
        });
        assert!(
            result.is_err(),
            "Expected 'const' to be rejected as a reserved word in ES5"
        );
    }

    // -----------------------------------------------------------------------
    // Test 11: Function expression
    // -----------------------------------------------------------------------

    #[test]
    fn test_tokenize_function() {
        let tokens = tokenize_es5("function add(a, b) { return a + b; }");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "function");
    }
}
