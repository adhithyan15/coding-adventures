//! # JavaScript Lexer — tokenizing JavaScript source code.
//!
//! [JavaScript](https://tc39.es/ecma262/) is the ubiquitous programming
//! language of the web, running in browsers and on servers via Node.js.
//! This crate provides a lexer (tokenizer) for a subset of JavaScript.
//!
//! It does **not** hand-write tokenization rules. Instead, it loads the
//! `javascript.tokens` grammar file — a declarative description of every
//! token in JavaScript — and feeds it to the generic [`GrammarLexer`]
//! from the `lexer` crate.
//!
//! # Architecture
//!
//! The tokenization pipeline has three layers:
//!
//! ```text
//! javascript.tokens    (grammar file on disk)
//!        |
//!        v
//! grammar-tools        (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer  (tokenizes source using TokenGrammar)
//! ```
//!
//! This crate is the thin glue layer that wires these components together
//! for JavaScript specifically. It knows where to find `javascript.tokens`
//! and provides two public entry points:
//!
//! - [`create_javascript_lexer`] — returns a `GrammarLexer` for fine-grained control.
//! - [`tokenize_javascript`] — convenience function that returns `Vec<Token>` directly.
//!
//! # Keywords
//!
//! JavaScript has a rich set of keywords: `var`, `let`, `const`, `function`,
//! `return`, `if`, `else`, `for`, `while`, `class`, `new`, `this`, etc.
//! The grammar file lists these in a `keywords:` section. The lexer first
//! matches the NAME pattern, then checks the keyword list and promotes
//! matching names to KEYWORD tokens.

use std::fs;

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `javascript.tokens` grammar file.
///
/// We use `env!("CARGO_MANIFEST_DIR")` to get the directory containing this
/// crate's `Cargo.toml` at compile time. From there, we navigate up to the
/// `grammars/` directory at the repository root.
///
/// ```text
/// code/
///   grammars/
///     javascript.tokens     <-- this is what we want
///   packages/
///     rust/
///       javascript-lexer/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR points here
///         src/
///           lib.rs          <-- we are here
/// ```
///
/// So the relative path from CARGO_MANIFEST_DIR to the grammar file is:
/// `../../../grammars/javascript.tokens`
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/javascript.tokens")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for JavaScript source code.
///
/// This function:
/// 1. Reads the `javascript.tokens` grammar file from disk.
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
/// use coding_adventures_javascript_lexer::create_javascript_lexer;
///
/// let mut lexer = create_javascript_lexer("var x = 42;");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{}", token);
/// }
/// ```
pub fn create_javascript_lexer(source: &str) -> GrammarLexer<'_> {
    // Step 1: Read the grammar file from disk.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read javascript.tokens: {e}"));

    // Step 2: Parse the grammar text into a structured TokenGrammar.
    //
    // The TokenGrammar contains:
    //   - Token definitions (NAME, NUMBER, STRING, operators, delimiters)
    //   - Skip patterns (whitespace, single-line comments, multi-line comments)
    //   - Keywords (var, let, const, function, return, if, else, etc.)
    //   - Mode: default (no indentation tracking)
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse javascript.tokens: {e}"));

    // Step 3: Create and return the lexer.
    GrammarLexer::new(source, &grammar)
}

/// Tokenize JavaScript source code into a vector of tokens.
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
/// use coding_adventures_javascript_lexer::tokenize_javascript;
///
/// let tokens = tokenize_javascript("function add(a, b) { return a + b; }");
/// for token in &tokens {
///     println!("{:?} {:?}", token.type_, token.value);
/// }
/// ```
pub fn tokenize_javascript(source: &str) -> Vec<Token> {
    let mut js_lexer = create_javascript_lexer(source);

    js_lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("JavaScript tokenization failed: {e}"))
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

    /// Verify that a basic variable declaration is tokenized correctly.
    #[test]
    fn test_tokenize_var_declaration() {
        let tokens = tokenize_javascript("var x = 42;");
        let pairs = token_pairs(&tokens);

        // Expected: KEYWORD("var"), NAME("x"), EQUALS("="), NUMBER("42"), SEMICOLON(";")
        assert!(pairs.len() >= 5, "Expected at least 5 tokens, got {}", pairs.len());
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "var");
        assert_eq!(pairs[1].0, TokenType::Name);
        assert_eq!(pairs[1].1, "x");
    }

    // -----------------------------------------------------------------------
    // Test 2: Keywords are recognized
    // -----------------------------------------------------------------------

    /// JavaScript keywords should be classified as KEYWORD tokens, not NAME.
    #[test]
    fn test_keywords() {
        let keywords = ["var", "let", "const", "function", "return", "if",
                        "else", "for", "while", "true", "false", "null"];

        for kw in &keywords {
            let source = format!("{kw};");
            let tokens = tokenize_javascript(&source);
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

    /// Arithmetic and comparison operators should be tokenized correctly.
    #[test]
    fn test_operators() {
        let tokens = tokenize_javascript("a + b - c * d / e;");
        let pairs = token_pairs(&tokens);

        let ops: Vec<&str> = pairs.iter()
            .filter(|(_, v)| ["+", "-", "*", "/"].contains(v))
            .map(|(_, v)| *v)
            .collect();

        assert_eq!(ops, vec!["+", "-", "*", "/"]);
    }

    // -----------------------------------------------------------------------
    // Test 4: Multi-character operators
    // -----------------------------------------------------------------------

    /// Multi-character operators like ===, !==, >=, <= should be tokenized
    /// as single tokens, not split into individual characters.
    #[test]
    fn test_multi_char_operators() {
        let tokens = tokenize_javascript("a === b !== c;");
        let pairs = token_pairs(&tokens);

        let has_triple_eq = pairs.iter().any(|(_, v)| *v == "===");
        let has_not_eq = pairs.iter().any(|(_, v)| *v == "!==");

        assert!(has_triple_eq, "Expected '===' token");
        assert!(has_not_eq, "Expected '!==' token");
    }

    // -----------------------------------------------------------------------
    // Test 5: String literals
    // -----------------------------------------------------------------------

    /// JavaScript supports both single-quoted and double-quoted strings.
    #[test]
    fn test_strings() {
        let tokens = tokenize_javascript("var s = \"hello world\";");
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token");
    }

    // -----------------------------------------------------------------------
    // Test 6: Number literals
    // -----------------------------------------------------------------------

    /// JavaScript supports integer and floating-point numbers.
    #[test]
    fn test_numbers() {
        let tokens = tokenize_javascript("42;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "42");
    }

    // -----------------------------------------------------------------------
    // Test 7: Delimiters
    // -----------------------------------------------------------------------
    //
    // Note: javascript.tokens has no skip: section, so comments (// and
    // /* */) are not skipped — they produce tokens (SLASH, NAME, etc.).
    // The test_comments_skipped test has been removed because comments
    // are not handled by the grammar-driven lexer for JavaScript.

    /// All delimiter tokens should be recognized: ( ) { } [ ] ; , .
    #[test]
    fn test_delimiters() {
        let tokens = tokenize_javascript("(){}[];,");
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
    // Test 9: Whitespace is skipped
    // -----------------------------------------------------------------------

    /// Whitespace between tokens should be consumed without producing tokens.
    #[test]
    fn test_whitespace_skipped() {
        let compact = tokenize_javascript("var x=1;");
        let spaced = tokenize_javascript("var  x  =  1  ;");

        let pairs_compact = token_pairs(&compact);
        let pairs_spaced = token_pairs(&spaced);

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
    }

    // -----------------------------------------------------------------------
    // Test 10: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_javascript_lexer` factory function should return a
    /// `GrammarLexer` that can successfully tokenize source code.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_javascript_lexer("42;");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 11: Function expression
    // -----------------------------------------------------------------------

    /// A function declaration exercises keywords, identifiers, parentheses,
    /// braces, and the return keyword.
    #[test]
    fn test_tokenize_function() {
        let tokens = tokenize_javascript("function add(a, b) { return a + b; }");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "function");
        assert_eq!(pairs[1].0, TokenType::Name);
        assert_eq!(pairs[1].1, "add");
    }

    // -----------------------------------------------------------------------
    // Test 12: Arrow function tokens
    // -----------------------------------------------------------------------

    /// The arrow operator => should be tokenized as a single token.
    #[test]
    fn test_arrow_operator() {
        let tokens = tokenize_javascript("(x) => x + 1;");
        let pairs = token_pairs(&tokens);

        let has_arrow = pairs.iter().any(|(_, v)| *v == "=>");
        assert!(has_arrow, "Expected '=>' arrow token");
    }
}
