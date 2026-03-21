//! # CSS Lexer — tokenizing CSS source code.
//!
//! [CSS](https://www.w3.org/Style/CSS/) (Cascading Style Sheets) is the
//! language that describes the presentation of HTML and XML documents. CSS
//! tokenization is notably more complex than JSON or Starlark because of
//! compound tokens (e.g., `10px` is a single DIMENSION token, not NUMBER +
//! IDENT), context-dependent disambiguation (e.g., `#fff` as a color vs.
//! `#header` as an ID selector — both are HASH tokens, with the grammar
//! handling disambiguation), and diverse token types (at-keywords, function
//! tokens, URL tokens, unicode ranges, and CSS nesting).
//!
//! This crate provides a lexer (tokenizer) for CSS. It does **not**
//! hand-write tokenization rules. Instead, it loads the `css.tokens`
//! grammar file — a declarative description of every token in CSS — and
//! feeds it to the generic [`GrammarLexer`] from the `lexer` crate.
//!
//! # Architecture
//!
//! The tokenization pipeline has three layers:
//!
//! ```text
//! css.tokens           (grammar file on disk)
//!        |
//!        v
//! grammar-tools        (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer  (tokenizes source using TokenGrammar)
//! ```
//!
//! This crate is the thin glue layer that wires these components together
//! for CSS specifically. It knows where to find `css.tokens` and provides
//! two public entry points:
//!
//! - [`create_css_lexer`] — returns a `GrammarLexer` for fine-grained control.
//! - [`tokenize_css`] — convenience function that returns `Vec<Token>` directly.
//!
//! # Why grammar-driven instead of hand-written?
//!
//! A hand-written CSS lexer would be hundreds of lines of Rust with intricate
//! logic for dimension tokens, escape sequences, URL tokens, and at-keywords.
//! The grammar-driven approach replaces all of that with a declarative grammar
//! file plus ~30 lines of Rust glue code. When CSS evolves (e.g., adding new
//! function tokens or at-rules), you edit the grammar file — no Rust code
//! changes needed.

use std::fs;

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `css.tokens` grammar file.
///
/// We use `env!("CARGO_MANIFEST_DIR")` to get the directory containing this
/// crate's `Cargo.toml` at compile time. From there, we navigate up to the
/// `grammars/` directory at the repository root.
///
/// The directory structure looks like:
///
/// ```text
/// code/
///   grammars/
///     css.tokens            <-- this is what we want
///   packages/
///     rust/
///       css-lexer/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR points here
///         src/
///           lib.rs          <-- we are here
/// ```
///
/// So the relative path from CARGO_MANIFEST_DIR to the grammar file is:
/// `../../../grammars/css.tokens`
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/css.tokens")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for CSS source code.
///
/// This function:
/// 1. Reads the `css.tokens` grammar file from disk.
/// 2. Parses it into a `TokenGrammar` using `grammar-tools`.
/// 3. Constructs a `GrammarLexer` with the grammar and the given source.
///
/// The returned lexer is ready to call `.tokenize()` on. Use this when you
/// need access to the lexer object itself (e.g., for incremental tokenization
/// or custom error handling).
///
/// # Panics
///
/// Panics if the grammar file cannot be read or parsed. This should never
/// happen in practice — the grammar file is checked into the repository and
/// validated by the grammar-tools test suite. A panic here indicates a
/// broken build or missing file.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_css_lexer::create_css_lexer;
///
/// let mut lexer = create_css_lexer("body { color: red; }");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{}", token);
/// }
/// ```
pub fn create_css_lexer(source: &str) -> GrammarLexer<'_> {
    // Step 1: Read the grammar file from disk.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read css.tokens: {e}"));

    // Step 2: Parse the grammar text into a structured TokenGrammar.
    //
    // The TokenGrammar contains:
    //   - Token definitions (DIMENSION, NUMBER, HASH, STRING, IDENT, etc.)
    //   - Skip patterns (whitespace, comments)
    //   - No keywords (CSS does not have reserved keywords at the token level)
    //   - Mode: default (no indentation tracking)
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse css.tokens: {e}"));

    // Step 3: Create and return the lexer.
    GrammarLexer::new(source, &grammar)
}

/// Tokenize CSS source code into a vector of tokens.
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
/// use coding_adventures_css_lexer::tokenize_css;
///
/// let tokens = tokenize_css("h1 { font-size: 16px; }");
/// for token in &tokens {
///     println!("{:?} {:?}", token.type_, token.value);
/// }
/// ```
pub fn tokenize_css(source: &str) -> Vec<Token> {
    let mut css_lexer = create_css_lexer(source);

    css_lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("CSS tokenization failed: {e}"))
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
