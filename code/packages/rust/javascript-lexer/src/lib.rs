//! # JavaScript Lexer — tokenizing JavaScript source code.
//!
//! [JavaScript](https://tc39.es/ecma262/) is the ubiquitous programming
//! language of the web, running in browsers and on servers via Node.js.
//! This crate provides a lexer (tokenizer) for a subset of JavaScript.
//!
//! It does **not** hand-write tokenization rules. Instead, it loads the
//! appropriate `.tokens` grammar file — a declarative description of every
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
//! for JavaScript specifically. It knows where to find the grammar files
//! and provides two public entry points:
//!
//! - [`create_javascript_lexer`] — returns a `GrammarLexer` for fine-grained control.
//! - [`tokenize_javascript`] — convenience function that returns `Vec<Token>` directly.
//!
//! # Version-Aware API
//!
//! Both entry points accept a `version` parameter that selects which grammar
//! file to use. The JavaScript (ECMAScript) versioning scheme uses the
//! edition names defined by TC39:
//!
//! | `version` | grammar file loaded |
//! |---|---|
//! | `""` (empty) | `grammars/javascript.tokens` (generic) |
//! | `"es1"` | `grammars/ecmascript/es1.tokens` |
//! | `"es3"` | `grammars/ecmascript/es3.tokens` |
//! | `"es5"` | `grammars/ecmascript/es5.tokens` |
//! | `"es2015"` | `grammars/ecmascript/es2015.tokens` |
//! | `"es2016"` | `grammars/ecmascript/es2016.tokens` |
//! | … | … |
//! | `"es2025"` | `grammars/ecmascript/es2025.tokens` |
//!
//! An unknown version string returns `Err(String)`.
//!
//! # Keywords
//!
//! JavaScript has a rich set of keywords: `var`, `let`, `const`, `function`,
//! `return`, `if`, `else`, `for`, `while`, `class`, `new`, `this`, etc.
//! The grammar file lists these in a `keywords:` section. The lexer first
//! matches the NAME pattern, then checks the keyword list and promotes
//! matching names to KEYWORD tokens.

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
///       javascript-lexer/
///         Cargo.toml    <-- env!("CARGO_MANIFEST_DIR")
/// ```
fn grammar_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("grammars")
}

/// Validate the JavaScript/ECMAScript version string and return the path to
/// the corresponding `.tokens` grammar file.
///
/// Valid version strings are:
/// - `""` — selects the generic `javascript.tokens`
/// - `"es1"`, `"es3"`, `"es5"` — early ECMAScript editions
/// - `"es2015"` through `"es2025"` — annual ECMAScript releases
///
/// Returns `Err(String)` for any unrecognised version string, so callers
/// can surface a clear error message rather than panicking on a missing file.
fn grammar_path(version: &str) -> Result<PathBuf, String> {
    let root = grammar_root();

    match version {
        // Empty string → the generic, version-agnostic grammar.
        "" => Ok(root.join("javascript.tokens")),

        // Versioned ECMAScript grammars live in grammars/ecmascript/.
        "es1" | "es3" | "es5"
        | "es2015" | "es2016" | "es2017" | "es2018" | "es2019"
        | "es2020" | "es2021" | "es2022" | "es2023" | "es2024" | "es2025" => {
            Ok(root.join("ecmascript").join(format!("{version}.tokens")))
        }

        // Anything else is an error — we'd rather fail loudly than silently
        // fall back to the generic grammar and produce confusing results.
        other => Err(format!(
            "Unknown JavaScript/ECMAScript version '{other}'. \
             Valid values: \"\", \"es1\", \"es3\", \"es5\", \
             \"es2015\"–\"es2025\""
        )),
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for JavaScript source code.
///
/// The `version` parameter selects which grammar file to load:
/// - `""` — uses the generic `javascript.tokens` grammar (recommended for
///   most use cases where you don't need version-specific behaviour).
/// - `"es1"`, `"es3"`, `"es5"`, `"es2015"`–`"es2025"` — uses a
///   version-specific grammar for that ECMAScript edition.
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
/// use coding_adventures_javascript_lexer::create_javascript_lexer;
///
/// // Generic grammar (version-agnostic):
/// let mut lexer = create_javascript_lexer("var x = 42;", "").unwrap();
/// let tokens = lexer.tokenize().expect("tokenization failed");
///
/// // ES2015 (ES6) grammar:
/// let mut lexer_es6 = create_javascript_lexer("let x = 42;", "es2015").unwrap();
/// ```
pub fn create_javascript_lexer<'src>(
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
    //   - Keywords (var, let, const, function, return, if, else, etc.)
    //   - Mode: default (no indentation tracking)
    let grammar = parse_token_grammar(&grammar_text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    // Create and return the lexer.
    Ok(GrammarLexer::new(source, &grammar))
}

/// Tokenize JavaScript source code into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, and tokenization in one call. The returned vector always
/// ends with an `EOF` token.
///
/// The `version` parameter is the same as for [`create_javascript_lexer`]:
/// pass `""` for the generic grammar or `"es2015"` etc. for a versioned one.
///
/// # Errors
///
/// Returns `Err(String)` if the version is unknown, the grammar file is
/// missing or malformed, or the source contains an unrecognised character.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_javascript_lexer::tokenize_javascript;
///
/// // Generic grammar:
/// let tokens = tokenize_javascript(
///     "function add(a, b) { return a + b; }",
///     "",
/// ).unwrap();
///
/// // ES5 grammar:
/// let tokens_es5 = tokenize_javascript("var x = 1;", "es5").unwrap();
/// ```
pub fn tokenize_javascript(source: &str, version: &str) -> Result<Vec<Token>, String> {
    tokenize_javascript_impl(source, version)
}

/// Internal implementation that owns the grammar string for the duration
/// of tokenization, avoiding lifetime issues with GrammarLexer<'src>.
fn tokenize_javascript_impl(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let path = grammar_path(version)?;

    let grammar_text = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    let grammar = parse_token_grammar(&grammar_text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer
        .tokenize()
        .map_err(|e| format!("JavaScript tokenization failed: {e}"))
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
    // Test 1: Simple variable declaration (generic grammar)
    // -----------------------------------------------------------------------

    /// Verify that a basic variable declaration is tokenized correctly.
    #[test]
    fn test_tokenize_var_declaration() {
        let tokens = tokenize_javascript("var x = 42;", "").unwrap();
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
            let tokens = tokenize_javascript(&source, "").unwrap();
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
        let tokens = tokenize_javascript("a + b - c * d / e;", "").unwrap();
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
        let tokens = tokenize_javascript("a === b !== c;", "").unwrap();
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
        let tokens = tokenize_javascript("var s = \"hello world\";", "").unwrap();
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
        let tokens = tokenize_javascript("42;", "").unwrap();
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "42");
    }

    // -----------------------------------------------------------------------
    // Test 7: Delimiters
    // -----------------------------------------------------------------------

    /// All delimiter tokens should be recognized: ( ) { } [ ] ; , .
    #[test]
    fn test_delimiters() {
        let tokens = tokenize_javascript("(){}[];,", "").unwrap();
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
    // Test 8: Whitespace is skipped
    // -----------------------------------------------------------------------

    /// Whitespace between tokens should be consumed without producing tokens.
    #[test]
    fn test_whitespace_skipped() {
        let compact = tokenize_javascript("var x=1;", "").unwrap();
        let spaced = tokenize_javascript("var  x  =  1  ;", "").unwrap();

        let pairs_compact = token_pairs(&compact);
        let pairs_spaced = token_pairs(&spaced);

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
    }

    // -----------------------------------------------------------------------
    // Test 9: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_javascript_lexer` factory function should return a
    /// `GrammarLexer` that can successfully tokenize source code.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_javascript_lexer("42;", "").unwrap();
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 10: Function expression
    // -----------------------------------------------------------------------

    /// A function declaration exercises keywords, identifiers, parentheses,
    /// braces, and the return keyword.
    #[test]
    fn test_tokenize_function() {
        let tokens = tokenize_javascript("function add(a, b) { return a + b; }", "").unwrap();
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "function");
        assert_eq!(pairs[1].0, TokenType::Name);
        assert_eq!(pairs[1].1, "add");
    }

    // -----------------------------------------------------------------------
    // Test 11: Arrow function tokens
    // -----------------------------------------------------------------------

    /// The arrow operator => should be tokenized as a single token.
    #[test]
    fn test_arrow_operator() {
        let tokens = tokenize_javascript("(x) => x + 1;", "").unwrap();
        let pairs = token_pairs(&tokens);

        let has_arrow = pairs.iter().any(|(_, v)| *v == "=>");
        assert!(has_arrow, "Expected '=>' arrow token");
    }

    // -----------------------------------------------------------------------
    // Test 12: Versioned grammar — es2015
    // -----------------------------------------------------------------------

    /// The es2015 versioned grammar should tokenize a basic let declaration.
    #[test]
    fn test_versioned_es2015() {
        let tokens = tokenize_javascript("let x = 42;", "es2015").unwrap();
        let pairs = token_pairs(&tokens);

        assert!(pairs.len() >= 5, "Expected at least 5 tokens with es2015 grammar");
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "let");
    }

    // -----------------------------------------------------------------------
    // Test 13: All versioned grammars parse a number literal
    // -----------------------------------------------------------------------

    /// Every versioned ECMAScript grammar should successfully tokenize a
    /// simple number literal.
    #[test]
    fn test_all_versioned_grammars() {
        let versions = [
            "es1", "es3", "es5",
            "es2015", "es2016", "es2017", "es2018", "es2019",
            "es2020", "es2021", "es2022", "es2023", "es2024", "es2025",
        ];
        for v in &versions {
            let result = tokenize_javascript("42;", v);
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
        let result = tokenize_javascript("var x = 1;", "es99");
        assert!(result.is_err(), "Expected Err for unknown version 'es99'");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("es99"),
            "Error message should mention the bad version: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 15: create_javascript_lexer with unknown version returns Err
    // -----------------------------------------------------------------------

    /// The factory function should also return Err for unknown versions.
    #[test]
    fn test_create_lexer_unknown_version() {
        let result = create_javascript_lexer("var x = 1;", "bad-version");
        assert!(result.is_err(), "Expected Err from create_javascript_lexer with bad version");
    }
}
