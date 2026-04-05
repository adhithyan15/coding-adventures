//! # ECMAScript 1 (1997) Lexer — tokenizing ES1 JavaScript source code.
//!
//! [ECMAScript 1](https://www.ecma-international.org/publications-and-standards/standards/ecma-262/)
//! was the very first standardized version of JavaScript, published in June 1997.
//! Brendan Eich created the language for Netscape Navigator in 1995; two years
//! later, ECMA International published this specification.
//!
//! This crate provides a lexer (tokenizer) specifically for the ES1 subset of
//! JavaScript. It does **not** hand-write tokenization rules. Instead, it loads
//! the `es1.tokens` grammar file — a declarative description of every token in
//! ES1 — and feeds it to the generic [`GrammarLexer`] from the `lexer` crate.
//!
//! # What ES1 has
//!
//! - 23 keywords (break, case, continue, default, delete, do, else, for,
//!   function, if, in, new, return, switch, this, typeof, var, void, while, with,
//!   true, false, null)
//! - Basic operators: arithmetic, bitwise, logical, comparison, assignment
//! - The `==` and `!=` operators (abstract equality with type coercion)
//! - String literals (single and double quoted)
//! - Numeric literals (decimal integers, floats, hex with 0x prefix)
//!
//! # What ES1 does NOT have
//!
//! - No `===` or `!==` (strict equality — added in ES3)
//! - No `try`/`catch`/`finally`/`throw` (error handling — added in ES3)
//! - No regex literals (implementation-defined in ES1 — formalized in ES3)
//! - No template literals, arrow functions, let/const (added in ES2015)
//!
//! # Architecture
//!
//! ```text
//! es1.tokens           (grammar file on disk)
//!        |
//!        v
//! grammar-tools        (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer  (tokenizes source using TokenGrammar)
//! ```
//!
//! This crate is the thin glue layer that wires these components together
//! for ES1 specifically. It knows where to find `es1.tokens` and provides
//! two public entry points:
//!
//! - [`create_es1_lexer`] — returns a `GrammarLexer` for fine-grained control.
//! - [`tokenize_es1`] — convenience function that returns `Vec<Token>` directly.

use std::fs;

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `es1.tokens` grammar file.
///
/// We use `env!("CARGO_MANIFEST_DIR")` to get the directory containing this
/// crate's `Cargo.toml` at compile time. From there, we navigate up to the
/// `grammars/ecmascript/` directory at the repository root.
///
/// ```text
/// code/
///   grammars/
///     ecmascript/
///       es1.tokens            <-- this is what we want
///   packages/
///     rust/
///       ecmascript-es1-lexer/
///         Cargo.toml          <-- CARGO_MANIFEST_DIR points here
///         src/
///           lib.rs            <-- we are here
/// ```
///
/// So the relative path from CARGO_MANIFEST_DIR to the grammar file is:
/// `../../../grammars/ecmascript/es1.tokens`
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/ecmascript/es1.tokens")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for ECMAScript 1 source code.
///
/// This function:
/// 1. Reads the `es1.tokens` grammar file from disk.
/// 2. Parses it into a `TokenGrammar` using `grammar-tools`.
/// 3. Constructs a `GrammarLexer` with the grammar and the given source.
///
/// The returned lexer is ready to call `.tokenize()` on.
///
/// # Panics
///
/// Panics if the grammar file cannot be read or parsed.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ecmascript_es1_lexer::create_es1_lexer;
///
/// let mut lexer = create_es1_lexer("var x = 42;");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{}", token);
/// }
/// ```
pub fn create_es1_lexer(source: &str) -> GrammarLexer<'_> {
    // Step 1: Read the grammar file from disk.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read es1.tokens: {e}"));

    // Step 2: Parse the grammar text into a structured TokenGrammar.
    //
    // The TokenGrammar contains:
    //   - Token definitions (NAME, NUMBER, STRING, operators, delimiters)
    //   - Skip patterns (whitespace, single-line comments, multi-line comments)
    //   - Keywords (break, case, continue, default, delete, do, else, for,
    //     function, if, in, new, return, switch, this, typeof, var, void,
    //     while, with, true, false, null)
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse es1.tokens: {e}"));

    // Step 3: Create and return the lexer.
    GrammarLexer::new(source, &grammar)
}

/// Tokenize ECMAScript 1 source code into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, and tokenization in one call. The returned vector always
/// ends with an `EOF` token.
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed, or if the source
/// contains an unexpected character (via `LexerError` propagation).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_ecmascript_es1_lexer::tokenize_es1;
///
/// let tokens = tokenize_es1("function add(a, b) { return a + b; }");
/// for token in &tokens {
///     println!("{:?} {:?}", token.type_, token.value);
/// }
/// ```
pub fn tokenize_es1(source: &str) -> Vec<Token> {
    let mut lexer = create_es1_lexer(source);

    lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("ES1 tokenization failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

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
    // Test 1: Simple variable declaration
    // -----------------------------------------------------------------------

    /// Verify that a basic `var` declaration is tokenized correctly.
    /// ES1 uses `var` — there is no `let` or `const`.
    #[test]
    fn test_tokenize_var_declaration() {
        let tokens = tokenize_es1("var x = 42;");
        let pairs = token_pairs(&tokens);

        // Expected: KEYWORD("var"), NAME("x"), EQUALS("="), NUMBER("42"), SEMICOLON(";")
        assert!(pairs.len() >= 5, "Expected at least 5 tokens, got {}", pairs.len());
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "var");
        assert_eq!(pairs[1].0, TokenType::Name);
        assert_eq!(pairs[1].1, "x");
    }

    // -----------------------------------------------------------------------
    // Test 2: ES1 keywords are recognized
    // -----------------------------------------------------------------------

    /// ES1 keywords should be classified as KEYWORD tokens, not NAME.
    /// ES1 has 23 keywords (no try, catch, finally, throw, instanceof).
    #[test]
    fn test_es1_keywords() {
        let keywords = [
            "break", "case", "continue", "default", "delete", "do",
            "else", "for", "function", "if", "in", "new", "return",
            "switch", "this", "typeof", "var", "void", "while", "with",
            "true", "false", "null",
        ];

        for kw in &keywords {
            let source = format!("{kw};");
            let tokens = tokenize_es1(&source);
            let pairs = token_pairs(&tokens);

            assert_eq!(
                pairs[0].0, TokenType::Keyword,
                "Expected '{}' to be a KEYWORD, got {:?}",
                kw, pairs[0].0
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: ES1 does NOT have strict equality (=== / !==)
    // -----------------------------------------------------------------------

    /// ES1 only has `==` and `!=` (abstract equality with type coercion).
    /// The `===` and `!==` operators were added in ES3. In ES1, `===` should
    /// be tokenized as `==` followed by `=`, not as a single token.
    #[test]
    fn test_no_strict_equality() {
        let tokens = tokenize_es1("a == b;");
        let pairs = token_pairs(&tokens);

        let has_double_eq = pairs.iter().any(|(_, v)| *v == "==");
        assert!(has_double_eq, "Expected '==' token in ES1");

        // In ES1, === should NOT be a single token
        let has_triple_eq = pairs.iter().any(|(_, v)| *v == "===");
        assert!(!has_triple_eq, "ES1 should NOT have '===' as a single token");
    }

    // -----------------------------------------------------------------------
    // Test 4: try/catch are NOT keywords in ES1
    // -----------------------------------------------------------------------

    /// `try` and `catch` were added in ES3. In ES1, they should be
    /// regular identifiers (NAME tokens), not keywords.
    #[test]
    fn test_try_catch_not_keywords() {
        let tokens = tokenize_es1("try;");
        let pairs = token_pairs(&tokens);

        assert_eq!(
            pairs[0].0, TokenType::Name,
            "Expected 'try' to be NAME in ES1, got {:?}",
            pairs[0].0
        );

        let tokens = tokenize_es1("catch;");
        let pairs = token_pairs(&tokens);

        assert_eq!(
            pairs[0].0, TokenType::Name,
            "Expected 'catch' to be NAME in ES1, got {:?}",
            pairs[0].0
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Arithmetic operators
    // -----------------------------------------------------------------------

    /// Arithmetic operators should be tokenized correctly.
    #[test]
    fn test_operators() {
        let tokens = tokenize_es1("a + b - c * d / e;");
        let pairs = token_pairs(&tokens);

        let ops: Vec<&str> = pairs.iter()
            .filter(|(_, v)| ["+", "-", "*", "/"].contains(v))
            .map(|(_, v)| *v)
            .collect();

        assert_eq!(ops, vec!["+", "-", "*", "/"]);
    }

    // -----------------------------------------------------------------------
    // Test 6: Abstract equality operators (== and !=)
    // -----------------------------------------------------------------------

    /// ES1 has `==` and `!=` for abstract (type-coercing) equality checks.
    #[test]
    fn test_abstract_equality() {
        let tokens = tokenize_es1("a == b != c;");
        let pairs = token_pairs(&tokens);

        let has_eq = pairs.iter().any(|(_, v)| *v == "==");
        let has_neq = pairs.iter().any(|(_, v)| *v == "!=");

        assert!(has_eq, "Expected '==' token");
        assert!(has_neq, "Expected '!=' token");
    }

    // -----------------------------------------------------------------------
    // Test 7: String literals
    // -----------------------------------------------------------------------

    /// ES1 supports both single-quoted and double-quoted strings.
    #[test]
    fn test_strings() {
        let tokens = tokenize_es1("var s = \"hello world\";");
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token");
    }

    // -----------------------------------------------------------------------
    // Test 8: Number literals
    // -----------------------------------------------------------------------

    /// ES1 supports integer and floating-point numbers, plus hex (0x prefix).
    #[test]
    fn test_numbers() {
        let tokens = tokenize_es1("42;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "42");
    }

    // -----------------------------------------------------------------------
    // Test 9: Delimiters
    // -----------------------------------------------------------------------

    /// All delimiter tokens should be recognized: ( ) { } [ ] ; , .
    #[test]
    fn test_delimiters() {
        let tokens = tokenize_es1("(){}[];,");
        let pairs = token_pairs(&tokens);

        let values: Vec<&str> = pairs.iter().map(|(_, v)| *v).collect();
        assert!(values.contains(&"("));
        assert!(values.contains(&")"));
        assert!(values.contains(&"{"));
        assert!(values.contains(&"}"));
        assert!(values.contains(&"["));
        assert!(values.contains(&"]"));
        assert!(values.contains(&";"));
        assert!(values.contains(&","));
    }

    // -----------------------------------------------------------------------
    // Test 10: Whitespace is skipped
    // -----------------------------------------------------------------------

    /// Whitespace between tokens should be consumed without producing tokens.
    #[test]
    fn test_whitespace_skipped() {
        let compact = tokenize_es1("var x=1;");
        let spaced = tokenize_es1("var  x  =  1  ;");

        let pairs_compact = token_pairs(&compact);
        let pairs_spaced = token_pairs(&spaced);

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
    }

    // -----------------------------------------------------------------------
    // Test 11: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_es1_lexer` factory function should return a
    /// `GrammarLexer` that can successfully tokenize source code.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_es1_lexer("42;");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 12: Function expression
    // -----------------------------------------------------------------------

    /// A function declaration exercises keywords, identifiers, parentheses,
    /// braces, and the return keyword.
    #[test]
    fn test_tokenize_function() {
        let tokens = tokenize_es1("function add(a, b) { return a + b; }");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "function");
        assert_eq!(pairs[1].0, TokenType::Name);
        assert_eq!(pairs[1].1, "add");
    }

    // -----------------------------------------------------------------------
    // Test 13: Comments are skipped
    // -----------------------------------------------------------------------

    /// ES1 supports single-line (//) and multi-line (/* */) comments.
    /// The skip patterns in es1.tokens should consume them.
    #[test]
    fn test_comments_skipped() {
        let tokens = tokenize_es1("var x = 1; // this is a comment\nvar y = 2;");
        let pairs = token_pairs(&tokens);

        // Should have tokens for both var statements but no comment tokens
        let names: Vec<&str> = pairs.iter()
            .filter(|(t, _)| *t == TokenType::Name)
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    // -----------------------------------------------------------------------
    // Test 14: instanceof is NOT a keyword in ES1
    // -----------------------------------------------------------------------

    /// `instanceof` was added in ES3. In ES1, it should be a NAME.
    #[test]
    fn test_instanceof_not_keyword() {
        let tokens = tokenize_es1("instanceof;");
        let pairs = token_pairs(&tokens);

        assert_eq!(
            pairs[0].0, TokenType::Name,
            "Expected 'instanceof' to be NAME in ES1, got {:?}",
            pairs[0].0
        );
    }
}
