//! # Python Lexer — tokenizing Python source code across versions.
//!
//! [Python](https://www.python.org/) is one of the most widely-used
//! programming languages in the world, with a rich history spanning
//! versions 2.7 through 3.12+ and beyond. Each version introduced
//! new syntax: f-strings in 3.6, the walrus operator in 3.8,
//! structural pattern matching in 3.10, and type parameter syntax
//! in 3.12.
//!
//! This crate provides a lexer (tokenizer) for Python that supports
//! multiple language versions. It does **not** hand-write tokenization
//! rules. Instead, it loads a versioned `python{version}.tokens` grammar
//! file — a declarative description of every token in that version — and
//! feeds it to the generic [`GrammarLexer`] from the `lexer` crate.
//!
//! # Architecture
//!
//! The tokenization pipeline has three layers:
//!
//! ```text
//! python{version}.tokens   (grammar file on disk)
//!        |
//!        v
//! grammar-tools            (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer      (tokenizes source using TokenGrammar)
//! ```
//!
//! This crate is the thin glue layer that wires these components together
//! for Python specifically. It knows where to find the versioned grammar
//! files and provides two public entry points:
//!
//! - [`create_python_lexer`] — returns a `GrammarLexer` for fine-grained control.
//! - [`tokenize_python`] — convenience function that returns `Vec<Token>` directly.
//!
//! # Supported versions
//!
//! | Version | Grammar file           | Key additions                              |
//! |---------|------------------------|--------------------------------------------|
//! | `"2.7"` | `python2.7.tokens`     | Classic Python 2 syntax                    |
//! | `"3.0"` | `python3.0.tokens`     | Python 3 baseline (print as function, etc.)|
//! | `"3.6"` | `python3.6.tokens`     | f-strings, variable annotations            |
//! | `"3.8"` | `python3.8.tokens`     | Walrus operator `:=`, positional-only `/`  |
//! | `"3.10"`| `python3.10.tokens`    | match/case soft keywords (PEP 634)         |
//! | `"3.12"`| `python3.12.tokens`    | type soft keyword (PEP 695), f-string rework|
//!
//! The default version is `"3.12"`.
//!
//! # Why grammar-driven instead of hand-written?
//!
//! A hand-written lexer for Python would be well over 1000 lines of Rust,
//! with version-specific branches for each syntax change. The grammar-driven
//! approach replaces all that with declarative grammar files plus ~60 lines
//! of Rust glue code. When the language evolves (e.g., Python 3.13 adds new
//! syntax), you add a new grammar file — no Rust code changes needed.
//!
//! The tradeoff is performance: regex matching is slower than hand-tuned
//! character loops. For source files of typical size (<50,000 lines), this
//! difference is negligible.
//!
//! # Indentation mode
//!
//! Python uses significant indentation. The grammar files declare
//! `mode: indentation`, which tells the `GrammarLexer` to:
//!
//! 1. Track an indentation stack (starts at `[0]`).
//! 2. Emit `INDENT` tokens when indentation increases.
//! 3. Emit `DEDENT` tokens when indentation decreases.
//! 4. Emit `NEWLINE` tokens at logical line boundaries.
//! 5. Suppress `INDENT`/`DEDENT`/`NEWLINE` inside brackets `()`, `[]`, `{}`.
//! 6. Reject tab characters in leading whitespace.
//!
//! # Soft keywords (3.10+)
//!
//! Python 3.10 introduced `match`, `case`, and `_` as soft keywords —
//! words that are keywords only inside match statements and remain valid
//! identifiers everywhere else. Python 3.12 added `type` as a soft
//! keyword. The grammar files declare these in a `soft_keywords:` section.
//! The lexer emits plain `NAME` tokens for them; the parser is responsible
//! for disambiguating based on syntactic position.

use std::fs;

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

// ===========================================================================
// Supported versions
// ===========================================================================

/// The set of Python versions this lexer supports.
///
/// Each version corresponds to a `python{version}.tokens` grammar file
/// in the `code/grammars/python/` directory. The grammar file declares
/// the complete lexical grammar for that version of Python.
pub const SUPPORTED_VERSIONS: &[&str] = &["2.7", "3.0", "3.6", "3.8", "3.10", "3.12"];

/// The default Python version used when the caller does not specify one.
///
/// We default to the latest stable version (3.12) because most Python
/// code being written today targets 3.10+ and the 3.12 grammar is a
/// superset of earlier 3.x grammars for the purposes of tokenization.
pub const DEFAULT_VERSION: &str = "3.12";

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the versioned `python{version}.tokens` grammar file.
///
/// We use `env!("CARGO_MANIFEST_DIR")` to get the directory containing this
/// crate's `Cargo.toml` at compile time. From there, we navigate up to the
/// `grammars/python/` directory at the repository root.
///
/// The directory structure looks like:
///
/// ```text
/// code/
///   grammars/
///     python/
///       python2.7.tokens     <-- version "2.7"
///       python3.0.tokens     <-- version "3.0"
///       python3.6.tokens     <-- version "3.6"
///       python3.8.tokens     <-- version "3.8"
///       python3.10.tokens    <-- version "3.10"
///       python3.12.tokens    <-- version "3.12"
///   packages/
///     rust/
///       python-lexer/
///         Cargo.toml          <-- CARGO_MANIFEST_DIR points here
///         src/
///           lib.rs            <-- we are here
/// ```
///
/// So the relative path from CARGO_MANIFEST_DIR to a grammar file is:
/// `../../../grammars/python/python{version}.tokens`
fn grammar_path(version: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/python/python{version}.tokens")
}

/// Validate that the given version string is supported.
///
/// Returns the version string unchanged if valid, or panics with a
/// descriptive error message listing all supported versions.
fn validate_version(version: &str) -> &str {
    if SUPPORTED_VERSIONS.contains(&version) {
        version
    } else {
        panic!(
            "Unsupported Python version: '{}'. Supported versions: {}",
            version,
            SUPPORTED_VERSIONS.join(", ")
        );
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for Python source code.
///
/// This function:
/// 1. Validates the requested Python version.
/// 2. Reads the corresponding `python{version}.tokens` grammar file.
/// 3. Parses it into a `TokenGrammar` using `grammar-tools`.
/// 4. Constructs a `GrammarLexer` with the grammar and the given source.
///
/// The returned lexer is ready to call `.tokenize()` on. Use this when you
/// need access to the lexer object itself (e.g., for incremental tokenization
/// or custom error handling).
///
/// # Arguments
///
/// - `source` — The Python source code to tokenize.
/// - `version` — The Python version to use (e.g., `"3.12"`, `"2.7"`).
///   Use `"3.12"` for the latest.
///
/// # Panics
///
/// Panics if:
/// - The version is not one of the supported versions.
/// - The grammar file cannot be read or parsed.
///
/// These should never happen in practice — grammar files are checked into
/// the repository and validated by the grammar-tools test suite. A panic
/// here indicates a broken build or missing file.
///
/// # Example
///
/// ```no_run
/// use coding_adventures_python_lexer::create_python_lexer;
///
/// let mut lexer = create_python_lexer("x = 1 + 2\n", "3.12");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{}", token);
/// }
/// ```
pub fn create_python_lexer<'a>(source: &'a str, version: &str) -> GrammarLexer<'a> {
    // Step 1: Validate the version string against the supported set.
    let version = validate_version(version);

    // Step 2: Read the grammar file from disk.
    //
    // We read the file at runtime (not compile time) because the grammar file
    // may be updated independently of this crate. This also avoids bloating
    // the binary with embedded grammar text.
    let path = grammar_path(version);
    let grammar_text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read python{version}.tokens at {path}: {e}"));

    // Step 3: Parse the grammar text into a structured TokenGrammar.
    //
    // The TokenGrammar contains:
    //   - Token definitions (patterns, names, aliases)
    //   - Skip patterns (whitespace, comments)
    //   - Keywords (def, if, else, class, etc.)
    //   - Reserved keywords (version-specific)
    //   - Soft keywords (match, case, type — 3.10+)
    //   - Mode (indentation)
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse python{version}.tokens: {e}"));

    // Step 4: Create and return the lexer.
    //
    // The GrammarLexer compiles all token patterns into anchored regexes
    // and is ready to tokenize the source string.
    GrammarLexer::new(source, &grammar)
}

/// Tokenize Python source code into a vector of tokens.
///
/// This is the most convenient entry point — it handles grammar loading,
/// lexer creation, and tokenization in one call. The returned vector always
/// ends with an `EOF` token.
///
/// # Arguments
///
/// - `source` — The Python source code to tokenize.
/// - `version` — The Python version to use (e.g., `"3.12"`, `"2.7"`).
///
/// # Panics
///
/// Panics if the grammar file cannot be read/parsed, or if the source
/// contains an unexpected character (via `LexerError` propagation).
///
/// # Example
///
/// ```no_run
/// use coding_adventures_python_lexer::tokenize_python;
///
/// let tokens = tokenize_python("def greet(name):\n    return name\n", "3.12");
/// for token in &tokens {
///     println!("{:?} {:?}", token.type_, token.value);
/// }
/// ```
pub fn tokenize_python(source: &str, version: &str) -> Vec<Token> {
    // Create a fresh lexer for this source text and version.
    let mut python_lexer = create_python_lexer(source, version);

    // Tokenize and unwrap — any LexerError becomes a panic.
    //
    // In a production compiler, you would want to propagate the error
    // via Result. For this educational codebase, panicking with a clear
    // message is sufficient and keeps the API simple.
    python_lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("Python tokenization failed: {e}"))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper: collect token types (excluding EOF) for easier assertions.
    // -----------------------------------------------------------------------

    /// Extract the (type, value) pairs from a token stream, excluding
    /// the final EOF token. This makes test assertions more concise.
    fn token_pairs(tokens: &[Token]) -> Vec<(TokenType, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: Simple arithmetic expression
    // -----------------------------------------------------------------------

    /// Verify that a basic assignment with arithmetic operators is tokenized
    /// correctly. This tests NAME, EQUALS, INT, and PLUS tokens.
    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize_python("x = 1 + 2\n", "3.12");
        let pairs = token_pairs(&tokens);

        // Expected tokens (in indentation mode, the lexer emits NEWLINE
        // at the end of each logical line):
        //   NAME("x"), EQUALS("="), INT("1"), PLUS("+"), INT("2"), NEWLINE
        assert_eq!(pairs.len(), 6);
        assert_eq!(pairs[0].0, TokenType::Name);
        assert_eq!(pairs[0].1, "x");
        assert_eq!(pairs[1].0, TokenType::Equals);
        assert_eq!(pairs[3].0, TokenType::Plus);
    }

    // -----------------------------------------------------------------------
    // Test 2: Keywords are recognized
    // -----------------------------------------------------------------------

    /// Python keywords (def, return, if, else, for, in, etc.) should be
    /// classified as KEYWORD tokens, not NAME tokens.
    #[test]
    fn test_keywords() {
        let keywords = ["def", "return", "if", "else", "for", "in", "and",
                        "or", "not", "pass", "break", "continue", "lambda",
                        "class", "import", "from", "as", "True", "False", "None"];

        for kw in &keywords {
            let source = format!("{kw}\n");
            let tokens = tokenize_python(&source, "3.12");

            assert_eq!(
                tokens[0].type_, TokenType::Keyword,
                "Expected '{}' to be a KEYWORD, got {:?}",
                kw, tokens[0].type_
            );
            assert_eq!(tokens[0].value, *kw);
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: Indentation produces INDENT/DEDENT tokens
    // -----------------------------------------------------------------------

    /// In indentation mode, the lexer tracks leading whitespace and emits
    /// INDENT when indentation increases and DEDENT when it decreases.
    #[test]
    fn test_indentation() {
        let source = "def f():\n    return 1\n";
        let tokens = tokenize_python(source, "3.12");

        let has_indent = tokens.iter().any(|t| t.type_ == TokenType::Indent);
        let has_dedent = tokens.iter().any(|t| t.type_ == TokenType::Dedent);
        let has_newline = tokens.iter().any(|t| t.type_ == TokenType::Newline);

        assert!(has_indent, "Expected INDENT token in output");
        assert!(has_dedent, "Expected DEDENT token in output");
        assert!(has_newline, "Expected NEWLINE token in output");

        let indent_count = tokens.iter().filter(|t| t.type_ == TokenType::Indent).count();
        let dedent_count = tokens.iter().filter(|t| t.type_ == TokenType::Dedent).count();
        assert_eq!(indent_count, 1, "Expected exactly 1 INDENT");
        assert_eq!(dedent_count, 1, "Expected exactly 1 DEDENT");
    }

    // -----------------------------------------------------------------------
    // Test 4: Brackets suppress NEWLINE/INDENT/DEDENT
    // -----------------------------------------------------------------------

    /// When inside brackets, the lexer suppresses NEWLINE, INDENT, and
    /// DEDENT tokens. This allows multi-line function calls and list literals.
    #[test]
    fn test_bracket_suppression() {
        let source = "f(\n    x,\n    y\n)\n";
        let tokens = tokenize_python(source, "3.12");

        let indent_count = tokens.iter().filter(|t| t.type_ == TokenType::Indent).count();
        assert_eq!(indent_count, 0, "Expected no INDENT inside brackets");

        let newline_count = tokens.iter().filter(|t| t.type_ == TokenType::Newline).count();
        assert_eq!(newline_count, 1, "Expected exactly 1 NEWLINE (after closing paren)");
    }

    // -----------------------------------------------------------------------
    // Test 5: Multi-character operators
    // -----------------------------------------------------------------------

    /// Python has several two-character operators that must be tokenized
    /// as single tokens, not split into two single-character tokens.
    #[test]
    fn test_operators() {
        let source = "a ** b // c == d != e <= f >= g\n";
        let tokens = tokenize_python(source, "3.12");
        let pairs = token_pairs(&tokens);

        let ops: Vec<&str> = pairs.iter()
            .filter(|(_, v)| {
                !v.chars().all(|c| c.is_alphabetic() || c == '_') && *v != "\\n"
            })
            .map(|(_, v)| *v)
            .collect();

        assert_eq!(ops, vec!["**", "//", "==", "!=", "<=", ">="]);
    }

    // -----------------------------------------------------------------------
    // Test 6: String literals
    // -----------------------------------------------------------------------

    /// Python supports double-quoted string literals. The lexer should
    /// strip the quotes and process escape sequences.
    #[test]
    fn test_strings() {
        let source = "x = \"hello world\"\n";
        let tokens = tokenize_python(source, "3.12");

        let string_token = tokens.iter().find(|t| t.type_ == TokenType::String
            || t.type_name.as_deref() == Some("STRING"));

        assert!(string_token.is_some(), "Expected a STRING token");
        let st = string_token.unwrap();
        assert_eq!(st.value, "hello world");
    }

    // -----------------------------------------------------------------------
    // Test 7: Comments are skipped
    // -----------------------------------------------------------------------

    /// Comments in Python start with `#` and run to the end of the line.
    /// They should be consumed by the lexer without producing tokens.
    #[test]
    fn test_comments_skipped() {
        let source = "x = 1  # this is a comment\n";
        let tokens = tokenize_python(source, "3.12");

        let has_comment = tokens.iter().any(|t| t.value.contains("comment") || t.value.contains("#"));
        assert!(!has_comment, "Comments should not produce tokens");

        // Verify the token count: x, =, 1, NEWLINE, EOF = 5 tokens.
        assert_eq!(tokens.len(), 5);
    }

    // -----------------------------------------------------------------------
    // Test 8: Float literals
    // -----------------------------------------------------------------------

    /// Python supports floating-point literals like `3.14`, `1e10`, `.5`,
    /// and `1.5e-3`. These should be tokenized as FLOAT tokens.
    #[test]
    fn test_float_literals() {
        let source = "3.14\n";
        let tokens = tokenize_python(source, "3.12");

        let floats: Vec<&Token> = tokens.iter()
            .filter(|t| t.type_name.as_deref() == Some("FLOAT"))
            .collect();

        assert_eq!(floats.len(), 1, "Expected 1 FLOAT token");
        assert_eq!(floats[0].value, "3.14");

        let source2 = "1e10\n";
        let tokens2 = tokenize_python(source2, "3.12");

        let floats2: Vec<&Token> = tokens2.iter()
            .filter(|t| t.type_name.as_deref() == Some("FLOAT"))
            .collect();

        assert_eq!(floats2.len(), 1, "Expected 1 FLOAT token for scientific notation");
        assert_eq!(floats2[0].value, "1e10");
    }

    // -----------------------------------------------------------------------
    // Test 9: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_python_lexer` factory function should return a
    /// `GrammarLexer` that can successfully tokenize source code.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_python_lexer("42\n", "3.12");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        assert!(tokens.len() >= 3);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 10: Single-quoted strings
    // -----------------------------------------------------------------------

    /// Python supports both single-quoted and double-quoted strings.
    #[test]
    fn test_single_quoted_strings() {
        let source = "x = 'hello'\n";
        let tokens = tokenize_python(source, "3.12");

        let string_token = tokens.iter().find(|t| t.type_ == TokenType::String
            || t.type_name.as_deref() == Some("STRING"));

        assert!(string_token.is_some(), "Expected a STRING token for single-quoted string");
        assert_eq!(string_token.unwrap().value, "hello");
    }

    // -----------------------------------------------------------------------
    // Test 11: Augmented assignment operators
    // -----------------------------------------------------------------------

    /// Python supports augmented assignment operators like `+=`, `-=`, `*=`.
    #[test]
    fn test_augmented_assignment_operators() {
        let source = "x += 1\n";
        let tokens = tokenize_python(source, "3.12");

        let plus_eq = tokens.iter().find(|t| t.value == "+=");
        assert!(plus_eq.is_some(), "Expected '+=' token");
    }

    // -----------------------------------------------------------------------
    // Test 12: Delimiters
    // -----------------------------------------------------------------------

    /// All delimiter tokens should be recognized.
    #[test]
    fn test_delimiters() {
        let source = "()[]{},:.;\n";
        let tokens = tokenize_python(source, "3.12");

        let values: Vec<&str> = tokens.iter()
            .filter(|t| t.type_ != TokenType::Eof && t.type_ != TokenType::Newline)
            .map(|t| t.value.as_str())
            .collect();

        assert!(values.contains(&"("));
        assert!(values.contains(&")"));
        assert!(values.contains(&"["));
        assert!(values.contains(&"]"));
        assert!(values.contains(&"{"));
        assert!(values.contains(&"}"));
        assert!(values.contains(&","));
        assert!(values.contains(&":"));
        assert!(values.contains(&"."));
        assert!(values.contains(&";"));
    }

    // -----------------------------------------------------------------------
    // Test 13: Nested indentation
    // -----------------------------------------------------------------------

    /// Multiple levels of indentation should produce multiple INDENT tokens
    /// on the way in and multiple DEDENT tokens on the way out.
    #[test]
    fn test_nested_indentation() {
        let source = "if True:\n    if True:\n        x = 1\n";
        let tokens = tokenize_python(source, "3.12");

        let indent_count = tokens.iter().filter(|t| t.type_ == TokenType::Indent).count();
        let dedent_count = tokens.iter().filter(|t| t.type_ == TokenType::Dedent).count();

        assert_eq!(indent_count, 2, "Expected 2 INDENT tokens for nested blocks");
        assert_eq!(dedent_count, 2, "Expected 2 DEDENT tokens for nested blocks");
    }

    // -----------------------------------------------------------------------
    // Test 14: Version selection
    // -----------------------------------------------------------------------

    /// Each supported version should successfully load and tokenize.
    #[test]
    fn test_all_versions_load() {
        for version in SUPPORTED_VERSIONS {
            let tokens = tokenize_python("x = 1\n", version);
            assert!(
                tokens.len() >= 4,
                "Version {} should produce at least 4 tokens, got {}",
                version, tokens.len()
            );
            assert_eq!(
                tokens.last().unwrap().type_, TokenType::Eof,
                "Version {} should end with EOF",
                version
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 15: Unsupported version panics
    // -----------------------------------------------------------------------

    /// An unsupported version string should cause a panic with a helpful
    /// error message listing the valid versions.
    #[test]
    #[should_panic(expected = "Unsupported Python version")]
    fn test_unsupported_version() {
        tokenize_python("x = 1\n", "4.0");
    }

    // -----------------------------------------------------------------------
    // Test 16: Default version is 3.12
    // -----------------------------------------------------------------------

    /// Verify that using the DEFAULT_VERSION constant gives the same result
    /// as explicitly passing "3.12".
    #[test]
    fn test_default_version() {
        let tokens_default = tokenize_python("x = 1\n", DEFAULT_VERSION);
        let tokens_explicit = tokenize_python("x = 1\n", "3.12");

        assert_eq!(tokens_default.len(), tokens_explicit.len());
        for (a, b) in tokens_default.iter().zip(tokens_explicit.iter()) {
            assert_eq!(a.type_, b.type_);
            assert_eq!(a.value, b.value);
        }
    }

    // -----------------------------------------------------------------------
    // Test 17: Soft keywords in grammar (3.10+)
    // -----------------------------------------------------------------------

    /// Python 3.10+ grammar files declare soft_keywords. Verify that the
    /// grammar is parsed correctly and the soft keywords are available.
    #[test]
    fn test_soft_keywords_in_grammar() {
        // Load the 3.12 grammar and verify soft_keywords are parsed.
        let path = grammar_path("3.12");
        let grammar_text = fs::read_to_string(&path)
            .expect("Should read python3.12.tokens");
        let grammar = parse_token_grammar(&grammar_text)
            .expect("Should parse python3.12.tokens");

        // Python 3.12 should have match, case, _, type as soft keywords.
        assert!(
            grammar.soft_keywords.contains(&"match".to_string()),
            "Expected 'match' in soft_keywords"
        );
        assert!(
            grammar.soft_keywords.contains(&"case".to_string()),
            "Expected 'case' in soft_keywords"
        );
        assert!(
            grammar.soft_keywords.contains(&"type".to_string()),
            "Expected 'type' in soft_keywords"
        );
    }

    // -----------------------------------------------------------------------
    // Test 18: Soft keywords are NAME tokens (not KEYWORD)
    // -----------------------------------------------------------------------

    /// Soft keywords like `match` and `case` should be emitted as NAME
    /// tokens, not KEYWORD tokens. The parser is responsible for
    /// disambiguating them based on syntactic context.
    #[test]
    fn test_soft_keywords_are_name_tokens() {
        // "match" used as a variable name — should be NAME, not KEYWORD.
        let tokens = tokenize_python("match = 42\n", "3.12");
        assert_eq!(
            tokens[0].type_, TokenType::Name,
            "Expected 'match' to be NAME (soft keyword), got {:?}",
            tokens[0].type_
        );
        assert_eq!(tokens[0].value, "match");

        // "type" used as a variable name — should be NAME.
        let tokens = tokenize_python("type = int\n", "3.12");
        assert_eq!(
            tokens[0].type_, TokenType::Name,
            "Expected 'type' to be NAME (soft keyword), got {:?}",
            tokens[0].type_
        );
    }

    // -----------------------------------------------------------------------
    // Test 19: Python 2.7 does not have soft keywords
    // -----------------------------------------------------------------------

    /// Python 2.7 grammar should have no soft keywords.
    #[test]
    fn test_python27_no_soft_keywords() {
        let path = grammar_path("2.7");
        let grammar_text = fs::read_to_string(&path)
            .expect("Should read python2.7.tokens");
        let grammar = parse_token_grammar(&grammar_text)
            .expect("Should parse python2.7.tokens");

        assert!(
            grammar.soft_keywords.is_empty(),
            "Python 2.7 should have no soft keywords"
        );
    }
}
