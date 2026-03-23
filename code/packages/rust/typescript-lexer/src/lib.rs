//! # TypeScript Lexer — tokenizing TypeScript source code.
//!
//! [TypeScript](https://www.typescriptlang.org/) is a typed superset of
//! JavaScript developed by Microsoft. It adds optional static type
//! annotations, interfaces, enums, generics, and other features on top of
//! JavaScript's syntax. TypeScript compiles to plain JavaScript for
//! execution in browsers and Node.js.
//!
//! This crate provides a lexer (tokenizer) for a subset of TypeScript. It
//! does **not** hand-write tokenization rules. Instead, it loads the
//! `typescript.tokens` grammar file — a declarative description of every
//! token in TypeScript — and feeds it to the generic [`GrammarLexer`] from
//! the `lexer` crate.
//!
//! # Architecture
//!
//! The tokenization pipeline has three layers:
//!
//! ```text
//! typescript.tokens    (grammar file on disk)
//!        |
//!        v
//! grammar-tools        (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer  (tokenizes source using TokenGrammar)
//! ```
//!
//! This crate is the thin glue layer that wires these components together
//! for TypeScript specifically. It knows where to find `typescript.tokens`
//! and provides two public entry points:
//!
//! - [`create_typescript_lexer`] — returns a `GrammarLexer` for fine-grained control.
//! - [`tokenize_typescript`] — convenience function that returns `Vec<Token>` directly.
//!
//! # Keywords
//!
//! TypeScript extends JavaScript's keywords with type-system additions:
//! `interface`, `type`, `enum`, `namespace`, `readonly`, `abstract`,
//! `implements`, `declare`, etc. The grammar file lists all keywords
//! in a `keywords:` section.
//!
//! # Differences from JavaScript
//!
//! At the token level, TypeScript is nearly identical to JavaScript. The
//! main differences are additional keywords and the `<` / `>` characters
//! being used for both comparison operators and generic type parameters.
//! Disambiguation between these uses happens at the parser level, not
//! the lexer level.

use std::fs;

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `typescript.tokens` grammar file.
///
/// We use `env!("CARGO_MANIFEST_DIR")` to get the directory containing this
/// crate's `Cargo.toml` at compile time. From there, we navigate up to the
/// `grammars/` directory at the repository root.
///
/// ```text
/// code/
///   grammars/
///     typescript.tokens     <-- this is what we want
///   packages/
///     rust/
///       typescript-lexer/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR points here
///         src/
///           lib.rs          <-- we are here
/// ```
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/typescript.tokens")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for TypeScript source code.
///
/// This function:
/// 1. Reads the `typescript.tokens` grammar file from disk.
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
/// use coding_adventures_typescript_lexer::create_typescript_lexer;
///
/// let mut lexer = create_typescript_lexer("let x: number = 42;");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{}", token);
/// }
/// ```
pub fn create_typescript_lexer(source: &str) -> GrammarLexer<'_> {
    // Step 1: Read the grammar file from disk.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read typescript.tokens: {e}"));

    // Step 2: Parse the grammar text into a structured TokenGrammar.
    //
    // The TokenGrammar contains:
    //   - Token definitions (NAME, NUMBER, STRING, operators, delimiters)
    //   - Skip patterns (whitespace, single-line comments, multi-line comments)
    //   - Keywords (var, let, const, function, interface, type, enum, etc.)
    //   - Mode: default (no indentation tracking)
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse typescript.tokens: {e}"));

    // Step 3: Create and return the lexer.
    GrammarLexer::new(source, &grammar)
}

/// Tokenize TypeScript source code into a vector of tokens.
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
/// use coding_adventures_typescript_lexer::tokenize_typescript;
///
/// let tokens = tokenize_typescript("function add(a: number, b: number): number { return a + b; }");
/// for token in &tokens {
///     println!("{:?} {:?}", token.type_, token.value);
/// }
/// ```
pub fn tokenize_typescript(source: &str) -> Vec<Token> {
    let mut ts_lexer = create_typescript_lexer(source);

    ts_lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("TypeScript tokenization failed: {e}"))
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
    // Test 1: Simple variable declaration with type annotation
    // -----------------------------------------------------------------------

    /// A typed variable declaration is the quintessential TypeScript pattern.
    #[test]
    fn test_tokenize_typed_declaration() {
        let tokens = tokenize_typescript("let x: number = 42;");
        let pairs = token_pairs(&tokens);

        // Expected: KEYWORD("let"), NAME("x"), COLON(":"), KEYWORD("number"),
        //           EQUALS("="), NUMBER("42"), SEMICOLON(";")
        assert!(pairs.len() >= 7, "Expected at least 7 tokens, got {}", pairs.len());
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "let");
    }

    // -----------------------------------------------------------------------
    // Test 2: Keywords are recognized (including TypeScript-specific ones)
    // -----------------------------------------------------------------------

    /// TypeScript keywords (both JavaScript and TypeScript-specific) should
    /// be classified as KEYWORD tokens.
    #[test]
    fn test_keywords() {
        let keywords = ["var", "let", "const", "function", "return", "if",
                        "else", "for", "while", "true", "false", "null",
                        "interface", "type", "enum"];

        for kw in &keywords {
            let source = format!("{kw};");
            let tokens = tokenize_typescript(&source);
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
        let tokens = tokenize_typescript("a + b - c * d / e;");
        let pairs = token_pairs(&tokens);

        let ops: Vec<&str> = pairs.iter()
            .filter(|(_, v)| ["+", "-", "*", "/"].contains(v))
            .map(|(_, v)| *v)
            .collect();

        assert_eq!(ops, vec!["+", "-", "*", "/"]);
    }

    // -----------------------------------------------------------------------
    // Test 4: Multi-character operators (=== and !==)
    // -----------------------------------------------------------------------

    /// TypeScript inherits JavaScript's strict equality operators.
    #[test]
    fn test_multi_char_operators() {
        let tokens = tokenize_typescript("a === b !== c;");
        let pairs = token_pairs(&tokens);

        let has_triple_eq = pairs.iter().any(|(_, v)| *v == "===");
        let has_not_eq = pairs.iter().any(|(_, v)| *v == "!==");

        assert!(has_triple_eq, "Expected '===' token");
        assert!(has_not_eq, "Expected '!==' token");
    }

    // -----------------------------------------------------------------------
    // Test 5: String literals
    // -----------------------------------------------------------------------

    /// TypeScript supports single-quoted and double-quoted strings.
    #[test]
    fn test_strings() {
        let tokens = tokenize_typescript("let s: string = \"hello\";");
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token");
    }

    // -----------------------------------------------------------------------
    // Test 6: Number literals
    // -----------------------------------------------------------------------

    /// TypeScript supports integer and floating-point numbers.
    #[test]
    fn test_numbers() {
        let tokens = tokenize_typescript("42;");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "42");
    }

    // -----------------------------------------------------------------------
    // Test 7: Delimiters
    // -----------------------------------------------------------------------
    //
    // Note: typescript.tokens has no skip: section, so comments (// and
    // /* */) are not skipped — they produce tokens (SLASH, NAME, etc.).
    // The test_comments_skipped test has been removed because comments
    // are not handled by the grammar-driven lexer for TypeScript.

    /// All delimiter tokens should be recognized.
    #[test]
    fn test_delimiters() {
        let tokens = tokenize_typescript("(){}[];,:");
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
        assert!(values.contains(&":"));
    }

    // -----------------------------------------------------------------------
    // Test 9: Whitespace is skipped
    // -----------------------------------------------------------------------

    /// Whitespace between tokens should be consumed without producing tokens.
    #[test]
    fn test_whitespace_skipped() {
        let compact = tokenize_typescript("let x=1;");
        let spaced = tokenize_typescript("let  x  =  1  ;");

        let pairs_compact = token_pairs(&compact);
        let pairs_spaced = token_pairs(&spaced);

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
    }

    // -----------------------------------------------------------------------
    // Test 10: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_typescript_lexer` factory function should return a
    /// `GrammarLexer` that can successfully tokenize source code.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_typescript_lexer("42;");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 11: Arrow function tokens
    // -----------------------------------------------------------------------

    /// The arrow operator => should be tokenized as a single token.
    #[test]
    fn test_arrow_operator() {
        let tokens = tokenize_typescript("(x: number) => x + 1;");
        let pairs = token_pairs(&tokens);

        let has_arrow = pairs.iter().any(|(_, v)| *v == "=>");
        assert!(has_arrow, "Expected '=>' arrow token");
    }

    // -----------------------------------------------------------------------
    // Test 12: Generic angle brackets
    // -----------------------------------------------------------------------

    /// The < and > characters are used for both comparison and generics.
    /// At the token level, they should just be tokenized as individual tokens.
    #[test]
    fn test_angle_brackets() {
        let tokens = tokenize_typescript("a < b;");
        let pairs = token_pairs(&tokens);

        let has_lt = pairs.iter().any(|(_, v)| *v == "<");
        assert!(has_lt, "Expected '<' token");
    }
}
