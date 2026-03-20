//! # JSON Lexer — tokenizing JSON source text.
//!
//! [JSON](https://www.json.org/) (JavaScript Object Notation) is a lightweight
//! data interchange format defined by [RFC 8259](https://tools.ietf.org/html/rfc8259).
//! It is the lingua franca of web APIs, configuration files, and data storage.
//! JSON's simplicity — only seven value types and six structural characters —
//! makes it an ideal first target for the grammar-driven lexer infrastructure.
//!
//! This crate provides a lexer (tokenizer) for JSON. It does **not** hand-write
//! tokenization rules. Instead, it loads the `json.tokens` grammar file — a
//! declarative description of every token in JSON — and feeds it to the generic
//! [`GrammarLexer`] from the `lexer` crate.
//!
//! # Architecture
//!
//! The tokenization pipeline has three layers:
//!
//! ```text
//! json.tokens          (grammar file on disk)
//!        |
//!        v
//! grammar-tools        (parses .tokens -> TokenGrammar struct)
//!        |
//!        v
//! lexer::GrammarLexer  (tokenizes source using TokenGrammar)
//! ```
//!
//! This crate is the thin glue layer that wires these components together
//! for JSON specifically. It knows where to find `json.tokens` and provides
//! two public entry points:
//!
//! - [`create_json_lexer`] — returns a `GrammarLexer` for fine-grained control.
//! - [`tokenize_json`] — convenience function that returns `Vec<Token>` directly.
//!
//! # Why grammar-driven instead of hand-written?
//!
//! A hand-written lexer for JSON would be ~200 lines of Rust with
//! character-by-character logic for strings and numbers. The grammar-driven
//! approach replaces all that with a 57-line declarative grammar file plus
//! ~30 lines of Rust glue code. When the format evolves (e.g., adding
//! comments for JSON5), you edit the grammar file — no Rust code changes needed.
//!
//! # No indentation, no keywords
//!
//! Unlike Starlark or Python, JSON has no significant whitespace and no
//! keywords. The grammar file uses `mode: default` (implicit), so the
//! lexer does not emit INDENT, DEDENT, or NEWLINE tokens. Whitespace is
//! consumed silently via the `skip:` section. The literal tokens `true`,
//! `false`, and `null` are their own token types (TRUE, FALSE, NULL) rather
//! than being NAME tokens promoted to keywords.

use std::fs;

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::Token;

// ===========================================================================
// Grammar file location
// ===========================================================================

/// Build the path to the `json.tokens` grammar file.
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
///     json.tokens           <-- this is what we want
///   packages/
///     rust/
///       json-lexer/
///         Cargo.toml        <-- CARGO_MANIFEST_DIR points here
///         src/
///           lib.rs          <-- we are here
/// ```
///
/// So the relative path from CARGO_MANIFEST_DIR to the grammar file is:
/// `../../../grammars/json.tokens`
fn grammar_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../../grammars/json.tokens")
}

// ===========================================================================
// Public API
// ===========================================================================

/// Create a `GrammarLexer` configured for JSON source text.
///
/// This function:
/// 1. Reads the `json.tokens` grammar file from disk.
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
/// use coding_adventures_json_lexer::create_json_lexer;
///
/// let mut lexer = create_json_lexer("{\"key\": 42}");
/// let tokens = lexer.tokenize().expect("tokenization failed");
/// for token in &tokens {
///     println!("{}", token);
/// }
/// ```
pub fn create_json_lexer(source: &str) -> GrammarLexer<'_> {
    // Step 1: Read the grammar file from disk.
    //
    // We read the file at runtime (not compile time) because the grammar file
    // may be updated independently of this crate. This also avoids bloating
    // the binary with embedded grammar text.
    let grammar_text = fs::read_to_string(grammar_path())
        .unwrap_or_else(|e| panic!("Failed to read json.tokens: {e}"));

    // Step 2: Parse the grammar text into a structured TokenGrammar.
    //
    // The TokenGrammar contains:
    //   - Token definitions (patterns, names)
    //   - Skip patterns (whitespace)
    //   - No keywords (JSON has none)
    //   - No reserved keywords
    //   - Mode: default (no indentation tracking)
    let grammar = parse_token_grammar(&grammar_text)
        .unwrap_or_else(|e| panic!("Failed to parse json.tokens: {e}"));

    // Step 3: Create and return the lexer.
    //
    // The GrammarLexer compiles all token patterns into anchored regexes
    // and is ready to tokenize the source string.
    GrammarLexer::new(source, &grammar)
}

/// Tokenize JSON source text into a vector of tokens.
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
/// use coding_adventures_json_lexer::tokenize_json;
///
/// let tokens = tokenize_json("{\"name\": \"Alice\", \"age\": 30}");
/// for token in &tokens {
///     println!("{:?} {:?}", token.type_, token.value);
/// }
/// ```
pub fn tokenize_json(source: &str) -> Vec<Token> {
    // Create a fresh lexer for this source text.
    let mut json_lexer = create_json_lexer(source);

    // Tokenize and unwrap — any LexerError becomes a panic.
    //
    // In a production tool, you would want to propagate the error
    // via Result. For this educational codebase, panicking with a clear
    // message is sufficient and keeps the API simple.
    json_lexer
        .tokenize()
        .unwrap_or_else(|e| panic!("JSON tokenization failed: {e}"))
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

    /// Extract the (TokenType, value) pairs from a token stream, excluding
    /// the final EOF token. This makes test assertions more concise.
    ///
    /// For known token types (NUMBER, STRING, LBRACE, etc.), the lexer maps
    /// them to TokenType enum variants and sets `type_name` to `None`.
    /// For custom types (TRUE, FALSE, NULL), `type_` is `TokenType::Name`
    /// and `type_name` holds the grammar name (e.g., `Some("TRUE")`).
    fn token_pairs(tokens: &[Token]) -> Vec<(TokenType, &str)> {
        tokens
            .iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Test 1: Simple number
    // -----------------------------------------------------------------------

    /// Verify that a plain integer is tokenized as a NUMBER token.
    /// NUMBER maps to the built-in TokenType::Number.
    #[test]
    fn test_tokenize_integer() {
        let tokens = tokenize_json("42");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "42");
    }

    // -----------------------------------------------------------------------
    // Test 2: Negative number
    // -----------------------------------------------------------------------

    /// In JSON, the minus sign is part of the number token, not a separate
    /// operator. `-42` should be a single NUMBER token, not MINUS + NUMBER.
    #[test]
    fn test_tokenize_negative_number() {
        let tokens = tokenize_json("-42");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "-42");
    }

    // -----------------------------------------------------------------------
    // Test 3: Decimal number
    // -----------------------------------------------------------------------

    /// Floating-point numbers with a decimal point.
    #[test]
    fn test_tokenize_decimal() {
        let tokens = tokenize_json("3.14");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "3.14");
    }

    // -----------------------------------------------------------------------
    // Test 4: Exponent notation
    // -----------------------------------------------------------------------

    /// Scientific notation with exponents: 1e10, 2.5E-3, etc.
    #[test]
    fn test_tokenize_exponent() {
        let tokens = tokenize_json("1e10");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "1e10");

        // Also test negative exponent with decimal.
        let tokens2 = tokenize_json("2.5E-3");
        let pairs2 = token_pairs(&tokens2);

        assert_eq!(pairs2.len(), 1);
        assert_eq!(pairs2[0].0, TokenType::Number);
        assert_eq!(pairs2[0].1, "2.5E-3");
    }

    // -----------------------------------------------------------------------
    // Test 5: String token
    // -----------------------------------------------------------------------

    /// Double-quoted strings. The lexer should strip the surrounding quotes
    /// and return the inner content as the token value.
    #[test]
    fn test_tokenize_string() {
        let tokens = tokenize_json("\"hello world\"");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::String);
        assert_eq!(pairs[0].1, "hello world");
    }

    // -----------------------------------------------------------------------
    // Test 6: String with escape sequences
    // -----------------------------------------------------------------------

    /// JSON strings can contain escape sequences like \n, \t, \\, \", etc.
    #[test]
    fn test_tokenize_string_escapes() {
        let tokens = tokenize_json("\"line1\\nline2\"");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        // The lexer processes escape sequences, so \n becomes a real newline.
        assert!(pairs[0].1.contains("line1") && pairs[0].1.contains("line2"));
    }

    // -----------------------------------------------------------------------
    // Test 7: true, false, null literals
    // -----------------------------------------------------------------------

    /// The three JSON literal values each have their own token type:
    /// TRUE, FALSE, and NULL. Since these are not in the built-in TokenType
    /// enum, they map to TokenType::Name with type_name set to the grammar
    /// name ("TRUE", "FALSE", "NULL").
    #[test]
    fn test_tokenize_true_false_null() {
        let tokens_true = tokenize_json("true");
        let non_eof: Vec<&Token> = tokens_true.iter().filter(|t| t.type_ != TokenType::Eof).collect();
        assert_eq!(non_eof.len(), 1);
        assert_eq!(non_eof[0].type_name.as_deref(), Some("TRUE"));
        assert_eq!(non_eof[0].value, "true");

        let tokens_false = tokenize_json("false");
        let non_eof: Vec<&Token> = tokens_false.iter().filter(|t| t.type_ != TokenType::Eof).collect();
        assert_eq!(non_eof.len(), 1);
        assert_eq!(non_eof[0].type_name.as_deref(), Some("FALSE"));
        assert_eq!(non_eof[0].value, "false");

        let tokens_null = tokenize_json("null");
        let non_eof: Vec<&Token> = tokens_null.iter().filter(|t| t.type_ != TokenType::Eof).collect();
        assert_eq!(non_eof.len(), 1);
        assert_eq!(non_eof[0].type_name.as_deref(), Some("NULL"));
        assert_eq!(non_eof[0].value, "null");
    }

    // -----------------------------------------------------------------------
    // Test 8: Structural tokens (braces, brackets, colon, comma)
    // -----------------------------------------------------------------------

    /// JSON has six structural characters: { } [ ] : ,
    /// Each maps to a built-in TokenType variant.
    #[test]
    fn test_tokenize_structural_tokens() {
        let tokens = tokenize_json("{ } [ ] : ,");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 6);

        let types: Vec<TokenType> = pairs.iter().map(|(t, _)| *t).collect();

        assert_eq!(types[0], TokenType::LBrace);
        assert_eq!(types[1], TokenType::RBrace);
        assert_eq!(types[2], TokenType::LBracket);
        assert_eq!(types[3], TokenType::RBracket);
        assert_eq!(types[4], TokenType::Colon);
        assert_eq!(types[5], TokenType::Comma);
    }

    // -----------------------------------------------------------------------
    // Test 9: Simple JSON object
    // -----------------------------------------------------------------------

    /// A minimal JSON object with one key-value pair. This tests the
    /// interaction of STRING, COLON, NUMBER, LBRACE, and RBRACE tokens.
    #[test]
    fn test_tokenize_simple_object() {
        let tokens = tokenize_json("{\"key\": 42}");
        let pairs = token_pairs(&tokens);

        // Expected: LBRACE, STRING("key"), COLON, NUMBER("42"), RBRACE
        assert_eq!(pairs.len(), 5);
        assert_eq!(pairs[0].0, TokenType::LBrace);
        assert_eq!(pairs[1].0, TokenType::String);
        assert_eq!(pairs[2].0, TokenType::Colon);
        assert_eq!(pairs[3].0, TokenType::Number);
        assert_eq!(pairs[4].0, TokenType::RBrace);
    }

    // -----------------------------------------------------------------------
    // Test 10: Simple JSON array
    // -----------------------------------------------------------------------

    /// A JSON array with multiple values of different types.
    #[test]
    fn test_tokenize_array() {
        let tokens = tokenize_json("[1, \"two\", true, null]");
        let pairs = token_pairs(&tokens);

        // Expected: LBRACKET, NUMBER, COMMA, STRING, COMMA, TRUE, COMMA, NULL, RBRACKET
        assert_eq!(pairs.len(), 9);
        assert_eq!(pairs[0].0, TokenType::LBracket);
        assert_eq!(pairs[8].0, TokenType::RBracket);
    }

    // -----------------------------------------------------------------------
    // Test 11: Whitespace is skipped
    // -----------------------------------------------------------------------

    /// Whitespace (spaces, tabs, newlines, carriage returns) between tokens
    /// should be consumed without producing tokens.
    #[test]
    fn test_whitespace_skipped() {
        let tokens_compact = tokenize_json("{\"a\":1}");
        let tokens_spaced = tokenize_json("  {  \"a\"  :  1  }  ");
        let tokens_newlines = tokenize_json("{\n  \"a\":\n  1\n}");

        // All three should produce the same tokens (ignoring position info).
        let pairs_compact = token_pairs(&tokens_compact);
        let pairs_spaced = token_pairs(&tokens_spaced);
        let pairs_newlines = token_pairs(&tokens_newlines);

        assert_eq!(pairs_compact.len(), pairs_spaced.len());
        assert_eq!(pairs_compact.len(), pairs_newlines.len());

        // Values should be identical.
        for i in 0..pairs_compact.len() {
            assert_eq!(pairs_compact[i].1, pairs_spaced[i].1);
            assert_eq!(pairs_compact[i].1, pairs_newlines[i].1);
        }
    }

    // -----------------------------------------------------------------------
    // Test 12: Factory function returns a working lexer
    // -----------------------------------------------------------------------

    /// The `create_json_lexer` factory function should return a `GrammarLexer`
    /// that can successfully tokenize source text.
    #[test]
    fn test_create_lexer() {
        let mut lexer = create_json_lexer("42");
        let tokens = lexer.tokenize().expect("Lexer should tokenize successfully");

        // Should produce: NUMBER("42"), EOF
        assert!(tokens.len() >= 2);
        assert_eq!(tokens.last().unwrap().type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Test 13: Zero is a valid number
    // -----------------------------------------------------------------------

    /// The number 0 is valid in JSON (but 007 is not, because leading zeros
    /// are not allowed). We just test that 0 tokenizes correctly.
    #[test]
    fn test_tokenize_zero() {
        let tokens = tokenize_json("0");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "0");
    }

    // -----------------------------------------------------------------------
    // Test 14: Negative decimal with exponent
    // -----------------------------------------------------------------------

    /// A number that exercises all three optional parts of the NUMBER regex:
    /// negative sign, decimal fraction, and exponent.
    #[test]
    fn test_tokenize_full_number() {
        let tokens = tokenize_json("-3.14e+2");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::Number);
        assert_eq!(pairs[0].1, "-3.14e+2");
    }

    // -----------------------------------------------------------------------
    // Test 15: Empty string
    // -----------------------------------------------------------------------

    /// An empty JSON string `""` should produce a STRING token with an
    /// empty value.
    #[test]
    fn test_tokenize_empty_string() {
        let tokens = tokenize_json("\"\"");
        let pairs = token_pairs(&tokens);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, TokenType::String);
        assert_eq!(pairs[0].1, "");
    }

    // -----------------------------------------------------------------------
    // Test 16: Nested structure tokens
    // -----------------------------------------------------------------------

    /// A nested JSON structure exercises the lexer's ability to handle
    /// multiple levels of braces and brackets without confusion.
    #[test]
    fn test_tokenize_nested() {
        let source = "{\"a\": [1, {\"b\": 2}]}";
        let tokens = tokenize_json(source);
        let pairs = token_pairs(&tokens);

        // Count structural tokens using TokenType.
        let lbrace_count = pairs.iter().filter(|(t, _)| *t == TokenType::LBrace).count();
        let rbrace_count = pairs.iter().filter(|(t, _)| *t == TokenType::RBrace).count();
        let lbracket_count = pairs.iter().filter(|(t, _)| *t == TokenType::LBracket).count();
        let rbracket_count = pairs.iter().filter(|(t, _)| *t == TokenType::RBracket).count();

        assert_eq!(lbrace_count, 2, "Expected 2 opening braces");
        assert_eq!(rbrace_count, 2, "Expected 2 closing braces");
        assert_eq!(lbracket_count, 1, "Expected 1 opening bracket");
        assert_eq!(rbracket_count, 1, "Expected 1 closing bracket");
    }

    // -----------------------------------------------------------------------
    // Test 17: Empty object and empty array
    // -----------------------------------------------------------------------

    /// Empty containers `{}` and `[]` are valid JSON. They should produce
    /// just the opening and closing tokens.
    #[test]
    fn test_tokenize_empty_containers() {
        let tokens_obj = tokenize_json("{}");
        let pairs_obj = token_pairs(&tokens_obj);
        assert_eq!(pairs_obj.len(), 2);
        assert_eq!(pairs_obj[0].0, TokenType::LBrace);
        assert_eq!(pairs_obj[1].0, TokenType::RBrace);

        let tokens_arr = tokenize_json("[]");
        let pairs_arr = token_pairs(&tokens_arr);
        assert_eq!(pairs_arr.len(), 2);
        assert_eq!(pairs_arr[0].0, TokenType::LBracket);
        assert_eq!(pairs_arr[1].0, TokenType::RBracket);
    }
}
