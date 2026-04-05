//! # ECMAScript 3 (1999) Lexer — tokenizing ES3 JavaScript source code.
//!
//! [ECMAScript 3](https://www.ecma-international.org/publications-and-standards/standards/ecma-262/)
//! was published in December 1999 and is the version that made JavaScript a
//! real, complete language. It landed two years after ES1 and added features
//! that developers today consider fundamental.
//!
//! This crate provides a lexer (tokenizer) specifically for the ES3 subset of
//! JavaScript. It loads the `es3.tokens` grammar file and feeds it to the
//! generic [`GrammarLexer`] from the `lexer` crate.
//!
//! # What ES3 adds over ES1
//!
//! - `===` and `!==` (strict equality — no type coercion)
//! - `try`/`catch`/`finally`/`throw` (structured error handling)
//! - Regular expression literals (`/pattern/flags`)
//! - `instanceof` operator
//! - Expanded future-reserved words
//!
//! # What ES3 does NOT have
//!
//! - No getters/setters in object literals (added in ES5)
//! - No strict mode (added in ES5)
//! - No `debugger` keyword (future-reserved in ES3, promoted in ES5)
//! - No let/const/class/arrow functions (added in ES2015)
//!
//! # Architecture
//!
//! ```text
//! es3.tokens           (grammar file on disk)
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

/// Build the path to the `es3.tokens` grammar file.
///
/// ```text
/// code/
///   grammars/
///     ecmascript/
///       es3.tokens            <-- this is what we want
///   packages/
///     rust/
///       ecmascript-es3-lexer/
///         Cargo.toml          <-- CARGO_MANIFEST_DIR points here
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/ecmascript/es3.tokens")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for ECMAScript 3 source code.
///
/// This function:
/// 1. Reads the `es3.tokens` grammar file from disk.
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
/// use coding_adventures_ecmascript_es3_lexer::create_es3_lexer;
///
/// let mut lexer = create_es3_lexer("var x = 42;");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// ```
pub fn create_es3_lexer(source: &str) -> GrammarLexer<'_> {
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read es3.tokens: {e}"));

    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse es3.tokens: {e}"));

    GrammarLexer::new(source, &grammar)
}

/// Tokenize ECMAScript 3 source code into a vector of tokens.
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
/// use coding_adventures_ecmascript_es3_lexer::tokenize_es3;
///
/// let tokens = tokenize_es3("try { x(); } catch (e) { }");
/// ```
pub fn tokenize_es3(source: &str) -> Vec<Token> {
    let mut lexer = create_es3_lexer(source);

    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("ES3 tokenization failed: {e}"))
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
        let tokens = tokenize_es3("var x = 42;");
        let pairs = token_pairs(&tokens);

        assert!(pairs.len() >= 5, "Expected at least 5 tokens, got {}", pairs.len());
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "var");
    }

    // -----------------------------------------------------------------------
    // Test 2: ES3 has strict equality (=== and !==)
    // -----------------------------------------------------------------------

    /// The defining lexical addition in ES3: strict equality operators.
    /// Unlike `==` and `!=`, these do NOT perform type coercion.
    ///
    /// ```
    /// "" == 0     // true  (abstract equality coerces types)
    /// "" === 0    // false (strict equality, different types)
    /// ```
    #[test]
    fn test_strict_equality() {
        let tokens = tokenize_es3("a === b !== c;");
        let pairs = token_pairs(&tokens);

        let has_triple_eq = pairs.iter().any(|(_, v)| *v == "===");
        let has_strict_neq = pairs.iter().any(|(_, v)| *v == "!==");

        assert!(has_triple_eq, "Expected '===' token in ES3");
        assert!(has_strict_neq, "Expected '!==' token in ES3");
    }

    // -----------------------------------------------------------------------
    // Test 3: try/catch/finally/throw are keywords in ES3
    // -----------------------------------------------------------------------

    /// ES3 adds structured error handling keywords.
    #[test]
    fn test_try_catch_keywords() {
        let error_keywords = ["try", "catch", "finally", "throw"];

        for kw in &error_keywords {
            let source = format!("{kw};");
            let tokens = tokenize_es3(&source);
            let pairs = token_pairs(&tokens);

            assert_eq!(
                pairs[0].0, TokenType::Keyword,
                "Expected '{}' to be a KEYWORD in ES3, got {:?}",
                kw, pairs[0].0
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 4: instanceof is a keyword in ES3
    // -----------------------------------------------------------------------

    /// ES3 adds `instanceof` for prototype chain checking.
    #[test]
    fn test_instanceof_keyword() {
        let tokens = tokenize_es3("instanceof;");
        let pairs = token_pairs(&tokens);

        assert_eq!(
            pairs[0].0, TokenType::Keyword,
            "Expected 'instanceof' to be KEYWORD in ES3, got {:?}",
            pairs[0].0
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: ES3 keywords superset of ES1
    // -----------------------------------------------------------------------

    /// All ES1 keywords should still be keywords in ES3.
    #[test]
    fn test_es1_keywords_still_present() {
        let es1_keywords = [
            "break", "case", "continue", "default", "delete", "do",
            "else", "for", "function", "if", "in", "new", "return",
            "switch", "this", "typeof", "var", "void", "while", "with",
            "true", "false", "null",
        ];

        for kw in &es1_keywords {
            let source = format!("{kw};");
            let tokens = tokenize_es3(&source);
            let pairs = token_pairs(&tokens);

            assert_eq!(
                pairs[0].0, TokenType::Keyword,
                "Expected '{}' to be a KEYWORD in ES3, got {:?}",
                kw, pairs[0].0
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 6: debugger is NOT a keyword in ES3
    // -----------------------------------------------------------------------

    /// `debugger` is a future-reserved word in ES3, not a keyword.
    /// It becomes a keyword in ES5. In our lexer, reserved words cause
    /// a tokenization error when used as identifiers — this verifies that
    /// `debugger` is indeed treated as reserved (not a plain NAME).
    #[test]
    fn test_debugger_is_reserved() {
        // The lexer should reject `debugger` as a reserved word in ES3.
        // We verify this by attempting to tokenize it and expecting an error.
        let result = std::panic::catch_unwind(|| {
            tokenize_es3("debugger;")
        });
        assert!(
            result.is_err(),
            "Expected 'debugger' to be rejected as a reserved word in ES3"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: Abstract equality still works
    // -----------------------------------------------------------------------

    /// ES3 still has `==` and `!=` alongside the new `===` and `!==`.
    #[test]
    fn test_abstract_equality() {
        let tokens = tokenize_es3("a == b != c;");
        let pairs = token_pairs(&tokens);

        let has_eq = pairs.iter().any(|(_, v)| *v == "==");
        let has_neq = pairs.iter().any(|(_, v)| *v == "!=");

        assert!(has_eq, "Expected '==' token");
        assert!(has_neq, "Expected '!=' token");
    }

    // -----------------------------------------------------------------------
    // Test 8: Factory function
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_lexer() {
        let mut lexer = create_es3_lexer("42;");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 9: String literals
    // -----------------------------------------------------------------------

    #[test]
    fn test_strings() {
        let tokens = tokenize_es3("var s = \"hello world\";");
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token");
    }

    // -----------------------------------------------------------------------
    // Test 10: Comments are skipped
    // -----------------------------------------------------------------------

    #[test]
    fn test_comments_skipped() {
        let tokens = tokenize_es3("var x = 1; // comment\nvar y = 2;");
        let pairs = token_pairs(&tokens);

        let names: Vec<&str> = pairs.iter()
            .filter(|(t, _)| *t == TokenType::Name)
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    // -----------------------------------------------------------------------
    // Test 11: Function expression
    // -----------------------------------------------------------------------

    #[test]
    fn test_tokenize_function() {
        let tokens = tokenize_es3("function add(a, b) { return a + b; }");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "function");
    }
}
