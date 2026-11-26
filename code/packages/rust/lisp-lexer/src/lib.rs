//! # Lisp Lexer -- tokenizes Lisp source code into tokens.
//!
//! A **lexer** (also called a tokenizer or scanner) is the first stage in any
//! language processing pipeline. Its job is to take raw source text and break it
//! into meaningful units called **tokens**.
//!
//! ## Why Lisp Tokenization is Simple
//!
//! Lisp has one of the simplest token sets of any programming language. Compare:
//!
//! | Language   | Token Types | Special Cases                     |
//! |-----------|-------------|-----------------------------------|
//! | CSS        | ~20         | Units, colors, selectors, at-rules|
//! | JavaScript | ~15         | Regex literals, template strings  |
//! | **Lisp**   | **7**       | Negative numbers, operator symbols|
//!
//! Lisp's simplicity comes from its uniform syntax: everything is either an
//! **atom** (number, symbol, string) or a **list** (stuff in parentheses).
//! There are no operators, no keywords, no special syntax -- just atoms and lists.
//!
//! ## The One Tricky Part: Symbols
//!
//! In most languages, `+` is an operator with special tokenization rules. In Lisp,
//! `+` is just a symbol -- the same kind of token as `define` or `factorial`.
//! Lisp symbols can contain characters that other languages treat as operators:
//!
//! ```text
//! Valid Lisp symbols: +  -  *  /  =  <  >  <=  >=  set!  null?  list->string
//! ```
//!
//! This means the symbol character class must include `+`, `-`, `*`, `/`, `=`,
//! `<`, `>`, `!`, `?`, and `&`. The only tricky disambiguation is between
//! negative numbers (`-42`) and the minus symbol (`-`): if `-` is followed by
//! a digit, it's a number; otherwise it's a symbol.
//!
//! ## Token Priority
//!
//! When a character could start multiple token types, we use priority ordering:
//!
//! 1. **Whitespace and comments** -- always skipped first
//! 2. **Single-character delimiters** -- `(`, `)`, `'`, `.`
//! 3. **Strings** -- start with `"`
//! 4. **Numbers** -- start with digit, or `-` followed by digit
//! 5. **Symbols** -- everything else that matches the symbol character class

use std::fmt;

// ============================================================================
// Section 1: Token Types
// ============================================================================
//
// A token is a categorized chunk of source text. Each token has a **type**
// (what kind of thing it is) and a **value** (the actual text it came from).
//
// For example, in `(+ 42 x)`:
//   - `(`  -> Token { token_type: LParen, value: "(" }
//   - `+`  -> Token { token_type: Symbol, value: "+" }
//   - `42` -> Token { token_type: Number, value: "42" }
//   - `x`  -> Token { token_type: Symbol, value: "x" }
//   - `)`  -> Token { token_type: RParen, value: ")" }
// ============================================================================

/// The different kinds of tokens that can appear in Lisp source code.
///
/// Lisp has only 7 meaningful token types (plus EOF). This is far fewer than
/// most languages -- JavaScript has ~15, CSS has ~20. The simplicity of Lisp's
/// token set reflects the simplicity of its syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    /// An integer literal, possibly negative. Examples: `42`, `-7`, `0`.
    ///
    /// The regex pattern is `/-?[0-9]+/`. Negative numbers are tokenized as
    /// a single token (not a minus operator followed by a number) because
    /// NUMBER has higher priority than SYMBOL in the grammar.
    Number,

    /// An identifier or operator name. Examples: `define`, `+`, `car`, `null?`.
    ///
    /// Lisp symbols can contain characters that most languages reserve for
    /// operators: `+`, `-`, `*`, `/`, `=`, `<`, `>`, `!`, `?`, `&`.
    /// The character class is: `[a-zA-Z_+\-*/=<>!?&][a-zA-Z0-9_+\-*/=<>!?&]*`
    Symbol,

    /// A double-quoted string literal. Example: `"hello world"`.
    ///
    /// Strings support escape sequences with backslash: `\"`, `\\`, etc.
    /// The value stored in the token includes the surrounding quotes.
    String,

    /// Opening parenthesis `(`. Marks the start of a list.
    LParen,

    /// Closing parenthesis `)`. Marks the end of a list.
    RParen,

    /// Single quote `'`. Syntactic sugar for `(quote ...)`.
    ///
    /// Writing `'foo` is equivalent to writing `(quote foo)`.
    /// Writing `'(1 2 3)` is equivalent to `(quote (1 2 3))`.
    Quote,

    /// Dot `.` separator, used in dotted pair notation.
    ///
    /// Example: `(a . b)` creates a cons cell with car=a and cdr=b.
    Dot,

    /// End of input. Every token stream ends with exactly one EOF token.
    Eof,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::Number => write!(f, "NUMBER"),
            TokenType::Symbol => write!(f, "SYMBOL"),
            TokenType::String => write!(f, "STRING"),
            TokenType::LParen => write!(f, "LPAREN"),
            TokenType::RParen => write!(f, "RPAREN"),
            TokenType::Quote => write!(f, "QUOTE"),
            TokenType::Dot => write!(f, "DOT"),
            TokenType::Eof => write!(f, "EOF"),
        }
    }
}

/// A single token extracted from Lisp source code.
///
/// Each token records its type and the original text (value) it was extracted
/// from. Position tracking could be added in the future for better error
/// messages, but for now we keep it simple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// What kind of token this is (Number, Symbol, LParen, etc.).
    pub token_type: TokenType,

    /// The original source text for this token.
    ///
    /// For numbers, this is the digit string (e.g., `"42"` or `"-7"`).
    /// For symbols, the identifier text (e.g., `"define"` or `"+"`).
    /// For strings, the text including surrounding quotes (e.g., `"\"hello\""`).
    /// For delimiters, the single character (e.g., `"("`, `"'"`).
    /// For EOF, an empty string.
    pub value: std::string::String,
}

impl Token {
    /// Create a new token with the given type and value.
    pub fn new(token_type: TokenType, value: impl Into<std::string::String>) -> Self {
        Token {
            token_type,
            value: value.into(),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Token({}, {:?})", self.token_type, self.value)
    }
}

// ============================================================================
// Section 2: Error Type
// ============================================================================
//
// When the lexer encounters a character it doesn't recognize, it produces a
// LexerError. Good error messages include the position and the offending
// character so the user can find and fix the problem.
// ============================================================================

/// An error that occurs during tokenization.
///
/// This error is produced when the lexer encounters a character that doesn't
/// match any known token pattern. It includes the position (byte offset) and
/// the offending character for debugging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    /// A human-readable description of the error.
    pub message: std::string::String,
    /// The byte offset in the source where the error occurred.
    pub position: usize,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LexerError at position {}: {}", self.position, self.message)
    }
}

impl std::error::Error for LexerError {}

// ============================================================================
// Section 3: Grammar-driven tokenizer
// ============================================================================
//
// This crate is a *thin wrapper*, not a hand-written lexer. Every Lisp
// lexing rule lives in `code/grammars/lisp/lisp.tokens`, compiled to the
// checked-in `src/_grammar.rs` (regenerate with
// `grammar-tools compile-tokens lisp.tokens`). [`tokenize`] hands that
// grammar to the shared [`lexer::grammar_lexer::GrammarLexer`] and maps
// the generic tokens back onto this crate's public [`Token`] API, so
// downstream crates (`lisp-parser`) are unaffected by the retirement of
// the hand-written scanner.
//
// Two hand-scanner over-acceptances were retired along the way; both now
// agree with the Go lane and the spec (`code/specs/lisp.md`):
// form-feed is no longer skipped as whitespace, and a backslash-newline
// inside a string no longer scans (the grammar's `\\.` never matches
// `\\n`). Neither has ever appeared in a test or real input.
// ============================================================================

mod _grammar;

use std::sync::OnceLock;

use _grammar::token_grammar;
use grammar_tools::token_grammar::TokenGrammar;
use lexer::grammar_lexer::GrammarLexer;

static LISP_TOKEN_GRAMMAR: OnceLock<TokenGrammar> = OnceLock::new();

fn lisp_token_grammar() -> &'static TokenGrammar {
    LISP_TOKEN_GRAMMAR.get_or_init(token_grammar)
}

/// Borrow the compiled `lisp.tokens` grammar.
///
/// Exposed for tooling that wants to introspect the token set without
/// re-parsing the canonical `.tokens` file.
pub fn lisp_token_grammar_spec() -> &'static TokenGrammar {
    lisp_token_grammar()
}

/// Tokenize Lisp source code into a vector of tokens.
///
/// This is the main entry point for the Lisp lexer. Pass in a string of Lisp
/// source code, and get back a `Vec<Token>`. The vector always ends with an
/// `Eof` token.
///
/// # Token types produced
///
/// - `Number` -- integer literals, possibly negative (`42`, `-7`)
/// - `Symbol` -- identifiers and operators (`define`, `+`, `car`)
/// - `String` -- double-quoted strings (`"hello"`)
/// - `LParen` / `RParen` -- `(` and `)`
/// - `Quote` -- `'` (syntactic sugar for `(quote ...)`)
/// - `Dot` -- `.` (for dotted pairs like `(a . b)`)
///
/// Token priority comes from definition order in `lisp.tokens`: skip
/// patterns first, then NUMBER before SYMBOL (so `-42` is one token),
/// then the rest. `.` is always DOT, exactly as before.
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
    let line_starts = line_start_chars(source);
    let engine_tokens = GrammarLexer::new(source, lisp_token_grammar())
        .tokenize()
        .map_err(|e| map_error(source, &line_starts, &e))?;
    engine_tokens.iter().map(map_token).collect()
}

/// Char offsets at which each line starts (line 1 starts at 0).
fn line_start_chars(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, ch) in source.chars().enumerate() {
        if ch == '\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Map an engine token onto this crate's public [`Token`] type.
///
/// String values regain their quotes: the grammar strips them (`escapes:
/// none` keeps the inner text raw) while this API keeps them, so the
/// reconstruction is byte-identical to the lexeme.
fn map_token(token: &lexer::token::Token) -> Result<Token, LexerError> {
    let token_type = match token.effective_type_name() {
        "NUMBER" => TokenType::Number,
        "SYMBOL" => TokenType::Symbol,
        "STRING" => TokenType::String,
        "LPAREN" => TokenType::LParen,
        "RPAREN" => TokenType::RParen,
        "QUOTE" => TokenType::Quote,
        "DOT" => TokenType::Dot,
        "EOF" => TokenType::Eof,
        other => {
            return Err(LexerError {
                message: format!(
                    "Internal error: fixed lisp grammar emitted unknown token type {other}"
                ),
                position: 0,
            })
        }
    };
    let value = if token_type == TokenType::String {
        format!("\"{}\"", token.value)
    } else {
        token.value.clone()
    };
    Ok(Token { token_type, value })
}

/// Map an engine error onto this crate's [`LexerError`].
///
/// Positions convert from 1-based line/column (chars) back to the char
/// offset this API reports. A failure at a `"` always opens an
/// unterminated string -- no other rule consumes quotes -- while any
/// other failure re-words to the historical message with identical
/// `{:?}` rendering.
fn map_error(source: &str, line_starts: &[usize], err: &lexer::token::LexerError) -> LexerError {
    let line_start = line_starts
        .get(err.line.saturating_sub(1))
        .copied()
        .unwrap_or(0);
    let position = line_start + err.column.saturating_sub(1);
    let ch = source.chars().nth(position);
    if ch == Some('"') {
        return LexerError {
            message: "Unterminated string literal".to_string(),
            position,
        };
    }
    match ch {
        Some(c) => LexerError {
            message: format!("Unexpected character: {c:?}"),
            position,
        },
        None => LexerError {
            message: err.message.clone(),
            position,
        },
    }
}

// ============================================================================
// Section 5: Tests
// ============================================================================
//
// These tests verify that the lexer correctly tokenizes all Lisp constructs.
// They are organized by category, mirroring the Python test suite.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: tokenize and return just the token types (excluding EOF).
    fn types(source: &str) -> Vec<TokenType> {
        tokenize(source)
            .unwrap()
            .into_iter()
            .filter(|t| t.token_type != TokenType::Eof)
            .map(|t| t.token_type)
            .collect()
    }

    // Helper: tokenize and return just the values (excluding EOF).
    fn values(source: &str) -> Vec<std::string::String> {
        tokenize(source)
            .unwrap()
            .into_iter()
            .filter(|t| t.token_type != TokenType::Eof)
            .map(|t| t.value)
            .collect()
    }

    // =====================================================================
    // Basic Atoms
    // =====================================================================

    #[test]
    fn test_number() {
        assert_eq!(types("42"), vec![TokenType::Number]);
        assert_eq!(values("42"), vec!["42"]);
    }

    #[test]
    fn test_negative_number() {
        assert_eq!(types("-7"), vec![TokenType::Number]);
        assert_eq!(values("-7"), vec!["-7"]);
    }

    #[test]
    fn test_zero() {
        assert_eq!(types("0"), vec![TokenType::Number]);
    }

    #[test]
    fn test_symbol() {
        assert_eq!(types("define"), vec![TokenType::Symbol]);
        assert_eq!(values("define"), vec!["define"]);
    }

    #[test]
    fn test_string() {
        let tokens = tokenize("\"hello world\"").unwrap();
        let non_eof: Vec<_> = tokens.iter().filter(|t| t.token_type != TokenType::Eof).collect();
        assert_eq!(non_eof.len(), 1);
        assert_eq!(non_eof[0].token_type, TokenType::String);
    }

    #[test]
    fn test_string_with_escape() {
        let tokens = tokenize(r#""hello \"world\"""#).unwrap();
        let non_eof: Vec<_> = tokens.iter().filter(|t| t.token_type != TokenType::Eof).collect();
        assert_eq!(non_eof.len(), 1);
        assert_eq!(non_eof[0].token_type, TokenType::String);
    }

    // =====================================================================
    // Operator Symbols
    // =====================================================================

    #[test]
    fn test_plus() {
        assert_eq!(types("+"), vec![TokenType::Symbol]);
        assert_eq!(values("+"), vec!["+"]);
    }

    #[test]
    fn test_minus_in_expression() {
        // In `(- 3 1)`, the `-` is a Symbol because it's followed by space, not digit.
        assert_eq!(
            types("(- 3 1)"),
            vec![
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::Number,
                TokenType::Number,
                TokenType::RParen,
            ]
        );
    }

    #[test]
    fn test_star() {
        assert_eq!(types("*"), vec![TokenType::Symbol]);
    }

    #[test]
    fn test_slash() {
        assert_eq!(types("/"), vec![TokenType::Symbol]);
    }

    #[test]
    fn test_equals() {
        assert_eq!(types("="), vec![TokenType::Symbol]);
    }

    #[test]
    fn test_comparison() {
        assert_eq!(
            types("< > <= >="),
            vec![
                TokenType::Symbol,
                TokenType::Symbol,
                TokenType::Symbol,
                TokenType::Symbol,
            ]
        );
    }

    #[test]
    fn test_multi_char_symbol() {
        assert_eq!(types("set!"), vec![TokenType::Symbol]);
        assert_eq!(values("set!"), vec!["set!"]);
        assert_eq!(types("null?"), vec![TokenType::Symbol]);
        assert_eq!(values("null?"), vec!["null?"]);
    }

    // =====================================================================
    // Delimiters
    // =====================================================================

    #[test]
    fn test_parentheses() {
        assert_eq!(types("()"), vec![TokenType::LParen, TokenType::RParen]);
    }

    #[test]
    fn test_quote() {
        assert_eq!(types("'x"), vec![TokenType::Quote, TokenType::Symbol]);
    }

    #[test]
    fn test_dot() {
        assert_eq!(
            types("(a . b)"),
            vec![
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::Dot,
                TokenType::Symbol,
                TokenType::RParen,
            ]
        );
    }

    // =====================================================================
    // Whitespace and Comments
    // =====================================================================

    #[test]
    fn test_whitespace_skipped() {
        assert_eq!(types("  42  "), vec![TokenType::Number]);
        assert_eq!(types("a\tb"), vec![TokenType::Symbol, TokenType::Symbol]);
        assert_eq!(types("a\nb"), vec![TokenType::Symbol, TokenType::Symbol]);
    }

    #[test]
    fn test_comment_skipped() {
        assert_eq!(types("; this is a comment\n42"), vec![TokenType::Number]);
    }

    #[test]
    fn test_inline_comment() {
        assert_eq!(
            types("(+ 1 2) ; add them"),
            vec![
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::Number,
                TokenType::Number,
                TokenType::RParen,
            ]
        );
    }

    // =====================================================================
    // Full Expressions
    // =====================================================================

    #[test]
    fn test_simple_call() {
        assert_eq!(
            types("(+ 1 2)"),
            vec![
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::Number,
                TokenType::Number,
                TokenType::RParen,
            ]
        );
    }

    #[test]
    fn test_nested_call() {
        assert_eq!(
            types("(+ (* 2 3) 4)"),
            vec![
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::Number,
                TokenType::Number,
                TokenType::RParen,
                TokenType::Number,
                TokenType::RParen,
            ]
        );
    }

    #[test]
    fn test_define() {
        assert_eq!(
            types("(define x 42)"),
            vec![
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::Symbol,
                TokenType::Number,
                TokenType::RParen,
            ]
        );
    }

    #[test]
    fn test_lambda() {
        assert_eq!(
            types("(lambda (x) (* x x))"),
            vec![
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::RParen,
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::Symbol,
                TokenType::Symbol,
                TokenType::RParen,
                TokenType::RParen,
            ]
        );
    }

    #[test]
    fn test_quoted_symbol() {
        assert_eq!(types("'foo"), vec![TokenType::Quote, TokenType::Symbol]);
    }

    #[test]
    fn test_quoted_list() {
        assert_eq!(
            types("'(1 2 3)"),
            vec![
                TokenType::Quote,
                TokenType::LParen,
                TokenType::Number,
                TokenType::Number,
                TokenType::Number,
                TokenType::RParen,
            ]
        );
    }

    #[test]
    fn test_dotted_pair() {
        assert_eq!(
            types("(1 . 2)"),
            vec![
                TokenType::LParen,
                TokenType::Number,
                TokenType::Dot,
                TokenType::Number,
                TokenType::RParen,
            ]
        );
    }

    #[test]
    fn test_cond_expression() {
        assert_eq!(
            types("(cond ((eq x 0) 1) (t x))"),
            vec![
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::LParen,
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::Symbol,
                TokenType::Number,
                TokenType::RParen,
                TokenType::Number,
                TokenType::RParen,
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::Symbol,
                TokenType::RParen,
                TokenType::RParen,
            ]
        );
    }

    #[test]
    fn test_factorial() {
        let source = r#"
        (define factorial
          (lambda (n)
            (cond ((eq n 0) 1)
                  (t (* n (factorial (- n 1)))))))
        "#;
        let tokens: Vec<_> = tokenize(source)
            .unwrap()
            .into_iter()
            .filter(|t| t.token_type != TokenType::Eof)
            .collect();
        assert!(tokens.len() > 20);
        assert_eq!(tokens[0].token_type, TokenType::LParen);
        assert_eq!(tokens[1].value, "define");
        assert_eq!(tokens[2].value, "factorial");
    }

    #[test]
    fn test_empty_input() {
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Eof);
    }

    #[test]
    fn test_only_comments() {
        let tokens = tokenize("; just a comment\n; another one").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Eof);
    }

    #[test]
    fn test_eof_always_present() {
        let tokens = tokenize("(+ 1 2)").unwrap();
        assert_eq!(tokens.last().unwrap().token_type, TokenType::Eof);
    }

    // =====================================================================
    // Number vs Symbol Disambiguation
    // =====================================================================

    #[test]
    fn test_negative_number_in_context() {
        assert_eq!(types("-42"), vec![TokenType::Number]);
        assert_eq!(values("-42"), vec!["-42"]);
    }

    #[test]
    fn test_subtraction_expression() {
        let t = types("(- 3 1)");
        assert_eq!(
            t,
            vec![
                TokenType::LParen,
                TokenType::Symbol,
                TokenType::Number,
                TokenType::Number,
                TokenType::RParen,
            ]
        );
        let v = values("(- 3 1)");
        assert_eq!(v, vec!["(", "-", "3", "1", ")"]);
    }

    // =====================================================================
    // Error Cases
    // =====================================================================

    #[test]
    fn test_unterminated_string() {
        let result = tokenize("\"hello");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unterminated string"));
    }

    #[test]
    fn test_unexpected_character() {
        let result = tokenize("@");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unexpected character"));
    }

    // =====================================================================
    // Grammar-migration pins
    // =====================================================================

    #[test]
    fn checked_in_grammar_matches_lisp_tokens() {
        // Drift guard: the compiled `_grammar.rs` must equal a fresh
        // parse of the canonical source. Regenerate with
        // `grammar-tools compile-tokens lisp.tokens` (Rust emitter).
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let tokens_path = manifest
            .parent() // rust
            .unwrap()
            .parent() // packages
            .unwrap()
            .parent() // code
            .unwrap()
            .join("grammars/lisp/lisp.tokens");
        let source = std::fs::read_to_string(&tokens_path).unwrap();
        let fresh =
            grammar_tools::token_grammar::parse_token_grammar(&source).unwrap();
        assert_eq!(&fresh, lisp_token_grammar_spec());
    }

    #[test]
    fn error_positions_span_lines() {
        // `(` `a` `\n` `@`: the error sits on line 2, char offset 3.
        let err = tokenize("(a\n@)").unwrap_err();
        assert_eq!(err.position, 3);
        assert!(err.message.contains("Unexpected character"));
    }

    #[test]
    fn error_positions_count_chars_not_bytes() {
        // `é` is 2 bytes but 1 char: `@` sits at char offset 5, byte
        // offset 6. The line/column-to-offset mapping must count chars.
        let err = tokenize("(\"é\" @)").unwrap_err();
        assert_eq!(err.position, 5);
    }

    #[test]
    fn unterminated_string_on_later_line() {
        let err = tokenize("(a\n\"oops)").unwrap_err();
        assert_eq!(err.position, 3);
        assert!(err.message.contains("Unterminated string"));
    }

    #[test]
    fn form_feed_is_an_error() {
        // Spec-compliance fix retired with the hand scanner: the grammar
        // (and the spec, and the Go lane) never skipped `\x0C`; only
        // `is_ascii_whitespace` did.
        let err = tokenize("a\x0Cb").unwrap_err();
        assert_eq!(err.position, 1);
        assert!(err.message.contains("Unexpected character"));
    }

    #[test]
    fn backslash_newline_in_string_is_unterminated() {
        // The hand scanner skipped any char after `\`, including `\n`;
        // the grammar's `\\.` never matches a newline, so the string
        // cannot close. The grammar governs.
        let err = tokenize("\"a\\\nb\"").unwrap_err();
        assert_eq!(err.position, 0);
        assert!(err.message.contains("Unterminated string"));
    }

    #[test]
    fn spec_accessor_exposes_grammar() {
        let names: Vec<&str> = lisp_token_grammar_spec()
            .definitions
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        assert!(names.contains(&"SYMBOL"));
        assert!(names.contains(&"NUMBER"));
        assert!(names.contains(&"STRING"));
    }
}
