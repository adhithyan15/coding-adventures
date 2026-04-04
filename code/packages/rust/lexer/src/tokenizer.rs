//! # Hand-written tokenizer — a character-by-character Python lexer.
//!
//! This module implements a lexer the old-fashioned way: by reading source
//! code one character at a time and deciding what to do based on what we see.
//! This is called a **hand-written lexer** (or "hand-rolled lexer") as
//! opposed to a grammar-driven lexer that reads its rules from a file.
//!
//! # Why hand-write a lexer?
//!
//! Grammar-driven lexers are powerful and flexible, but a hand-written lexer
//! has advantages:
//!
//! - **No dependencies** — it does not need a grammar file or regex engine.
//! - **Full control** — tricky cases like string escapes and multi-character
//!   operators (`==` vs `=`) are handled with explicit, readable logic.
//! - **Better error messages** — because we know exactly where we are in
//!   the source, we can produce precise, context-aware error messages.
//! - **Educational value** — understanding how a lexer works character-by-
//!   character is fundamental to understanding compilers.
//!
//! # How it works
//!
//! The tokenizer maintains a **cursor** that walks through the source string.
//! At each step, it looks at the current character and decides:
//!
//! ```text
//! Character       Action
//! -----------     ------
//! space/tab/CR    Skip (whitespace is not significant between tokens)
//! newline         Emit a NEWLINE token (significant in Python)
//! digit           Read a full number (one or more digits)
//! letter or _     Read a full name (letters, digits, underscores)
//! double quote    Read a string literal (handling escape sequences)
//! '='             Look ahead: '==' or '='
//! simple char     Emit the corresponding single-character token
//! anything else   Error: unexpected character
//! ```
//!
//! This decision process is the core of every hand-written lexer. The
//! complexity comes from the details: how do you handle escape sequences
//! in strings? How do you distinguish `=` from `==`? How do you track
//! line and column numbers accurately?
//!
//! # The simple-tokens table
//!
//! Most single-character tokens follow a trivial pattern: see the character,
//! emit the token. Rather than writing a separate `if` branch for each one,
//! we use a lookup table that maps characters to their token types. This is
//! both more compact and easier to extend.

use std::collections::HashSet;

use crate::token::{LexerError, Token, TokenType};
use crate::tokenizer_dfa::{classify_char, new_tokenizer_dfa};

// ===========================================================================
// Simple token lookup table
// ===========================================================================

/// Map a single character to its token type, if it is a "simple" token.
///
/// Simple tokens are single-character operators and delimiters that don't
/// require any lookahead or complex logic. The `=` character is *not*
/// in this table because it requires lookahead to distinguish `=` from `==`.
///
/// # Design note
///
/// In Go, this was a `map[rune]TokenType`. In Rust, we use a match
/// expression instead of a HashMap because:
///
/// 1. The set of characters is small and fixed at compile time.
/// 2. A match compiles to a jump table, which is faster than a hash lookup.
/// 3. No heap allocation needed.
fn simple_token_type(ch: char) -> Option<TokenType> {
    match ch {
        '+' => Some(TokenType::Plus),
        '-' => Some(TokenType::Minus),
        '*' => Some(TokenType::Star),
        '/' => Some(TokenType::Slash),
        '(' => Some(TokenType::LParen),
        ')' => Some(TokenType::RParen),
        ',' => Some(TokenType::Comma),
        ':' => Some(TokenType::Colon),
        ';' => Some(TokenType::Semicolon),
        '{' => Some(TokenType::LBrace),
        '}' => Some(TokenType::RBrace),
        '[' => Some(TokenType::LBracket),
        ']' => Some(TokenType::RBracket),
        '.' => Some(TokenType::Dot),
        '!' => Some(TokenType::Bang),
        _ => None,
    }
}

// ===========================================================================
// Lexer configuration
// ===========================================================================

/// Configuration for the hand-written lexer.
///
/// The only configurable aspect is the set of **keywords** — identifiers
/// that should be recognized as `KEYWORD` tokens instead of `NAME` tokens.
///
/// # Why is this configurable?
///
/// Different languages have different reserved words. Python reserves `if`,
/// `else`, `while`; JavaScript reserves `function`, `var`, `let`. By making
/// the keyword list configurable, the same lexer can tokenize different
/// languages (as long as they share the same basic syntax structure).
///
/// # Example
///
/// ```
/// use lexer::tokenizer::LexerConfig;
///
/// let config = LexerConfig {
///     keywords: vec!["if".to_string(), "else".to_string(), "while".to_string()],
/// };
/// ```
#[derive(Debug, Clone)]
pub struct LexerConfig {
    /// The list of reserved words that should be classified as KEYWORD tokens.
    pub keywords: Vec<String>,
}

impl Default for LexerConfig {
    fn default() -> Self {
        LexerConfig {
            keywords: Vec::new(),
        }
    }
}

// ===========================================================================
// Lexer struct
// ===========================================================================

/// A hand-written lexer for Python-like source code.
///
/// The lexer holds a reference to the source string and maintains a cursor
/// position along with line/column tracking. It consumes characters one at
/// a time and produces a vector of tokens.
///
/// # Lifetime
///
/// The lexer borrows the source string for its lifetime — it does not
/// copy the source. Token values *are* owned strings, because they may
/// differ from the source (e.g., string escape processing).
///
/// # Usage
///
/// ```
/// use lexer::tokenizer::{Lexer, LexerConfig};
/// use lexer::token::TokenType;
///
/// let config = LexerConfig {
///     keywords: vec!["if".to_string(), "else".to_string()],
/// };
/// let mut lexer = Lexer::new("x = 1 + 2", Some(config));
/// let tokens = lexer.tokenize().unwrap();
///
/// assert_eq!(tokens[0].type_, TokenType::Name);
/// assert_eq!(tokens[0].value, "x");
/// ```
pub struct Lexer<'a> {
    /// The source code being tokenized (kept for potential future use
    /// in error messages that want to show the original source context).
    #[allow(dead_code)]
    source: &'a str,

    /// The source code as a vector of characters, for indexed access.
    /// We convert once upfront because Rust strings are UTF-8 encoded,
    /// and indexing into a UTF-8 string by byte position could split a
    /// multi-byte character. Working with a Vec<char> ensures each index
    /// corresponds to exactly one character.
    chars: Vec<char>,

    /// Current position in the chars vector.
    pos: usize,

    /// Current line number (1-based).
    line: usize,

    /// Current column number (1-based).
    column: usize,

    /// The set of keywords for fast O(1) lookup.
    keyword_set: HashSet<String>,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given source code.
    ///
    /// If no config is provided, the lexer uses an empty keyword set,
    /// which means all identifiers will be classified as NAME tokens.
    pub fn new(source: &'a str, config: Option<LexerConfig>) -> Self {
        let cfg = config.unwrap_or_default();
        let keyword_set: HashSet<String> = cfg.keywords.into_iter().collect();

        Lexer {
            source,
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            keyword_set,
        }
    }

    // -----------------------------------------------------------------------
    // Cursor operations
    // -----------------------------------------------------------------------

    /// Return the current character without advancing, or None if at EOF.
    fn current_char(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Advance the cursor by one character and return the consumed character.
    ///
    /// This method updates line and column tracking: a newline resets the
    /// column to 1 and increments the line; any other character increments
    /// the column.
    fn advance(&mut self) -> char {
        let ch = self.chars[self.pos];
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        ch
    }

    /// Skip whitespace characters (space, tab, carriage return).
    ///
    /// Newlines are NOT skipped here because in Python they are significant
    /// — they terminate statements. The main loop handles newlines separately.
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Token readers — each handles one category of token
    // -----------------------------------------------------------------------

    /// Read a number token: one or more consecutive digits.
    ///
    /// ```text
    /// Input:  "42 + 5"
    ///          ^^
    /// Result: Token(Number, "42", line, col)
    /// ```
    ///
    /// This is the simplest token reader. It just accumulates digits until
    /// it hits a non-digit character. A more complete lexer would also
    /// handle decimal points (`3.14`), exponents (`1e10`), hex (`0xFF`),
    /// and underscored literals (`1_000_000`).
    fn read_number(&mut self) -> Token {
        let start_line = self.line;
        let start_col = self.column;
        let mut value = String::new();

        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
                value.push(self.advance());
            } else {
                break;
            }
        }

        Token {
            type_: TokenType::Number,
            value,
            line: start_line,
            column: start_col,
            type_name: None, flags: None,
        }
    }

    /// Read a name (identifier) token: a letter or underscore followed by
    /// letters, digits, or underscores.
    ///
    /// ```text
    /// Input:  "my_var = 5"
    ///          ^^^^^^
    /// Result: Token(Name, "my_var", line, col)
    /// ```
    ///
    /// After reading the full name, we check if it is a keyword. If the
    /// name appears in the keyword set, we promote it from NAME to KEYWORD.
    /// This two-step approach (read as name, then check keywords) is simpler
    /// than trying to match keywords directly during scanning.
    fn read_name(&mut self) -> Token {
        let start_line = self.line;
        let start_col = self.column;
        let mut value = String::new();

        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' {
                value.push(self.advance());
            } else {
                break;
            }
        }

        // Keyword promotion: if this name is a reserved word, change its type.
        let type_ = if self.keyword_set.contains(&value) {
            TokenType::Keyword
        } else {
            TokenType::Name
        };

        Token {
            type_,
            value,
            line: start_line,
            column: start_col,
            type_name: None, flags: None,
        }
    }

    /// Read a string literal: everything between opening and closing `"`.
    ///
    /// ```text
    /// Input:  "Hello\nWorld"
    ///          ^^^^^^^^^^^^^
    /// Result: Token(String, "Hello\nWorld", line, col)
    /// ```
    ///
    /// String reading is the most complex token reader because of escape
    /// sequences. When we see a backslash, we don't take the next character
    /// literally — instead, we interpret the two-character sequence:
    ///
    /// ```text
    /// Escape    Meaning
    /// ------    -------
    /// \n        newline (line feed)
    /// \t        tab
    /// \\        literal backslash
    /// \"        literal double quote
    /// \x        x (any other character — passed through as-is)
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a `LexerError` if the string is not terminated (reaches EOF
    /// without a closing quote) or if a backslash appears at the very end
    /// of the input.
    fn read_string(&mut self) -> Result<Token, LexerError> {
        let start_line = self.line;
        let start_col = self.column;
        let mut value = String::new();

        // Consume the opening quote.
        self.advance();

        loop {
            match self.current_char() {
                None => {
                    // We reached EOF without finding a closing quote.
                    return Err(LexerError {
                        message: "Unterminated string literal".to_string(),
                        line: start_line,
                        column: start_col,
                    });
                }
                Some('"') => {
                    // Closing quote found — consume it and return.
                    self.advance();
                    break;
                }
                Some('\\') => {
                    // Escape sequence — consume the backslash and interpret
                    // the next character.
                    self.advance();
                    match self.current_char() {
                        None => {
                            return Err(LexerError {
                                message: "Unterminated string literal (ends with backslash)"
                                    .to_string(),
                                line: start_line,
                                column: start_col,
                            });
                        }
                        Some(escaped) => {
                            let resolved = match escaped {
                                'n' => '\n',
                                't' => '\t',
                                '\\' => '\\',
                                '"' => '"',
                                other => other,
                            };
                            value.push(resolved);
                            self.advance();
                        }
                    }
                }
                Some(ch) => {
                    // Regular character — add it to the string value.
                    value.push(ch);
                    self.advance();
                }
            }
        }

        Ok(Token {
            type_: TokenType::String,
            value,
            line: start_line,
            column: start_col,
            type_name: None, flags: None,
        })
    }

    // -----------------------------------------------------------------------
    // Main tokenization loop
    // -----------------------------------------------------------------------

    /// Tokenize the entire source code and return a vector of tokens.
    ///
    /// The returned vector always ends with an EOF token. If the source is
    /// empty, the result is a single EOF token.
    ///
    /// # Errors
    ///
    /// Returns a `LexerError` if the source contains:
    /// - An unterminated string literal
    /// - A character that does not match any token pattern
    ///
    /// # The main loop
    ///
    /// ```text
    /// while not at EOF:
    ///     skip whitespace (spaces, tabs, carriage returns)
    ///     look at current character:
    ///         newline  -> emit NEWLINE token
    ///         digit    -> read_number()
    ///         letter/_ -> read_name() (may promote to KEYWORD)
    ///         "        -> read_string() (handles escapes)
    ///         =        -> lookahead: == or =
    ///         simple   -> look up in simple-tokens table
    ///         other    -> error: unexpected character
    /// emit EOF token
    /// ```
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        let mut dfa = new_tokenizer_dfa();

        loop {
            let ch = self.current_char();
            let char_class = classify_char(ch);
            let next_state = dfa
                .process(char_class)
                .expect("DFA transition should never fail for a valid char class");

            match next_state.as_str() {
                "at_whitespace" => {
                    self.skip_whitespace();
                }
                "at_newline" => {
                    let tok = Token {
                        type_: TokenType::Newline,
                        value: "\\n".to_string(),
                        line: self.line,
                        column: self.column,
                        type_name: None, flags: None,
                    };
                    self.advance();
                    tokens.push(tok);
                }
                "in_number" => {
                    tokens.push(self.read_number());
                }
                "in_name" => {
                    tokens.push(self.read_name());
                }
                "in_string" => {
                    tokens.push(self.read_string()?);
                }
                "in_equals" => {
                    let start_line = self.line;
                    let start_col = self.column;
                    self.advance();

                    if self.current_char() == Some('=') {
                        self.advance();
                        tokens.push(Token {
                            type_: TokenType::EqualsEquals,
                            value: "==".to_string(),
                            line: start_line,
                            column: start_col,
                            type_name: None, flags: None,
                        });
                    } else {
                        tokens.push(Token {
                            type_: TokenType::Equals,
                            value: "=".to_string(),
                            line: start_line,
                            column: start_col,
                            type_name: None, flags: None,
                        });
                    }
                }
                "in_operator" => {
                    let c = ch.expect("char must exist for in_operator");
                    let token_type =
                        simple_token_type(c).expect("char must be a simple token");
                    let tok = Token {
                        type_: token_type,
                        value: c.to_string(),
                        line: self.line,
                        column: self.column,
                        type_name: None, flags: None,
                    };
                    self.advance();
                    tokens.push(tok);
                }
                "done" => {
                    break;
                }
                "error" => {
                    let c = ch.unwrap_or('\0');
                    return Err(LexerError {
                        message: format!("Unexpected character {:?}", c),
                        line: self.line,
                        column: self.column,
                    });
                }
                _ => {
                    // Should never happen with a well-formed DFA.
                    unreachable!("Unexpected DFA state: {}", next_state);
                }
            }

            // Reset the DFA back to "start" for the next character.
            dfa.reset();
        }

        // Every token stream ends with EOF.
        tokens.push(Token {
            type_: TokenType::Eof,
            value: String::new(),
            line: self.line,
            column: self.column,
            type_name: None, flags: None,
        });

        Ok(tokens)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenType;

    // -----------------------------------------------------------------------
    // Helper: quick tokenize with no keywords
    // -----------------------------------------------------------------------

    fn tokenize(source: &str) -> Vec<Token> {
        Lexer::new(source, None).tokenize().unwrap()
    }

    fn tokenize_with_keywords(source: &str, keywords: Vec<&str>) -> Vec<Token> {
        let config = LexerConfig {
            keywords: keywords.into_iter().map(|s| s.to_string()).collect(),
        };
        Lexer::new(source, Some(config)).tokenize().unwrap()
    }

    // -----------------------------------------------------------------------
    // Basic arithmetic expression
    // -----------------------------------------------------------------------

    #[test]
    fn test_math_expression() {
        // The classic first test: a simple arithmetic expression.
        // "x = 1 + 2 * 3" should produce:
        //   NAME("x"), EQUALS("="), NUMBER("1"), PLUS("+"),
        //   NUMBER("2"), STAR("*"), NUMBER("3"), EOF
        let tokens = tokenize("x = 1 + 2 * 3");

        let expected = vec![
            (TokenType::Name, "x"),
            (TokenType::Equals, "="),
            (TokenType::Number, "1"),
            (TokenType::Plus, "+"),
            (TokenType::Number, "2"),
            (TokenType::Star, "*"),
            (TokenType::Number, "3"),
            (TokenType::Eof, ""),
        ];

        assert_eq!(tokens.len(), expected.len());
        for (i, (exp_type, exp_val)) in expected.iter().enumerate() {
            assert_eq!(tokens[i].type_, *exp_type, "token {} type mismatch", i);
            assert_eq!(tokens[i].value, *exp_val, "token {} value mismatch", i);
        }
    }

    // -----------------------------------------------------------------------
    // Keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_keywords() {
        // "if x == 5" with "if" as a keyword should produce:
        //   KEYWORD("if"), NAME("x"), EQUALS_EQUALS("=="), NUMBER("5"), EOF
        let tokens = tokenize_with_keywords("if x == 5", vec!["if"]);

        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "if");

        assert_eq!(tokens[1].type_, TokenType::Name);
        assert_eq!(tokens[1].value, "x");

        assert_eq!(tokens[2].type_, TokenType::EqualsEquals);
        assert_eq!(tokens[2].value, "==");

        assert_eq!(tokens[3].type_, TokenType::Number);
        assert_eq!(tokens[3].value, "5");
    }

    #[test]
    fn test_non_keyword_name() {
        // Without any keywords configured, "if" is just a NAME.
        let tokens = tokenize("if");
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "if");
    }

    // -----------------------------------------------------------------------
    // String literals
    // -----------------------------------------------------------------------

    #[test]
    fn test_string_literal() {
        // A simple string with no escapes.
        let tokens = tokenize(r#""hello""#);
        assert_eq!(tokens[0].type_, TokenType::String);
        assert_eq!(tokens[0].value, "hello");
    }

    #[test]
    fn test_string_escape_newline() {
        // The escape sequence \n should be converted to an actual newline.
        let tokens = tokenize(r#"print("Hello\n")"#);

        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "print");

        assert_eq!(tokens[2].type_, TokenType::String);
        assert_eq!(tokens[2].value, "Hello\n");
    }

    #[test]
    fn test_string_escape_tab() {
        let tokens = tokenize(r#""\t""#);
        assert_eq!(tokens[0].type_, TokenType::String);
        assert_eq!(tokens[0].value, "\t");
    }

    #[test]
    fn test_string_escape_backslash() {
        let tokens = tokenize(r#""\\""#);
        assert_eq!(tokens[0].type_, TokenType::String);
        assert_eq!(tokens[0].value, "\\");
    }

    #[test]
    fn test_string_escape_quote() {
        let tokens = tokenize(r#""\"""#);
        assert_eq!(tokens[0].type_, TokenType::String);
        assert_eq!(tokens[0].value, "\"");
    }

    #[test]
    fn test_unterminated_string() {
        // A string without a closing quote should produce an error.
        let result = Lexer::new(r#""hello"#, None).tokenize();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unterminated string"));
    }

    // -----------------------------------------------------------------------
    // All single-character tokens
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_simple_tokens() {
        // Every single-character token should be recognized.
        let tokens = tokenize("+ - * / ( ) , : ; { } [ ] . !");

        let expected_types = vec![
            TokenType::Plus,
            TokenType::Minus,
            TokenType::Star,
            TokenType::Slash,
            TokenType::LParen,
            TokenType::RParen,
            TokenType::Comma,
            TokenType::Colon,
            TokenType::Semicolon,
            TokenType::LBrace,
            TokenType::RBrace,
            TokenType::LBracket,
            TokenType::RBracket,
            TokenType::Dot,
            TokenType::Bang,
            TokenType::Eof,
        ];

        assert_eq!(tokens.len(), expected_types.len());
        for (i, exp_type) in expected_types.iter().enumerate() {
            assert_eq!(tokens[i].type_, *exp_type, "token {} type mismatch", i);
        }
    }

    // -----------------------------------------------------------------------
    // Equals vs EqualsEquals
    // -----------------------------------------------------------------------

    #[test]
    fn test_equals_single() {
        let tokens = tokenize("x = 5");
        assert_eq!(tokens[1].type_, TokenType::Equals);
        assert_eq!(tokens[1].value, "=");
    }

    #[test]
    fn test_equals_double() {
        let tokens = tokenize("x == 5");
        assert_eq!(tokens[1].type_, TokenType::EqualsEquals);
        assert_eq!(tokens[1].value, "==");
    }

    #[test]
    fn test_equals_at_end_of_input() {
        // A single `=` at the very end of input (no next character to peek).
        let tokens = tokenize("x =");
        assert_eq!(tokens[1].type_, TokenType::Equals);
        assert_eq!(tokens[1].value, "=");
    }

    // -----------------------------------------------------------------------
    // Newlines
    // -----------------------------------------------------------------------

    #[test]
    fn test_newline_tokens() {
        let tokens = tokenize("x\ny");
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[1].type_, TokenType::Newline);
        assert_eq!(tokens[1].value, "\\n");
        assert_eq!(tokens[2].type_, TokenType::Name);
    }

    // -----------------------------------------------------------------------
    // Line and column tracking
    // -----------------------------------------------------------------------

    #[test]
    fn test_position_tracking() {
        // "x = 1" should have tokens at:
        //   x at 1:1, = at 1:3, 1 at 1:5
        let tokens = tokenize("x = 1");

        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].column, 1);

        assert_eq!(tokens[1].line, 1);
        assert_eq!(tokens[1].column, 3);

        assert_eq!(tokens[2].line, 1);
        assert_eq!(tokens[2].column, 5);
    }

    #[test]
    fn test_multiline_position_tracking() {
        // After a newline, line increments and column resets.
        let tokens = tokenize("x\ny");

        // x is at line 1, column 1
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].column, 1);

        // \n is at line 1, column 2
        assert_eq!(tokens[1].line, 1);
        assert_eq!(tokens[1].column, 2);

        // y is at line 2, column 1
        assert_eq!(tokens[2].line, 2);
        assert_eq!(tokens[2].column, 1);
    }

    // -----------------------------------------------------------------------
    // Empty input
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_input() {
        // Empty source produces just an EOF token.
        let tokens = tokenize("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Unexpected character
    // -----------------------------------------------------------------------

    #[test]
    fn test_unexpected_character() {
        let result = Lexer::new("x @ y", None).tokenize();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unexpected character"));
        assert_eq!(err.line, 1);
        assert_eq!(err.column, 3);
    }

    // -----------------------------------------------------------------------
    // Underscore in identifiers
    // -----------------------------------------------------------------------

    #[test]
    fn test_underscore_identifier() {
        let tokens = tokenize("_private __init__ _x2");
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "_private");
        assert_eq!(tokens[1].type_, TokenType::Name);
        assert_eq!(tokens[1].value, "__init__");
        assert_eq!(tokens[2].type_, TokenType::Name);
        assert_eq!(tokens[2].value, "_x2");
    }

    // -----------------------------------------------------------------------
    // Complex expression
    // -----------------------------------------------------------------------

    #[test]
    fn test_function_call() {
        // A realistic Python-like expression: print("hello", 42)
        let tokens = tokenize(r#"print("hello", 42)"#);

        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "print");
        assert_eq!(tokens[1].type_, TokenType::LParen);
        assert_eq!(tokens[2].type_, TokenType::String);
        assert_eq!(tokens[2].value, "hello");
        assert_eq!(tokens[3].type_, TokenType::Comma);
        assert_eq!(tokens[4].type_, TokenType::Number);
        assert_eq!(tokens[4].value, "42");
        assert_eq!(tokens[5].type_, TokenType::RParen);
        assert_eq!(tokens[6].type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Multiple keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_multiple_keywords() {
        let tokens = tokenize_with_keywords(
            "if x == 5:\n    return y",
            vec!["if", "return"],
        );

        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "if");

        // Find the return token
        let return_tok = tokens.iter().find(|t| t.value == "return").unwrap();
        assert_eq!(return_tok.type_, TokenType::Keyword);
    }

    // -----------------------------------------------------------------------
    // Whitespace handling
    // -----------------------------------------------------------------------

    #[test]
    fn test_tabs_and_spaces() {
        // Tabs and multiple spaces should be skipped.
        let tokens = tokenize("x\t=\t\t1");
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[1].type_, TokenType::Equals);
        assert_eq!(tokens[2].type_, TokenType::Number);
    }

    #[test]
    fn test_carriage_return_ignored() {
        // Carriage returns (\r) should be silently skipped.
        let tokens = tokenize("x\r\n");
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[1].type_, TokenType::Newline);
    }
}
