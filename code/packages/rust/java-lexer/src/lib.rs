//! # Java Lexer — tokenizing Java source code.
//!
//! [Java](https://docs.oracle.com/javase/specs/) is one of the most widely
//! used programming languages in the world, powering everything from Android
//! apps to enterprise backends. This crate provides a lexer (tokenizer) for
//! a subset of Java.
//!
//! It does **not** hand-write tokenization rules. Instead, it loads the
//! appropriate `.tokens` grammar file — a declarative description of every
//! token in Java — and feeds it to the generic [`GrammarLexer`] from the
//! `lexer` crate.
//!
//! # Architecture
//!
//! The tokenization pipeline has three layers:
//!
//! ```text
//! java{version}.tokens  (grammar file on disk)
//!        |
//!        v
//! grammar-tools         (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer   (tokenizes source using TokenGrammar)
//! ```
//!
//! This crate is the thin glue layer that wires these components together
//! for Java specifically. It knows where to find the grammar files and
//! provides two public entry points:
//!
//! - [`create_java_lexer`] — returns a `GrammarLexer` for fine-grained control.
//! - [`tokenize_java`] — convenience function that returns `Vec<Token>` directly.
//!
//! # Version-Aware API
//!
//! Both entry points accept a `version` parameter that selects which grammar
//! file to use. Java uses a numeric versioning scheme:
//!
//! | `version` | grammar file loaded |
//! |---|---|
//! | `"1.0"` | `grammars/java/java1.0.tokens` |
//! | `"1.1"` | `grammars/java/java1.1.tokens` |
//! | `"1.4"` | `grammars/java/java1.4.tokens` |
//! | `"5"` | `grammars/java/java5.tokens` |
//! | `"7"` | `grammars/java/java7.tokens` |
//! | `"8"` | `grammars/java/java8.tokens` |
//! | `"10"` | `grammars/java/java10.tokens` |
//! | `"14"` | `grammars/java/java14.tokens` |
//! | `"17"` | `grammars/java/java17.tokens` |
//! | `"21"` | `grammars/java/java21.tokens` (default) |
//!
//! An unknown version string returns `Err(String)`.
//!
//! # Keywords
//!
//! Java has a rich set of keywords: `class`, `public`, `private`, `static`,
//! `void`, `int`, `boolean`, `if`, `else`, `for`, `while`, `return`, `new`,
//! `this`, `extends`, `implements`, `interface`, `abstract`, `final`, etc.
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
///       java-lexer/
///         Cargo.toml    <-- env!("CARGO_MANIFEST_DIR")
/// ```
fn grammar_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("grammars")
}

/// Validate the Java version string and return the path to the corresponding
/// `.tokens` grammar file.
///
/// Valid version strings are:
/// - `"1.0"`, `"1.1"`, `"1.4"` — early Java releases (JDK 1.x era)
/// - `"5"`, `"7"`, `"8"` — the J2SE/Java SE era
/// - `"10"`, `"14"`, `"17"`, `"21"` — modern Java (post-modules, records, etc.)
///
/// Returns `Err(String)` for any unrecognised version string, so callers
/// can surface a clear error message rather than panicking on a missing file.
fn grammar_path(version: &str) -> Result<PathBuf, String> {
    let root = grammar_root();

    match version {
        // Early Java releases used the "1.x" naming convention.
        // Java 1.0 (1996) — the original release with applets and AWT.
        // Java 1.1 (1997) — inner classes, JDBC, RMI.
        // Java 1.4 (2002) — assertions, NIO, regex, logging.
        "1.0" | "1.1" | "1.4" => {
            Ok(root.join("java").join(format!("java{version}.tokens")))
        }

        // Starting with Java 5 (2004), Sun dropped the "1." prefix.
        // Java 5 — generics, enums, annotations, autoboxing, varargs.
        // Java 7 — diamond operator, try-with-resources, multi-catch.
        // Java 8 — lambdas, streams, default methods, Optional.
        // Java 10 — local variable type inference (var).
        // Java 14 — records (preview), switch expressions.
        // Java 17 — sealed classes, pattern matching for instanceof.
        // Java 21 — virtual threads, record patterns, pattern matching for switch.
        "5" | "7" | "8" | "10" | "14" | "17" | "21" => {
            Ok(root.join("java").join(format!("java{version}.tokens")))
        }

        // Anything else is an error — we'd rather fail loudly than silently
        // fall back to a default grammar and produce confusing results.
        other => Err(format!(
            "Unknown Java version '{other}'. \
             Valid values: \"1.0\", \"1.1\", \"1.4\", \
             \"5\", \"7\", \"8\", \"10\", \"14\", \"17\", \"21\""
        )),
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for Java source code.
///
/// The `version` parameter selects which grammar file to load:
/// - `"1.0"`, `"1.1"`, `"1.4"` — early JDK 1.x era grammars.
/// - `"5"`, `"7"`, `"8"`, `"10"`, `"14"`, `"17"`, `"21"` — modern Java
///   grammars. `"21"` is recommended for most use cases.
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
/// use coding_adventures_java_lexer::create_java_lexer;
///
/// // Java 21 (latest LTS):
/// let mut lexer = create_java_lexer("int x = 42;", "21").unwrap();
/// let tokens = lexer.tokenize().expect("tokenization failed");
///
/// // Java 8 grammar:
/// let mut lexer_8 = create_java_lexer("int x = 42;", "8").unwrap();
/// ```
pub fn create_java_lexer<'src>(
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
    //   - Keywords (class, public, static, void, int, if, else, return, etc.)
    //   - Mode: default (no indentation tracking)
    let grammar = parse_token_grammar(&grammar_text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    // Create and return the lexer.
    Ok(GrammarLexer::new(source, &grammar))
}

/// Tokenize Java source code into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, and tokenization in one call. The returned vector always
/// ends with an `EOF` token.
///
/// The `version` parameter is the same as for [`create_java_lexer`]:
/// pass a version like `"21"` for the latest LTS, or `"8"` for Java 8.
///
/// # Errors
///
/// Returns `Err(String)` if the version is unknown, the grammar file is
/// missing or malformed, or the source contains an unrecognised character.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_java_lexer::tokenize_java;
///
/// // Java 21:
/// let tokens = tokenize_java(
///     "class Hello { public static void main(String[] args) { } }",
///     "21",
/// ).unwrap();
///
/// // Java 8:
/// let tokens_8 = tokenize_java("int x = 1;", "8").unwrap();
/// ```
pub fn tokenize_java(source: &str, version: &str) -> Result<Vec<Token>, String> {
    tokenize_java_impl(source, version)
}

/// Internal implementation that owns the grammar string for the duration
/// of tokenization, avoiding lifetime issues with GrammarLexer<'src>.
fn tokenize_java_impl(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let path = grammar_path(version)?;

    let grammar_text = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    let grammar = parse_token_grammar(&grammar_text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer
        .tokenize()
        .map_err(|e| format!("Java tokenization failed: {e}"))
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
    // Test 1: Simple class declaration (Java 21 grammar)
    // -----------------------------------------------------------------------

    /// Verify that a basic class declaration is tokenized correctly.
    /// Java's most fundamental construct is the class — every program must
    /// contain at least one class definition.
    #[test]
    fn test_tokenize_class_declaration() {
        let tokens = tokenize_java("class Hello { }", "21").unwrap();
        let pairs = token_pairs(&tokens);

        // Expected: KEYWORD("class"), NAME("Hello"), LBRACE("{"), RBRACE("}")
        assert!(pairs.len() >= 4, "Expected at least 4 tokens, got {}", pairs.len());
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "class");
        assert_eq!(pairs[1].0, TokenType::Name);
        assert_eq!(pairs[1].1, "Hello");
    }

    // -----------------------------------------------------------------------
    // Test 2: Keywords are recognized
    // -----------------------------------------------------------------------

    /// Java keywords should be classified as KEYWORD tokens, not NAME.
    /// Java has more reserved words than most languages — including type
    /// keywords like `int`, `boolean`, `void` that other languages treat
    /// as built-in types rather than keywords.
    #[test]
    fn test_keywords() {
        let keywords = ["class", "public", "static", "void", "int",
                        "if", "else", "for", "while", "return",
                        "true", "false", "null", "new", "this"];

        for kw in &keywords {
            let source = format!("{kw};");
            let tokens = tokenize_java(&source, "21").unwrap();
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
        let tokens = tokenize_java("a + b - c * d / e;", "21").unwrap();
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

    /// Multi-character operators like ==, !=, >=, <= should be tokenized
    /// as single tokens, not split into individual characters.
    /// Java uses == for equality (not === like JavaScript).
    #[test]
    fn test_multi_char_operators() {
        let tokens = tokenize_java("a == b != c;", "21").unwrap();
        let pairs = token_pairs(&tokens);

        let has_eq = pairs.iter().any(|(_, v)| *v == "==");
        let has_ne = pairs.iter().any(|(_, v)| *v == "!=");

        assert!(has_eq, "Expected '==' token");
        assert!(has_ne, "Expected '!=' token");
    }

    // -----------------------------------------------------------------------
    // Test 5: String literals
    // -----------------------------------------------------------------------

    /// Java supports double-quoted strings.
    #[test]
    fn test_strings() {
        let tokens = tokenize_java("String s = \"hello world\";", "21").unwrap();
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token");
    }

    // -----------------------------------------------------------------------
    // Test 6: Number literals
    // -----------------------------------------------------------------------

    /// Java supports integer and floating-point numbers.
    #[test]
    fn test_numbers() {
        let tokens = tokenize_java("42;", "21").unwrap();
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
        let tokens = tokenize_java("(){}[];,", "21").unwrap();
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
        let compact = tokenize_java("int x=1;", "21").unwrap();
        let spaced = tokenize_java("int  x  =  1  ;", "21").unwrap();

        let pairs_compact = token_pairs(&compact);
        let pairs_spaced = token_pairs(&spaced);

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
    }

    // -----------------------------------------------------------------------
    // Test 9: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_java_lexer` factory function should return a
    /// `GrammarLexer` that can successfully tokenize source code.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_java_lexer("42;", "21").unwrap();
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 10: Method declaration
    // -----------------------------------------------------------------------

    /// A method declaration exercises keywords, identifiers, parentheses,
    /// braces, and the return keyword — the bread and butter of Java.
    #[test]
    fn test_tokenize_method() {
        let tokens = tokenize_java("public static void main(String[] args) { }", "21").unwrap();
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "public");
        assert_eq!(pairs[1].0, TokenType::Keyword);
        assert_eq!(pairs[1].1, "static");
    }

    // -----------------------------------------------------------------------
    // Test 11: Versioned grammar — Java 8
    // -----------------------------------------------------------------------

    /// The Java 8 versioned grammar should tokenize a basic int declaration.
    /// Java 8 was a landmark release that introduced lambdas and streams.
    #[test]
    fn test_versioned_java8() {
        let tokens = tokenize_java("int x = 42;", "8").unwrap();
        let pairs = token_pairs(&tokens);

        assert!(pairs.len() >= 5, "Expected at least 5 tokens with Java 8 grammar");
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "int");
    }

    // -----------------------------------------------------------------------
    // Test 12: All versioned grammars parse a number literal
    // -----------------------------------------------------------------------

    /// Every versioned Java grammar should successfully tokenize a simple
    /// number literal. This smoke test ensures all grammar files are valid
    /// and loadable.
    #[test]
    fn test_all_versioned_grammars() {
        let versions = [
            "1.0", "1.1", "1.4",
            "5", "7", "8", "10", "14", "17", "21",
        ];
        for v in &versions {
            let result = tokenize_java("42;", v);
            assert!(result.is_ok(), "Version '{v}' should parse successfully: {:?}", result.err());

            let tokens = result.unwrap();
            let pairs = token_pairs(&tokens);
            assert_eq!(pairs[0].0, TokenType::Number, "First token for '{v}' should be NUMBER");
        }
    }

    // -----------------------------------------------------------------------
    // Test 13: Unknown version returns Err
    // -----------------------------------------------------------------------

    /// Passing an unrecognised version string should return Err, not panic.
    #[test]
    fn test_unknown_version_returns_err() {
        let result = tokenize_java("int x = 1;", "99");
        assert!(result.is_err(), "Expected Err for unknown version '99'");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("99"),
            "Error message should mention the bad version: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 14: create_java_lexer with unknown version returns Err
    // -----------------------------------------------------------------------

    /// The factory function should also return Err for unknown versions.
    #[test]
    fn test_create_lexer_unknown_version() {
        let result = create_java_lexer("int x = 1;", "bad-version");
        assert!(result.is_err(), "Expected Err from create_java_lexer with bad version");
    }

    // -----------------------------------------------------------------------
    // Test 15: Empty string version is invalid
    // -----------------------------------------------------------------------

    /// Unlike JavaScript which uses "" for the generic grammar, Java always
    /// requires a specific version string. Empty string should return Err.
    #[test]
    fn test_empty_version_returns_err() {
        let result = tokenize_java("int x = 1;", "");
        assert!(result.is_err(), "Expected Err for empty version string");
    }
}
