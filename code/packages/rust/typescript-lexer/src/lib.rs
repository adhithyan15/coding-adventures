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
//! appropriate `.tokens` grammar file — a declarative description of every
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
//! for TypeScript specifically. It knows where to find the grammar files
//! and provides two public entry points:
//!
//! - [`create_typescript_lexer`] — returns a `GrammarLexer` for fine-grained control.
//! - [`tokenize_typescript`] — convenience function that returns `Vec<Token>` directly.
//!
//! # Version-Aware API
//!
//! Both entry points accept an optional `version` parameter. When supplied,
//! the lexer uses a version-specific grammar file:
//!
//! | `version` | grammar file loaded |
//! |---|---|
//! | `""` (empty) | `grammars/typescript.tokens` (generic) |
//! | `"ts1.0"` | `grammars/typescript/ts1.0.tokens` |
//! | `"ts2.0"` | `grammars/typescript/ts2.0.tokens` |
//! | `"ts3.0"` | `grammars/typescript/ts3.0.tokens` |
//! | `"ts4.0"` | `grammars/typescript/ts4.0.tokens` |
//! | `"ts5.0"` | `grammars/typescript/ts5.0.tokens` |
//! | `"ts5.8"` | `grammars/typescript/ts5.8.tokens` |
//!
//! An unknown version string returns `Err(String)`.
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
use std::path::PathBuf;

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Returns the root `grammars/` directory by navigating up from this crate.
///
/// ```text
/// code/
///   grammars/           <-- returned by this function
///   packages/
///     rust/
///       typescript-lexer/
///         Cargo.toml    <-- env!("CARGO_MANIFEST_DIR")
/// ```
fn grammar_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("grammars")
}

/// Validate the TypeScript version string and return the path to the
/// corresponding `.tokens` grammar file.
///
/// Valid version strings are:
/// - `""` — selects the generic `typescript.tokens`
/// - `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, `"ts5.8"`
///   — selects `typescript/<version>.tokens`
///
/// Returns `Err(String)` for any unrecognised version string, so callers
/// can surface a clear error message rather than panicking on a missing file.
fn grammar_path(version: &str) -> Result<PathBuf, String> {
    let root = grammar_root();

    match version {
        // Empty string → the generic, version-agnostic grammar.
        "" => Ok(root.join("typescript.tokens")),

        // Versioned TypeScript grammars live in grammars/typescript/.
        "ts1.0" | "ts2.0" | "ts3.0" | "ts4.0" | "ts5.0" | "ts5.8" => {
            Ok(root.join("typescript").join(format!("{version}.tokens")))
        }

        // Anything else is an error — we'd rather fail loudly than silently
        // fall back to the generic grammar and produce confusing results.
        other => Err(format!(
            "Unknown TypeScript version '{other}'. \
             Valid values: \"\", \"ts1.0\", \"ts2.0\", \"ts3.0\", \
             \"ts4.0\", \"ts5.0\", \"ts5.8\""
        )),
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for TypeScript source code.
///
/// The `version` parameter selects which grammar file to load:
/// - `""` — uses the generic `typescript.tokens` grammar (recommended for
///   most use cases where you don't need version-specific behaviour).
/// - `"ts1.0"` through `"ts5.8"` — uses a version-specific grammar that
///   matches the token set of that TypeScript release.
///
/// This function:
/// 1. Resolves the grammar file path from the version string.
/// 2. Reads the `.tokens` grammar file from disk.
/// 3. Parses it into a `TokenGrammar` using `grammar-tools`.
/// 4. Constructs a `GrammarLexer` with the grammar and the given source.
///
/// The returned lexer is ready to call `.tokenize()` on.
///
/// # Errors
///
/// Returns `Err(String)` if:
/// - The `version` string is not recognised.
/// - The grammar file cannot be read or parsed.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_typescript_lexer::create_typescript_lexer;
///
/// // Generic grammar (version-agnostic):
/// let mut lexer = create_typescript_lexer("let x: number = 42;", "").unwrap();
/// let tokens = lexer.tokenize().expect("tokenization failed");
///
/// // TypeScript 5.8 grammar:
/// let mut lexer58 = create_typescript_lexer("let x: number = 42;", "ts5.8").unwrap();
/// ```
pub fn create_typescript_lexer<'src>(
    source: &'src str,
    version: &str,
) -> Result<GrammarLexer<'src>, String> {
    // Resolve the grammar file path; fail early on unknown version strings.
    let path = grammar_path(version)?;

    // Read the grammar file from disk.
    let grammar_text = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    // Parse the grammar text into a structured TokenGrammar.
    //
    // The TokenGrammar contains:
    //   - Token definitions (NAME, NUMBER, STRING, operators, delimiters)
    //   - Skip patterns (whitespace, comments)
    //   - Keywords (var, let, const, function, interface, type, enum, etc.)
    //   - Mode: default (no indentation tracking)
    let grammar = parse_token_grammar(&grammar_text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    // Create and return the lexer.
    Ok(GrammarLexer::new(source, &grammar))
}

/// Tokenize TypeScript source code into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, and tokenization in one call. The returned vector always
/// ends with an `EOF` token.
///
/// The `version` parameter is the same as for [`create_typescript_lexer`]:
/// pass `""` for the generic grammar or `"ts5.8"` etc. for a versioned one.
///
/// # Errors
///
/// Returns `Err(String)` if the version is unknown, the grammar file is
/// missing or malformed, or the source contains an unrecognised character.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_typescript_lexer::tokenize_typescript;
///
/// // Generic grammar:
/// let tokens = tokenize_typescript(
///     "function add(a: number, b: number): number { return a + b; }",
///     "",
/// ).unwrap();
///
/// // TypeScript 4.0 grammar:
/// let tokens_v4 = tokenize_typescript("let x = 1;", "ts4.0").unwrap();
/// ```
pub fn tokenize_typescript(source: &str, version: &str) -> Result<Vec<Token>, String> {
    // The grammar is owned inside create_typescript_lexer, so we must
    // re-create the lexer here. We call it through a helper that owns the
    // grammar string for the duration of tokenization.
    tokenize_typescript_impl(source, version)
}

/// Internal implementation that owns the grammar string for the duration
/// of tokenization, avoiding lifetime issues with GrammarLexer<'src>.
fn tokenize_typescript_impl(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let path = grammar_path(version)?;

    let grammar_text = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    let grammar = parse_token_grammar(&grammar_text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer
        .tokenize()
        .map_err(|e| format!("TypeScript tokenization failed: {e}"))
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
    // Test 1: Simple variable declaration with type annotation (generic grammar)
    // -----------------------------------------------------------------------

    /// A typed variable declaration is the quintessential TypeScript pattern.
    #[test]
    fn test_tokenize_typed_declaration() {
        let tokens = tokenize_typescript("let x: number = 42;", "").unwrap();
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
            let tokens = tokenize_typescript(&source, "").unwrap();
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
        let tokens = tokenize_typescript("a + b - c * d / e;", "").unwrap();
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
        let tokens = tokenize_typescript("a === b !== c;", "").unwrap();
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
        let tokens = tokenize_typescript("let s: string = \"hello\";", "").unwrap();
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
        let tokens = tokenize_typescript("42;", "").unwrap();
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "42");
    }

    // -----------------------------------------------------------------------
    // Test 7: Delimiters
    // -----------------------------------------------------------------------

    /// All delimiter tokens should be recognized.
    #[test]
    fn test_delimiters() {
        let tokens = tokenize_typescript("(){}[];,:", "").unwrap();
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
    // Test 8: Whitespace is skipped
    // -----------------------------------------------------------------------

    /// Whitespace between tokens should be consumed without producing tokens.
    #[test]
    fn test_whitespace_skipped() {
        let compact = tokenize_typescript("let x=1;", "").unwrap();
        let spaced = tokenize_typescript("let  x  =  1  ;", "").unwrap();

        let pairs_compact = token_pairs(&compact);
        let pairs_spaced = token_pairs(&spaced);

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
    }

    // -----------------------------------------------------------------------
    // Test 9: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_typescript_lexer` factory function should return a
    /// `GrammarLexer` that can successfully tokenize source code.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_typescript_lexer("42;", "").unwrap();
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 10: Arrow function tokens
    // -----------------------------------------------------------------------

    /// The arrow operator => should be tokenized as a single token.
    #[test]
    fn test_arrow_operator() {
        let tokens = tokenize_typescript("(x: number) => x + 1;", "").unwrap();
        let pairs = token_pairs(&tokens);

        let has_arrow = pairs.iter().any(|(_, v)| *v == "=>");
        assert!(has_arrow, "Expected '=>' arrow token");
    }

    // -----------------------------------------------------------------------
    // Test 11: Generic angle brackets
    // -----------------------------------------------------------------------

    /// The < and > characters are used for both comparison and generics.
    /// At the token level, they should just be tokenized as individual tokens.
    #[test]
    fn test_angle_brackets() {
        let tokens = tokenize_typescript("a < b;", "").unwrap();
        let pairs = token_pairs(&tokens);

        let has_lt = pairs.iter().any(|(_, v)| *v == "<");
        assert!(has_lt, "Expected '<' token");
    }

    // -----------------------------------------------------------------------
    // Test 12: Versioned grammars — ts5.8
    // -----------------------------------------------------------------------

    /// The ts5.8 versioned grammar should tokenize the same basic source
    /// successfully (it covers the same fundamental token set).
    #[test]
    fn test_versioned_ts58() {
        let tokens = tokenize_typescript("let x: number = 42;", "ts5.8").unwrap();
        let pairs = token_pairs(&tokens);

        assert!(pairs.len() >= 7, "Expected at least 7 tokens with ts5.8 grammar");
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "let");
    }

    // -----------------------------------------------------------------------
    // Test 13: Versioned grammars — ts1.0 through ts5.0
    // -----------------------------------------------------------------------

    /// Every versioned TypeScript grammar should successfully tokenize a
    /// simple number literal.
    #[test]
    fn test_all_versioned_grammars() {
        let versions = ["ts1.0", "ts2.0", "ts3.0", "ts4.0", "ts5.0", "ts5.8"];
        for v in &versions {
            let result = tokenize_typescript("42;", v);
            assert!(result.is_ok(), "Version '{v}' should parse successfully: {:?}", result.err());

            let tokens = result.unwrap();
            let pairs = token_pairs(&tokens);
            assert_eq!(pairs[0].0, TokenType::Number, "First token for '{v}' should be NUMBER");
        }
    }

    // -----------------------------------------------------------------------
    // Test 14: Unknown version returns Err
    // -----------------------------------------------------------------------

    /// Passing an unrecognised version string should return Err, not panic.
    #[test]
    fn test_unknown_version_returns_err() {
        let result = tokenize_typescript("let x = 1;", "ts99.0");
        assert!(result.is_err(), "Expected Err for unknown version 'ts99.0'");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("ts99.0"),
            "Error message should mention the bad version: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 15: create_typescript_lexer with unknown version returns Err
    // -----------------------------------------------------------------------

    /// The factory function should also return Err for unknown versions.
    #[test]
    fn test_create_lexer_unknown_version() {
        let result = create_typescript_lexer("let x = 1;", "bad-version");
        assert!(result.is_err(), "Expected Err from create_typescript_lexer with bad version");
    }
}
