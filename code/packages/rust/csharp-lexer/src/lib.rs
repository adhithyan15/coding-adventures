//! # C# Lexer — tokenizing C# source code.
//!
//! [C#](https://learn.microsoft.com/en-us/dotnet/csharp/) (pronounced
//! "C sharp") is a modern, multi-paradigm programming language developed by
//! Microsoft. It powers everything from Windows desktop apps to game
//! development with Unity, web services with ASP.NET, and cross-platform
//! mobile apps with Xamarin and MAUI.
//!
//! This crate provides a lexer (tokenizer) for C# source code. It does
//! **not** hand-write tokenization rules. Instead, it loads the appropriate
//! `.tokens` grammar file — a declarative description of every token in
//! C# — and feeds it to the generic [`GrammarLexer`] from the `lexer` crate.
//!
//! # Architecture
//!
//! The tokenization pipeline has three layers:
//!
//! ```text
//! csharp{version}.tokens  (grammar file on disk)
//!        |
//!        v
//! grammar-tools            (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer      (tokenizes source using TokenGrammar)
//! ```
//!
//! This crate is the thin glue layer that wires these components together
//! for C# specifically. It knows where to find the grammar files and
//! provides two public entry points:
//!
//! - [`create_csharp_lexer`] — returns a `GrammarLexer` for fine-grained
//!   control.
//! - [`tokenize_csharp`] — convenience function that returns `Vec<Token>`
//!   directly.
//!
//! # Version-Aware API
//!
//! Both entry points accept a `version` parameter that selects which grammar
//! file to use. C# uses a `major.minor` versioning scheme:
//!
//! | `version` | grammar file loaded |
//! |---|---|
//! | `"1.0"` | `grammars/csharp/csharp1.0.tokens` |
//! | `"2.0"` | `grammars/csharp/csharp2.0.tokens` |
//! | `"3.0"` | `grammars/csharp/csharp3.0.tokens` |
//! | `"4.0"` | `grammars/csharp/csharp4.0.tokens` |
//! | `"5.0"` | `grammars/csharp/csharp5.0.tokens` |
//! | `"6.0"` | `grammars/csharp/csharp6.0.tokens` |
//! | `"7.0"` | `grammars/csharp/csharp7.0.tokens` |
//! | `"8.0"` | `grammars/csharp/csharp8.0.tokens` |
//! | `"9.0"` | `grammars/csharp/csharp9.0.tokens` |
//! | `"10.0"` | `grammars/csharp/csharp10.0.tokens` |
//! | `"11.0"` | `grammars/csharp/csharp11.0.tokens` |
//! | `"12.0"` | `grammars/csharp/csharp12.0.tokens` (default) |
//!
//! An unknown version string returns `Err(String)`.
//!
//! # Keywords
//!
//! C# has a rich set of keywords including: `class`, `public`, `private`,
//! `protected`, `static`, `void`, `int`, `string`, `bool`, `if`, `else`,
//! `for`, `foreach`, `while`, `return`, `new`, `this`, `base`, `namespace`,
//! `using`, `interface`, `abstract`, `virtual`, `override`, `sealed`,
//! `readonly`, `const`, `var`, `async`, `await`, `null`, `true`, `false`.
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
///   grammars/            <-- returned by this function
///   packages/
///     rust/
///       csharp-lexer/
///         Cargo.toml     <-- env!("CARGO_MANIFEST_DIR")
/// ```
///
/// This relative navigation works regardless of where the repository is
/// checked out on disk, because `env!("CARGO_MANIFEST_DIR")` is baked in at
/// compile time using the absolute path at the time `cargo build` was run.
fn grammar_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("grammars")
}

/// Validate the C# version string and return the path to the corresponding
/// `.tokens` grammar file.
///
/// C# uses a `major.minor` versioning scheme. All 12 versions from 1.0
/// (released with the .NET Framework in 2002) through 12.0 (released with
/// .NET 8 in 2023) are supported.
///
/// # Version history highlights
///
/// - `"1.0"` (2002) — the original C# release: classes, interfaces, structs,
///   delegates, events, properties, indexers, boxing/unboxing.
/// - `"2.0"` (2005) — generics, iterators (`yield`), nullable types,
///   anonymous methods, partial classes.
/// - `"3.0"` (2007) — LINQ, lambda expressions, extension methods, anonymous
///   types, auto-implemented properties, `var` keyword.
/// - `"4.0"` (2010) — dynamic binding (`dynamic`), named/optional arguments,
///   covariance/contravariance.
/// - `"5.0"` (2012) — async/await, caller info attributes.
/// - `"6.0"` (2015) — expression-bodied members, string interpolation, null
///   conditional operators, `nameof`, `using static`.
/// - `"7.0"` (2017) — tuples, pattern matching, `out` variables, local
///   functions, binary literals, digit separators.
/// - `"8.0"` (2019) — nullable reference types, switch expressions, ranges,
///   indices, default interface methods, `using` declarations.
/// - `"9.0"` (2020) — records, init-only setters, top-level statements,
///   pattern matching enhancements, target-typed `new`.
/// - `"10.0"` (2021) — record structs, global `using`, file-scoped
///   namespaces, extended property patterns, `const` interpolated strings.
/// - `"11.0"` (2022) — raw string literals, generic math, list patterns,
///   required members, `scoped` ref.
/// - `"12.0"` (2023) — primary constructors for all classes, collection
///   expressions, `ref readonly` parameters, default lambda parameters.
///
/// Returns `Err(String)` for any unrecognised version string, so callers
/// can surface a clear error message rather than panicking on a missing file.
fn grammar_path(version: &str) -> Result<PathBuf, String> {
    let root = grammar_root();

    match version {
        // All 12 C# versions — each maps to its own .tokens grammar file.
        // The files live in grammars/csharp/ alongside the grammar files for
        // every other language supported by this repository.
        "1.0" | "2.0" | "3.0" | "4.0" | "5.0" | "6.0" |
        "7.0" | "8.0" | "9.0" | "10.0" | "11.0" | "12.0" => {
            Ok(root.join("csharp").join(format!("csharp{version}.tokens")))
        }

        // Anything else is an error — we'd rather fail loudly than silently
        // fall back to a default grammar and produce confusing results.
        other => Err(format!(
            "Unknown C# version '{other}'. \
             Valid values: \"1.0\", \"2.0\", \"3.0\", \"4.0\", \
             \"5.0\", \"6.0\", \"7.0\", \"8.0\", \
             \"9.0\", \"10.0\", \"11.0\", \"12.0\""
        )),
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for C# source code.
///
/// The `version` parameter selects which grammar file to load. Pass
/// `"12.0"` for the latest stable C# grammar, or an earlier version to
/// restrict the token set to that era.
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
/// use coding_adventures_csharp_lexer::create_csharp_lexer;
///
/// // C# 12.0 (latest):
/// let mut lexer = create_csharp_lexer("int x = 42;", "12.0").unwrap();
/// let tokens = lexer.tokenize().expect("tokenization failed");
///
/// // C# 8.0 grammar (nullable reference types era):
/// let mut lexer_8 = create_csharp_lexer("int x = 42;", "8.0").unwrap();
/// ```
pub fn create_csharp_lexer<'src>(
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
    //   - Skip patterns (whitespace, single-line comments //, block comments /* */)
    //   - Keywords (class, public, private, static, void, int, string, etc.)
    //   - Mode: default (no indentation tracking — C# uses braces, not indentation)
    let grammar = parse_token_grammar(&grammar_text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    // Create and return the lexer.
    Ok(GrammarLexer::new(source, &grammar))
}

/// Tokenize C# source code into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, and tokenization in one call. The returned vector always
/// ends with an `EOF` token.
///
/// The `version` parameter selects the C# edition:
/// - `"12.0"` — current, recommended for most use cases.
/// - `"8.0"` — for code targeting .NET Core 3.x / .NET Standard 2.1.
/// - `"7.0"` — for code targeting .NET Framework 4.7+ or .NET Core 2.x.
/// - earlier versions for legacy codebases.
///
/// # Errors
///
/// Returns `Err(String)` if the version is unknown, the grammar file is
/// missing or malformed, or the source contains an unrecognised character.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_csharp_lexer::tokenize_csharp;
///
/// // C# 12.0:
/// let tokens = tokenize_csharp(
///     "class Hello { public static void Main() { } }",
///     "12.0",
/// ).unwrap();
///
/// // C# 8.0:
/// let tokens_8 = tokenize_csharp("int x = 1;", "8.0").unwrap();
/// ```
pub fn tokenize_csharp(source: &str, version: &str) -> Result<Vec<Token>, String> {
    tokenize_csharp_impl(source, version)
}

/// Internal implementation that owns the grammar string for the duration
/// of tokenization, avoiding lifetime issues with GrammarLexer<'src>.
///
/// The public `tokenize_csharp` is a thin wrapper around this so that the
/// public API surface stays clean while the implementation handles lifetimes
/// correctly. (The `GrammarLexer` holds a reference into the grammar string,
/// so both must be alive at the same time — this function keeps them in the
/// same stack frame.)
fn tokenize_csharp_impl(source: &str, version: &str) -> Result<Vec<Token>, String> {
    let path = grammar_path(version)?;

    let grammar_text = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    let grammar = parse_token_grammar(&grammar_text)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    let mut lexer = GrammarLexer::new(source, &grammar);
    lexer
        .tokenize()
        .map_err(|e| format!("C# tokenization failed: {e}"))
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

    /// Collect (TokenType, value) pairs for all non-EOF tokens.
    ///
    /// This is the central helper for most tests — it strips the mandatory
    /// trailing EOF so assertions can focus on the actual content tokens.
    fn token_pairs(tokens: &[Token]) -> Vec<(TokenType, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: Simple class declaration (C# 12.0 grammar)
    // -----------------------------------------------------------------------

    /// Verify that a basic class declaration is tokenized correctly.
    ///
    /// In C#, every executable program must have at least one class. Even
    /// top-level programs (introduced in C# 9.0) are syntactic sugar that
    /// the compiler wraps in a hidden class. The class is the foundational
    /// building block of C# object-oriented programming.
    #[test]
    fn test_tokenize_class_declaration() {
        let tokens = tokenize_csharp("class Hello { }", "12.0").unwrap();
        let pairs = token_pairs(&tokens);

        // Expected: KEYWORD("class"), NAME("Hello"), LBRACE("{"), RBRACE("}")
        assert!(pairs.len() >= 4, "Expected at least 4 tokens, got {}", pairs.len());
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "class");
        assert_eq!(pairs[1].0, TokenType::Name);
        assert_eq!(pairs[1].1, "Hello");
    }

    // -----------------------------------------------------------------------
    // Test 2: C# keywords are recognized
    // -----------------------------------------------------------------------

    /// C# keywords should be classified as KEYWORD tokens, not NAME.
    ///
    /// C# distinguishes between keywords (reserved by the language) and
    /// contextual keywords (like `var`, `async`, `yield`, `get`, `set`
    /// that are only reserved in certain contexts). This test focuses on
    /// the always-reserved keywords.
    #[test]
    fn test_keywords() {
        let keywords = [
            "class", "public", "private", "protected",
            "static", "void", "int", "string", "bool",
            "if", "else", "for", "while", "return",
            "true", "false", "null", "new", "this",
            "namespace", "using", "interface", "abstract",
        ];

        for kw in &keywords {
            let source = format!("{kw};");
            let tokens = tokenize_csharp(&source, "12.0").unwrap();
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

    /// Arithmetic and assignment operators should be tokenized correctly.
    ///
    /// C# supports the standard arithmetic operators from C/Java, plus some
    /// C#-specific ones like `??` (null coalescing), `?.` (null conditional),
    /// and `=>` (lambda / expression body).
    #[test]
    fn test_operators() {
        let tokens = tokenize_csharp("a + b - c * d / e;", "12.0").unwrap();
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
    ///
    /// In C#, `==` is value equality for primitive types but can be
    /// overloaded for reference types. Unlike JavaScript, there is no `===`
    /// — reference equality is checked via `object.ReferenceEquals()`.
    #[test]
    fn test_multi_char_operators() {
        let tokens = tokenize_csharp("a == b != c;", "12.0").unwrap();
        let pairs = token_pairs(&tokens);

        let has_eq = pairs.iter().any(|(_, v)| *v == "==");
        let has_ne = pairs.iter().any(|(_, v)| *v == "!=");

        assert!(has_eq, "Expected '==' token");
        assert!(has_ne, "Expected '!=' token");
    }

    // -----------------------------------------------------------------------
    // Test 5: String literals
    // -----------------------------------------------------------------------

    /// C# supports double-quoted strings. Unlike Java or C, C# also has
    /// verbatim string literals (prefixed with `@`), raw string literals
    /// (triple-quoted, introduced in C# 11), and interpolated strings
    /// (prefixed with `$`). This test covers the basic double-quoted form.
    #[test]
    fn test_strings() {
        let tokens = tokenize_csharp("string s = \"hello world\";", "12.0").unwrap();
        let pairs = token_pairs(&tokens);

        let has_string = pairs.iter().any(|(t, _)| *t == TokenType::String);
        assert!(has_string, "Expected a STRING token");
    }

    // -----------------------------------------------------------------------
    // Test 6: Number literals
    // -----------------------------------------------------------------------

    /// C# supports integer literals, floating-point literals, and suffixes
    /// like `L` (long), `UL` (unsigned long), `F` (float), `D` (double),
    /// `M` (decimal). This test verifies the basic integer case.
    #[test]
    fn test_numbers() {
        let tokens = tokenize_csharp("42;", "12.0").unwrap();
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "42");
    }

    // -----------------------------------------------------------------------
    // Test 7: Delimiters
    // -----------------------------------------------------------------------

    /// All delimiter tokens should be recognized: ( ) { } [ ] ; , .
    ///
    /// C# uses the same brace-delimited block structure as Java and C.
    /// Square brackets are used for array types and attributes, and also
    /// for collection expressions (new in C# 12.0).
    #[test]
    fn test_delimiters() {
        let tokens = tokenize_csharp("(){}[];,", "12.0").unwrap();
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
    ///
    /// C# is a free-form language like Java and C — layout (indentation and
    /// spacing) has no syntactic significance. The lexer skips all whitespace
    /// characters: space, tab, carriage return, and newline.
    #[test]
    fn test_whitespace_skipped() {
        let compact = tokenize_csharp("int x=1;", "12.0").unwrap();
        let spaced = tokenize_csharp("int  x  =  1  ;", "12.0").unwrap();

        let pairs_compact = token_pairs(&compact);
        let pairs_spaced = token_pairs(&spaced);

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
    }

    // -----------------------------------------------------------------------
    // Test 9: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_csharp_lexer` factory function should return a
    /// `GrammarLexer` that can successfully tokenize source code.
    ///
    /// This tests the lower-level API that gives callers direct access to
    /// the lexer object (useful when you want to tokenize incrementally or
    /// inspect the lexer state between tokens).
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_csharp_lexer("42;", "12.0").unwrap();
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 10: Method declaration
    // -----------------------------------------------------------------------

    /// A method declaration exercises keywords, identifiers, parentheses,
    /// braces, and type names — the bread and butter of C#.
    ///
    /// C# entry points are traditionally `static void Main()` or
    /// `static void Main(string[] args)`. Since C# 9.0, top-level
    /// statements are also allowed, but the static Main pattern remains
    /// the most common form in larger programs.
    #[test]
    fn test_tokenize_method() {
        let tokens = tokenize_csharp(
            "public static void Main(string[] args) { }",
            "12.0",
        ).unwrap();
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "public");
        assert_eq!(pairs[1].0, TokenType::Keyword);
        assert_eq!(pairs[1].1, "static");
    }

    // -----------------------------------------------------------------------
    // Test 11: Versioned grammar — C# 8.0
    // -----------------------------------------------------------------------

    /// The C# 8.0 versioned grammar should tokenize a basic int declaration.
    ///
    /// C# 8.0 was a landmark release that introduced nullable reference types,
    /// switch expressions, ranges and indices, async streams, and default
    /// interface implementations. It requires .NET Core 3.x or later.
    #[test]
    fn test_versioned_csharp_8() {
        let tokens = tokenize_csharp("int x = 42;", "8.0").unwrap();
        let pairs = token_pairs(&tokens);

        assert!(pairs.len() >= 5, "Expected at least 5 tokens with C# 8.0 grammar");
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "int");
    }

    // -----------------------------------------------------------------------
    // Test 12: All 12 versioned grammars parse a number literal
    // -----------------------------------------------------------------------

    /// Every versioned C# grammar should successfully tokenize a simple
    /// number literal. This smoke test ensures all 12 grammar files are
    /// valid and loadable.
    ///
    /// The test covers the full version history from the original .NET
    /// Framework C# 1.0 (2002) through C# 12.0 (.NET 8, 2023).
    #[test]
    fn test_all_versioned_grammars() {
        let versions = [
            "1.0", "2.0", "3.0", "4.0", "5.0", "6.0",
            "7.0", "8.0", "9.0", "10.0", "11.0", "12.0",
        ];
        for v in &versions {
            let result = tokenize_csharp("42;", v);
            assert!(
                result.is_ok(),
                "Version '{v}' should parse successfully: {:?}",
                result.err()
            );

            let tokens = result.unwrap();
            let pairs = token_pairs(&tokens);
            assert_eq!(
                pairs[0].0, TokenType::Number,
                "First token for version '{v}' should be NUMBER"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 13: Unknown version returns Err
    // -----------------------------------------------------------------------

    /// Passing an unrecognised version string should return Err, not panic.
    ///
    /// This is important for tooling that dynamically selects the C# version
    /// based on a project file (`<LangVersion>` in .csproj). If the project
    /// specifies an unsupported version, the error message should include the
    /// bad value so users know exactly what to fix.
    #[test]
    fn test_unknown_version_returns_err() {
        let result = tokenize_csharp("int x = 1;", "99.0");
        assert!(result.is_err(), "Expected Err for unknown version '99.0'");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("99.0"),
            "Error message should mention the bad version: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 14: create_csharp_lexer with unknown version returns Err
    // -----------------------------------------------------------------------

    /// The factory function should also return Err for unknown versions.
    #[test]
    fn test_create_lexer_unknown_version() {
        let result = create_csharp_lexer("int x = 1;", "bad-version");
        assert!(
            result.is_err(),
            "Expected Err from create_csharp_lexer with bad version"
        );
    }

    // -----------------------------------------------------------------------
    // Test 15: Empty string version is invalid
    // -----------------------------------------------------------------------

    /// C# always requires a specific version string — empty string is invalid.
    ///
    /// Unlike some tooling that might interpret `""` as "use the latest",
    /// this crate requires an explicit version. This prevents subtle bugs
    /// where a missing version configuration silently uses the wrong grammar.
    #[test]
    fn test_empty_version_returns_err() {
        let result = tokenize_csharp("int x = 1;", "");
        assert!(result.is_err(), "Expected Err for empty version string");
    }

    // -----------------------------------------------------------------------
    // Test 16: Namespace declaration
    // -----------------------------------------------------------------------

    /// A namespace declaration exercises the `namespace` keyword, which is
    /// fundamental to C# code organization.
    ///
    /// Namespaces in C# are similar to Java packages — they organize types
    /// into logical groups and prevent naming conflicts. The `System` and
    /// `System.Collections.Generic` namespaces from the .NET BCL are used
    /// by virtually every C# program.
    #[test]
    fn test_namespace_declaration() {
        let tokens = tokenize_csharp("namespace MyApp { }", "12.0").unwrap();
        let pairs = token_pairs(&tokens);

        assert!(pairs.len() >= 4, "Expected at least 4 tokens");
        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "namespace");
    }

    // -----------------------------------------------------------------------
    // Test 17: Using directive
    // -----------------------------------------------------------------------

    /// The `using` directive should be recognized as a keyword.
    ///
    /// In C#, `using` serves double duty:
    /// 1. As a namespace import directive: `using System.Collections.Generic;`
    /// 2. As a resource management statement: `using (var r = new Resource())`
    /// 3. As a global import (C# 10.0): `global using System;`
    #[test]
    fn test_using_directive() {
        let tokens = tokenize_csharp("using System;", "12.0").unwrap();
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs[0].0, TokenType::Keyword);
        assert_eq!(pairs[0].1, "using");
    }
}
