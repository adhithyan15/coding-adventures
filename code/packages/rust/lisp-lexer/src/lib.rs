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
// Section 3: Character Classification
// ============================================================================
//
// Before we can tokenize, we need to know what characters belong to what
// token types. These helper functions classify characters into categories.
//
// The key insight for Lisp is that the symbol character class is very broad:
// it includes letters, digits (after the first character), and many characters
// that other languages reserve for operators.
// ============================================================================

/// Check if a character can **start** a Lisp symbol.
///
/// Symbol-start characters: letters, underscore, and the operator characters
/// `+`, `-`, `*`, `/`, `=`, `<`, `>`, `!`, `?`, `&`.
///
/// Note that digits CANNOT start a symbol -- a leading digit means we're
/// reading a number instead.
fn is_symbol_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
        || ch == '_'
        || ch == '+'
        || ch == '-'
        || ch == '*'
        || ch == '/'
        || ch == '='
        || ch == '<'
        || ch == '>'
        || ch == '!'
        || ch == '?'
        || ch == '&'
}

/// Check if a character can **continue** a Lisp symbol (after the first char).
///
/// Symbol-continuation characters: everything that can start a symbol, plus
/// digits. This allows symbols like `list->string` or `char-at-2`.
fn is_symbol_continue(ch: char) -> bool {
    is_symbol_start(ch) || ch.is_ascii_digit()
}

// ============================================================================
// Section 4: The Tokenizer
// ============================================================================
//
// The tokenizer is a hand-written scanner that processes source text one
// character at a time. It uses a simple algorithm:
//
// 1. Skip whitespace and comments
// 2. Look at the current character to decide what token type to try
// 3. Read the full token (consuming characters)
// 4. Emit the token and go back to step 1
//
// This is the same approach used by most production compilers (GCC, Clang,
// V8, CPython). It's fast, simple, and gives maximum control over edge cases.
// ============================================================================

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
/// - `Eof` -- end of input
///
/// Whitespace and comments (starting with `;`) are automatically skipped.
///
/// # Errors
///
/// Returns `LexerError` if the source contains characters that don't match
/// any Lisp token pattern.
///
/// # Examples
///
/// ```
/// use lisp_lexer::{tokenize, TokenType};
///
/// let tokens = tokenize("(+ 1 2)").unwrap();
/// assert_eq!(tokens[0].token_type, TokenType::LParen);
/// assert_eq!(tokens[1].token_type, TokenType::Symbol);
/// assert_eq!(tokens[1].value, "+");
/// assert_eq!(tokens[2].token_type, TokenType::Number);
/// assert_eq!(tokens[2].value, "1");
/// ```
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut pos = 0;

    while pos < chars.len() {
        // -----------------------------------------------------------------
        // Step 1: Skip whitespace and comments
        // -----------------------------------------------------------------
        // Whitespace: spaces, tabs, carriage returns, newlines.
        // Comments: everything from `;` to end of line.
        // These are not meaningful tokens -- we consume them silently.
        // -----------------------------------------------------------------
        if chars[pos].is_ascii_whitespace() {
            pos += 1;
            continue;
        }

        if chars[pos] == ';' {
            // Skip to end of line (or end of input)
            while pos < chars.len() && chars[pos] != '\n' {
                pos += 1;
            }
            continue;
        }

        // -----------------------------------------------------------------
        // Step 2: Single-character delimiter tokens
        // -----------------------------------------------------------------
        // These are unambiguous -- one character, one token. No lookahead
        // needed.
        // -----------------------------------------------------------------
        match chars[pos] {
            '(' => {
                tokens.push(Token::new(TokenType::LParen, "("));
                pos += 1;
                continue;
            }
            ')' => {
                tokens.push(Token::new(TokenType::RParen, ")"));
                pos += 1;
                continue;
            }
            '\'' => {
                tokens.push(Token::new(TokenType::Quote, "'"));
                pos += 1;
                continue;
            }
            '.' => {
                // A dot is only the DOT token if it's not followed by a digit.
                // (We don't support floating-point numbers in this Lisp, but
                // if we did, `.5` would be ambiguous.)
                tokens.push(Token::new(TokenType::Dot, "."));
                pos += 1;
                continue;
            }
            _ => {}
        }

        // -----------------------------------------------------------------
        // Step 3: String literals
        // -----------------------------------------------------------------
        // Strings start with `"` and end with the matching `"`. Backslash
        // can escape any character inside the string: `\"` produces a literal
        // double-quote, `\\` produces a literal backslash.
        //
        // The value stored in the token INCLUDES the surrounding quotes,
        // matching the Python lexer's behavior. The parser or compiler can
        // strip them later.
        // -----------------------------------------------------------------
        if chars[pos] == '"' {
            let start = pos;
            pos += 1; // skip opening quote
            while pos < chars.len() && chars[pos] != '"' {
                if chars[pos] == '\\' {
                    pos += 1; // skip the backslash
                }
                pos += 1; // skip the character (or the escaped character)
            }
            if pos >= chars.len() {
                return Err(LexerError {
                    message: "Unterminated string literal".to_string(),
                    position: start,
                });
            }
            pos += 1; // skip closing quote
            let value: std::string::String = chars[start..pos].iter().collect();
            tokens.push(Token::new(TokenType::String, value));
            continue;
        }

        // -----------------------------------------------------------------
        // Step 4: Numbers (including negative numbers)
        // -----------------------------------------------------------------
        // A number starts with a digit, or with `-` followed by a digit.
        // The `-` ambiguity is resolved here: if `-` is followed by a digit,
        // we tokenize it as a negative number. Otherwise, `-` falls through
        // to the symbol case below.
        //
        // This priority ordering means:
        //   `-42`  -> Token(Number, "-42")    (one token)
        //   `- 42` -> Token(Symbol, "-"), Token(Number, "42")  (two tokens)
        //   `(- 3 1)` -> LParen, Symbol("-"), Number("3"), Number("1"), RParen
        // -----------------------------------------------------------------
        if chars[pos].is_ascii_digit()
            || (chars[pos] == '-'
                && pos + 1 < chars.len()
                && chars[pos + 1].is_ascii_digit())
        {
            let start = pos;
            if chars[pos] == '-' {
                pos += 1; // consume the minus sign
            }
            while pos < chars.len() && chars[pos].is_ascii_digit() {
                pos += 1;
            }
            let value: std::string::String = chars[start..pos].iter().collect();
            tokens.push(Token::new(TokenType::Number, value));
            continue;
        }

        // -----------------------------------------------------------------
        // Step 5: Symbols
        // -----------------------------------------------------------------
        // Symbols are the catch-all for identifiers and operator names.
        // A symbol starts with a symbol-start character and continues with
        // symbol-continue characters.
        //
        // Examples: `define`, `lambda`, `+`, `<=`, `set!`, `null?`
        // -----------------------------------------------------------------
        if is_symbol_start(chars[pos]) {
            let start = pos;
            pos += 1;
            while pos < chars.len() && is_symbol_continue(chars[pos]) {
                pos += 1;
            }
            let value: std::string::String = chars[start..pos].iter().collect();
            tokens.push(Token::new(TokenType::Symbol, value));
            continue;
        }

        // -----------------------------------------------------------------
        // Step 6: Unrecognized character -- error
        // -----------------------------------------------------------------
        return Err(LexerError {
            message: format!("Unexpected character: {:?}", chars[pos]),
            position: pos,
        });
    }

    // Every token stream ends with EOF. This simplifies the parser: it can
    // always peek at the next token without checking for end-of-input.
    tokens.push(Token::new(TokenType::Eof, ""));

    Ok(tokens)
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
}
