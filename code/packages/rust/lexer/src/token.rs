//! # Tokens — the atoms of a programming language.
//!
//! Before a computer can understand source code, it must break the raw text
//! into meaningful chunks called **tokens**. This is the same process you use
//! when reading English: you don't process one letter at a time — you
//! recognize words, punctuation marks, and numbers as distinct units.
//!
//! A token has two essential properties:
//!
//! 1. **Type** — what *kind* of thing is it? A number? A variable name?
//!    A plus sign? The type tells the parser what role this token plays.
//!
//! 2. **Value** — what is the actual text? The number `42`, the name
//!    `total`, the operator `+`. The value carries the specific content.
//!
//! Tokens also carry their **position** in the source code (line and column),
//! which is critical for error messages. When the parser finds a problem,
//! it can point the user to the exact location in their source file.
//!
//! # Example
//!
//! Given the source code `x = 1 + 2`, a lexer produces these tokens:
//!
//! ```text
//! Token(Name,   "x", 1:1)
//! Token(Equals, "=", 1:3)
//! Token(Number, "1", 1:5)
//! Token(Plus,   "+", 1:7)
//! Token(Number, "2", 1:9)
//! Token(EOF,    "",  1:10)
//! ```
//!
//! Notice the EOF token at the end — it signals "no more input" and gives
//! the parser a clean way to know it has reached the end of the file.

use std::fmt;

// ===========================================================================
// TokenType — the classification of each token
// ===========================================================================

/// Every token the lexer can produce belongs to one of these categories.
///
/// The variants are ordered by frequency of use in typical programs:
/// identifiers and numbers first, then operators, then delimiters, then
/// special tokens like NEWLINE and EOF.
///
/// # Why an enum instead of strings?
///
/// Using an enum gives us three advantages over raw strings:
///
/// 1. **Compile-time checking** — if you misspell a variant, the compiler
///    catches it immediately. With strings, `"NMBER"` would silently pass.
///
/// 2. **Pattern matching** — Rust's `match` on enums is exhaustive. The
///    compiler forces you to handle every variant, preventing forgotten cases.
///
/// 3. **Zero-cost abstraction** — enums are stored as small integers (u8 in
///    this case), so comparing token types is a single integer comparison
///    rather than a string comparison.
///
/// # The full set of token types
///
/// ```text
/// Category     Tokens
/// ----------   ------
/// Values       NAME, NUMBER, STRING, KEYWORD
/// Arithmetic   PLUS, MINUS, STAR, SLASH
/// Assignment   EQUALS, EQUALS_EQUALS
/// Grouping     LPAREN, RPAREN, LBRACE, RBRACE, LBRACKET, RBRACKET
/// Punctuation  COMMA, COLON, SEMICOLON, DOT, BANG
/// Structure    NEWLINE, EOF
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    /// An identifier — a variable name, function name, or type name.
    /// Examples: `x`, `total`, `my_function`, `_private`.
    /// Identifiers start with a letter or underscore, followed by letters,
    /// digits, or underscores.
    Name,

    /// A numeric literal — currently integers only.
    /// Examples: `0`, `42`, `1000`.
    /// Floating-point support would be a natural extension.
    Number,

    /// A string literal — text enclosed in double quotes.
    /// The value stored in the token has escape sequences already processed:
    /// `"hello\n"` becomes the string `hello` followed by a newline character.
    String,

    /// A reserved word — an identifier that the language treats specially.
    /// Examples in Python: `if`, `else`, `while`, `def`, `return`.
    /// The lexer first recognizes these as NAME tokens, then promotes them
    /// to KEYWORD if they appear in the language's keyword list.
    Keyword,

    /// The `+` operator.
    Plus,

    /// The `-` operator.
    Minus,

    /// The `*` operator.
    Star,

    /// The `/` operator.
    Slash,

    /// The `=` assignment operator.
    /// Not to be confused with `==` (equality comparison).
    Equals,

    /// The `==` equality comparison operator.
    /// This is why lexers need lookahead: when the lexer sees `=`, it must
    /// check the next character to decide between `=` and `==`.
    EqualsEquals,

    /// The `(` left parenthesis — opens a group, function call, or tuple.
    LParen,

    /// The `)` right parenthesis — closes a group, function call, or tuple.
    RParen,

    /// The `,` comma — separates items in lists, function arguments, etc.
    Comma,

    /// The `:` colon — begins a block (Python), type annotation, dict entry.
    Colon,

    /// The `;` semicolon — statement terminator in many languages.
    Semicolon,

    /// The `{` left brace — opens a block or dictionary literal.
    LBrace,

    /// The `}` right brace — closes a block or dictionary literal.
    RBrace,

    /// The `[` left bracket — opens an array/list literal or index.
    LBracket,

    /// The `]` right bracket — closes an array/list literal or index.
    RBracket,

    /// The `.` dot — member access (`object.field`).
    Dot,

    /// The `!` bang — logical NOT or suffix operator.
    Bang,

    /// A newline character. In Python, newlines are significant — they
    /// terminate statements. In C-like languages, they are usually whitespace.
    /// The value is stored as the literal string `\n` for display purposes.
    Newline,

    /// End of file. Every token stream ends with exactly one EOF token.
    /// This simplifies parser logic: instead of checking "are there more
    /// tokens?" the parser can just check "is the current token EOF?"
    Eof,
}

/// Human-readable names for each token type.
///
/// This is used by the `Display` implementation so that error messages
/// and debug output show readable names like "Name" instead of "0".
impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            TokenType::Name => "Name",
            TokenType::Number => "Number",
            TokenType::String => "String",
            TokenType::Keyword => "Keyword",
            TokenType::Plus => "Plus",
            TokenType::Minus => "Minus",
            TokenType::Star => "Star",
            TokenType::Slash => "Slash",
            TokenType::Equals => "Equals",
            TokenType::EqualsEquals => "EqualsEquals",
            TokenType::LParen => "LParen",
            TokenType::RParen => "RParen",
            TokenType::Comma => "Comma",
            TokenType::Colon => "Colon",
            TokenType::Semicolon => "Semicolon",
            TokenType::LBrace => "LBrace",
            TokenType::RBrace => "RBrace",
            TokenType::LBracket => "LBracket",
            TokenType::RBracket => "RBracket",
            TokenType::Dot => "Dot",
            TokenType::Bang => "Bang",
            TokenType::Newline => "Newline",
            TokenType::Eof => "EOF",
        };
        write!(f, "{}", name)
    }
}

// ===========================================================================
// Token — a single unit of source code
// ===========================================================================

/// A single token produced by the lexer.
///
/// Each token carries four pieces of information:
///
/// - `type_` — what kind of token this is (see [`TokenType`]).
/// - `value` — the actual source text this token represents. For string
///   tokens, escape sequences have already been processed.
/// - `line` — the 1-based line number where this token starts.
/// - `column` — the 1-based column number where this token starts.
///
/// # Why `type_` instead of `type`?
///
/// `type` is a reserved word in Rust — you cannot use it as a field name.
/// The trailing underscore is a common Rust convention for identifiers that
/// would otherwise collide with keywords (like `type_`, `match_`, `ref_`).
///
/// # Position tracking
///
/// Line and column numbers start at 1 (not 0) because that matches what
/// editors display. When you see an error at "line 5, column 12", you can
/// go to line 5 in your editor and count to the 12th character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The classification of this token.
    pub type_: TokenType,

    /// The source text this token represents.
    /// For string tokens, escape sequences are already resolved:
    /// the source `"hello\n"` produces a value of `hello` + newline.
    pub value: std::string::String,

    /// The 1-based line number where this token starts.
    pub line: usize,

    /// The 1-based column number where this token starts.
    pub column: usize,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Token({}, {:?}, {}:{})",
            self.type_, self.value, self.line, self.column
        )
    }
}

// ===========================================================================
// LexerError — what goes wrong during tokenization
// ===========================================================================

/// An error encountered while tokenizing source code.
///
/// Lexer errors always include the position where the problem was found,
/// so the user can locate and fix the issue in their source file.
///
/// # Common causes
///
/// - **Unexpected character** — a character that does not match any token
///   pattern. For example, `@` in a language that does not use it.
/// - **Unterminated string** — a string literal that reaches EOF without
///   a closing quote.
/// - **Invalid escape sequence** — a backslash followed by a character
///   that is not a recognized escape (some lexers are lenient about this).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    /// Human-readable description of the error.
    pub message: std::string::String,

    /// The 1-based line number where the error occurred.
    pub line: usize,

    /// The 1-based column number where the error occurred.
    pub column: usize,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LexerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for LexerError {}

// ===========================================================================
// Token name to TokenType conversion
// ===========================================================================

/// Convert an UPPERCASE token name (as used in `.grammar` files) to a
/// [`TokenType`] value.
///
/// This is needed by the grammar-driven parser, which reads token type names
/// from grammar rules and needs to match them against actual token types.
///
/// # Token name mapping
///
/// | Grammar name    | TokenType     |
/// |-----------------|---------------|
/// | `NAME`          | `Name`        |
/// | `NUMBER`        | `Number`      |
/// | `STRING`        | `String`      |
/// | `KEYWORD`       | `Keyword`     |
/// | `PLUS`          | `Plus`        |
/// | `MINUS`         | `Minus`       |
/// | `STAR`          | `Star`        |
/// | `SLASH`         | `Slash`       |
/// | `EQUALS`        | `Equals`      |
/// | `EQUALS_EQUALS` | `EqualsEquals`|
/// | `LPAREN`        | `LParen`      |
/// | `RPAREN`        | `RParen`      |
/// | `COMMA`         | `Comma`       |
/// | `COLON`         | `Colon`       |
/// | `SEMICOLON`     | `Semicolon`   |
/// | `LBRACE`        | `LBrace`      |
/// | `RBRACE`        | `RBrace`      |
/// | `LBRACKET`      | `LBracket`    |
/// | `RBRACKET`      | `RBracket`    |
/// | `DOT`           | `Dot`         |
/// | `BANG`          | `Bang`        |
/// | `NEWLINE`       | `Newline`     |
/// | `EOF`           | `Eof`         |
///
/// Unknown names default to `Name`, matching the Go implementation's behavior.
pub fn string_to_token_type(name: &str) -> TokenType {
    match name {
        "NAME" => TokenType::Name,
        "NUMBER" => TokenType::Number,
        "STRING" => TokenType::String,
        "KEYWORD" => TokenType::Keyword,
        "PLUS" => TokenType::Plus,
        "MINUS" => TokenType::Minus,
        "STAR" => TokenType::Star,
        "SLASH" => TokenType::Slash,
        "EQUALS" => TokenType::Equals,
        "EQUALS_EQUALS" => TokenType::EqualsEquals,
        "LPAREN" => TokenType::LParen,
        "RPAREN" => TokenType::RParen,
        "COMMA" => TokenType::Comma,
        "COLON" => TokenType::Colon,
        "SEMICOLON" => TokenType::Semicolon,
        "LBRACE" => TokenType::LBrace,
        "RBRACE" => TokenType::RBrace,
        "LBRACKET" => TokenType::LBracket,
        "RBRACKET" => TokenType::RBracket,
        "DOT" => TokenType::Dot,
        "BANG" => TokenType::Bang,
        "NEWLINE" => TokenType::Newline,
        "EOF" => TokenType::Eof,
        _ => TokenType::Name,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // TokenType Display
    // -----------------------------------------------------------------------

    #[test]
    fn test_token_type_display() {
        // Every variant should have a human-readable display name.
        assert_eq!(format!("{}", TokenType::Name), "Name");
        assert_eq!(format!("{}", TokenType::Number), "Number");
        assert_eq!(format!("{}", TokenType::String), "String");
        assert_eq!(format!("{}", TokenType::Keyword), "Keyword");
        assert_eq!(format!("{}", TokenType::Plus), "Plus");
        assert_eq!(format!("{}", TokenType::Minus), "Minus");
        assert_eq!(format!("{}", TokenType::Star), "Star");
        assert_eq!(format!("{}", TokenType::Slash), "Slash");
        assert_eq!(format!("{}", TokenType::Equals), "Equals");
        assert_eq!(format!("{}", TokenType::EqualsEquals), "EqualsEquals");
        assert_eq!(format!("{}", TokenType::LParen), "LParen");
        assert_eq!(format!("{}", TokenType::RParen), "RParen");
        assert_eq!(format!("{}", TokenType::Comma), "Comma");
        assert_eq!(format!("{}", TokenType::Colon), "Colon");
        assert_eq!(format!("{}", TokenType::Semicolon), "Semicolon");
        assert_eq!(format!("{}", TokenType::LBrace), "LBrace");
        assert_eq!(format!("{}", TokenType::RBrace), "RBrace");
        assert_eq!(format!("{}", TokenType::LBracket), "LBracket");
        assert_eq!(format!("{}", TokenType::RBracket), "RBracket");
        assert_eq!(format!("{}", TokenType::Dot), "Dot");
        assert_eq!(format!("{}", TokenType::Bang), "Bang");
        assert_eq!(format!("{}", TokenType::Newline), "Newline");
        assert_eq!(format!("{}", TokenType::Eof), "EOF");
    }

    // -----------------------------------------------------------------------
    // TokenType properties
    // -----------------------------------------------------------------------

    #[test]
    fn test_token_type_is_copy() {
        // TokenType is Copy, so assigning it should not move the original.
        let t = TokenType::Plus;
        let t2 = t;
        assert_eq!(t, t2);
    }

    #[test]
    fn test_token_type_equality() {
        // Same variants are equal, different variants are not.
        assert_eq!(TokenType::Name, TokenType::Name);
        assert_ne!(TokenType::Name, TokenType::Keyword);
        assert_ne!(TokenType::Equals, TokenType::EqualsEquals);
    }

    // -----------------------------------------------------------------------
    // Token Display
    // -----------------------------------------------------------------------

    #[test]
    fn test_token_display() {
        let tok = Token {
            type_: TokenType::Name,
            value: "x".to_string(),
            line: 1,
            column: 1,
        };
        assert_eq!(format!("{}", tok), "Token(Name, \"x\", 1:1)");
    }

    #[test]
    fn test_token_display_string_with_escape() {
        // String values that contain special characters should be
        // displayed with Rust's Debug formatting (escaped).
        let tok = Token {
            type_: TokenType::String,
            value: "hello\nworld".to_string(),
            line: 3,
            column: 5,
        };
        let display = format!("{}", tok);
        assert!(display.contains("String"));
        assert!(display.contains("3:5"));
    }

    // -----------------------------------------------------------------------
    // Token equality
    // -----------------------------------------------------------------------

    #[test]
    fn test_token_equality() {
        let a = Token {
            type_: TokenType::Number,
            value: "42".to_string(),
            line: 1,
            column: 1,
        };
        let b = Token {
            type_: TokenType::Number,
            value: "42".to_string(),
            line: 1,
            column: 1,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_token_inequality_type() {
        let a = Token {
            type_: TokenType::Number,
            value: "42".to_string(),
            line: 1,
            column: 1,
        };
        let b = Token {
            type_: TokenType::Name,
            value: "42".to_string(),
            line: 1,
            column: 1,
        };
        assert_ne!(a, b);
    }

    // -----------------------------------------------------------------------
    // LexerError
    // -----------------------------------------------------------------------

    #[test]
    fn test_lexer_error_display() {
        let err = LexerError {
            message: "Unexpected character '@'".to_string(),
            line: 5,
            column: 12,
        };
        assert_eq!(
            format!("{}", err),
            "LexerError at 5:12: Unexpected character '@'"
        );
    }

    #[test]
    fn test_lexer_error_is_error_trait() {
        // LexerError implements std::error::Error, so it can be used with
        // Result and the ? operator.
        let err: Box<dyn std::error::Error> = Box::new(LexerError {
            message: "test".to_string(),
            line: 1,
            column: 1,
        });
        assert!(err.to_string().contains("test"));
    }

    // -----------------------------------------------------------------------
    // Token clone
    // -----------------------------------------------------------------------

    #[test]
    fn test_token_clone() {
        let original = Token {
            type_: TokenType::Keyword,
            value: "if".to_string(),
            line: 10,
            column: 5,
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // -----------------------------------------------------------------------
    // string_to_token_type
    // -----------------------------------------------------------------------

    #[test]
    fn test_string_to_token_type_known_names() {
        assert_eq!(string_to_token_type("NAME"), TokenType::Name);
        assert_eq!(string_to_token_type("NUMBER"), TokenType::Number);
        assert_eq!(string_to_token_type("STRING"), TokenType::String);
        assert_eq!(string_to_token_type("KEYWORD"), TokenType::Keyword);
        assert_eq!(string_to_token_type("PLUS"), TokenType::Plus);
        assert_eq!(string_to_token_type("EQUALS_EQUALS"), TokenType::EqualsEquals);
        assert_eq!(string_to_token_type("NEWLINE"), TokenType::Newline);
        assert_eq!(string_to_token_type("EOF"), TokenType::Eof);
    }

    #[test]
    fn test_string_to_token_type_unknown_defaults_to_name() {
        assert_eq!(string_to_token_type("UNKNOWN"), TokenType::Name);
        assert_eq!(string_to_token_type("FOOBAR"), TokenType::Name);
    }
}
