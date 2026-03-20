//! # Grammar-driven lexer — tokenize any language from a grammar specification.
//!
//! The hand-written lexer in [`crate::tokenizer`] knows how to tokenize
//! Python-like code because its rules are baked into Rust source code. But
//! what if you want to tokenize a *different* language — say, SQL, or JSON,
//! or a custom DSL? You would need to write a whole new lexer.
//!
//! The grammar-driven lexer solves this problem. Instead of hard-coding
//! rules, it reads them from a [`TokenGrammar`] — a structured description
//! of all the tokens in a language, parsed from a `.tokens` file by the
//! [`grammar_tools`] crate.
//!
//! # How it works
//!
//! The grammar-driven lexer operates in two phases:
//!
//! ## Phase 1: Compilation (at construction time)
//!
//! When you create a `GrammarLexer`, it compiles each token definition
//! from the grammar into a regex pattern anchored to the start of the
//! string (`^`). Literal patterns are escaped so that special regex
//! characters like `+` and `*` are treated as literal characters.
//!
//! ```text
//! Grammar definition          Compiled regex
//! ------------------          --------------
//! NUMBER = /[0-9]+/     ->    ^[0-9]+
//! PLUS   = "+"          ->    ^\+
//! ```
//!
//! ## Phase 2: Tokenization (the main loop)
//!
//! The lexer walks through the source code. At each position, it tries
//! every compiled pattern in order. The **first pattern that matches wins**
//! (first-match-wins semantics). This is why the order of definitions in
//! the `.tokens` file matters — `==` must come before `=`, or `=` would
//! always match first and `==` would never be recognized.
//!
//! ```text
//! Source: "x == 42"
//! Pos 0: try NAME -> matches "x"          -> emit Token(Name, "x")
//! Pos 1: skip whitespace
//! Pos 2: try NAME -> no match
//!         try NUMBER -> no match
//!         try EQUALS_EQUALS -> matches "==" -> emit Token(EqualsEquals, "==")
//! Pos 4: skip whitespace
//! Pos 5: try NAME -> no match
//!         try NUMBER -> matches "42"       -> emit Token(Number, "42")
//! Pos 7: EOF -> emit Token(Eof, "")
//! ```
//!
//! # Keyword promotion
//!
//! When a token matches the `NAME` pattern and its value is in the grammar's
//! keyword list, the lexer promotes it from `NAME` to `KEYWORD`. This is
//! the same approach the hand-written lexer uses.
//!
//! # String escape processing
//!
//! When a token matches the `STRING` pattern, the lexer strips the
//! surrounding quotes and processes escape sequences (`\n`, `\t`, `\\`,
//! `\"`). This matches the behavior of the hand-written lexer.

use regex::Regex;
use std::collections::HashSet;

use grammar_tools::token_grammar::TokenGrammar;

use crate::token::{LexerError, Token, TokenType};

// ===========================================================================
// Compiled pattern — a pre-compiled regex ready for matching
// ===========================================================================

/// A single token pattern, compiled and ready to match against source text.
///
/// Each compiled pattern pairs a token name (from the grammar) with a
/// regex that is anchored to the start of the remaining input (`^`).
/// Anchoring is essential: we want to match at the *current position*,
/// not anywhere later in the string.
struct CompiledPattern {
    /// The token name from the grammar (e.g., "NAME", "NUMBER", "PLUS").
    name: String,

    /// The compiled regex, anchored to the start of the string.
    pattern: Regex,
}

// ===========================================================================
// GrammarLexer
// ===========================================================================

/// A lexer that tokenizes source code according to a [`TokenGrammar`].
///
/// The grammar defines the token patterns and their priority order. The
/// lexer compiles these patterns once at construction time, then uses
/// them to tokenize any number of source strings.
///
/// # Example
///
/// ```
/// use grammar_tools::token_grammar::parse_token_grammar;
/// use lexer::grammar_lexer::GrammarLexer;
/// use lexer::token::TokenType;
///
/// // Define a simple grammar with numbers and plus signs.
/// let grammar = parse_token_grammar(r#"
/// NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
/// NUMBER = /[0-9]+/
/// PLUS   = "+"
/// EQUALS = "="
/// "#).unwrap();
///
/// let tokens = GrammarLexer::new("x = 1 + 2", &grammar).tokenize().unwrap();
///
/// assert_eq!(tokens[0].type_, TokenType::Name);
/// assert_eq!(tokens[0].value, "x");
/// assert_eq!(tokens[2].type_, TokenType::Number);
/// assert_eq!(tokens[2].value, "1");
/// ```
pub struct GrammarLexer<'a> {
    /// The source code being tokenized, as a vector of characters.
    /// We convert from &str to Vec<char> so we can index by character
    /// position without worrying about multi-byte UTF-8 sequences.
    chars: Vec<char>,

    /// The original source string (needed for regex matching, since
    /// regex operates on string slices, not character arrays).
    source: &'a str,

    /// Current position in the source string (byte offset).
    /// We track byte position for slicing the source string for regex,
    /// and character position separately for line/column tracking.
    byte_pos: usize,

    /// Current character position (for line/column tracking).
    char_pos: usize,

    /// Current line number (1-based).
    line: usize,

    /// Current column number (1-based).
    column: usize,

    /// The set of keywords for fast O(1) lookup.
    keyword_set: HashSet<String>,

    /// Pre-compiled patterns from the grammar, in priority order.
    patterns: Vec<CompiledPattern>,
}

impl<'a> GrammarLexer<'a> {
    /// Create a new grammar-driven lexer for the given source code.
    ///
    /// This constructor compiles all token patterns from the grammar into
    /// anchored regexes. If any pattern fails to compile, this function
    /// panics — that is a bug in the grammar, not in the source being
    /// tokenized.
    ///
    /// # Panics
    ///
    /// Panics if a regex pattern from the grammar cannot be compiled.
    /// This should be caught earlier by [`grammar_tools::token_grammar::validate_token_grammar`].
    pub fn new(source: &'a str, grammar: &TokenGrammar) -> Self {
        // Build the keyword set for O(1) lookup.
        let keyword_set: HashSet<String> = grammar.keywords.iter().cloned().collect();

        // Compile each token definition into an anchored regex.
        let patterns: Vec<CompiledPattern> = grammar
            .definitions
            .iter()
            .map(|defn| {
                // Anchor the pattern to the start of the string.
                // For regex patterns, prepend `^`.
                // For literal patterns, escape special regex characters first.
                let regex_str = if defn.is_regex {
                    format!("^{}", defn.pattern)
                } else {
                    format!("^{}", regex::escape(&defn.pattern))
                };

                let compiled = Regex::new(&regex_str).unwrap_or_else(|e| {
                    panic!(
                        "Failed to compile pattern for token {}: {}",
                        defn.name, e
                    )
                });

                CompiledPattern {
                    name: defn.name.clone(),
                    pattern: compiled,
                }
            })
            .collect();

        GrammarLexer {
            chars: source.chars().collect(),
            source,
            byte_pos: 0,
            char_pos: 0,
            line: 1,
            column: 1,
            keyword_set,
            patterns,
        }
    }

    // -----------------------------------------------------------------------
    // Cursor operations
    // -----------------------------------------------------------------------

    /// Advance the cursor by one character, updating line/column tracking.
    fn advance(&mut self) {
        if self.char_pos < self.chars.len() {
            let ch = self.chars[self.char_pos];
            self.byte_pos += ch.len_utf8();
            self.char_pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }

    /// Advance the cursor by `n` characters.
    fn advance_n(&mut self, n: usize) {
        for _ in 0..n {
            self.advance();
        }
    }

    // -----------------------------------------------------------------------
    // Token type resolution
    // -----------------------------------------------------------------------

    /// Map a grammar token name to a [`TokenType`] enum variant.
    ///
    /// The grammar uses string names like "NAME", "NUMBER", "PLUS".
    /// This function converts them to our enum. If a NAME token's value
    /// appears in the keyword set, it is promoted to KEYWORD.
    ///
    /// Unrecognized token names default to `TokenType::Name`. This is
    /// lenient — a stricter approach would return an error.
    fn resolve_token_type(&self, token_name: &str, value: &str) -> TokenType {
        // Keyword promotion: if the grammar token is NAME and the value
        // is a reserved word, promote to KEYWORD.
        if token_name == "NAME" {
            if self.keyword_set.contains(value) {
                return TokenType::Keyword;
            }
        }

        match token_name {
            "NAME" => TokenType::Name,
            "NUMBER" => TokenType::Number,
            "STRING" => TokenType::String,
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
            _ => TokenType::Name, // Default for unrecognized names.
        }
    }

    // -----------------------------------------------------------------------
    // Escape processing
    // -----------------------------------------------------------------------

    /// Process escape sequences in a string value.
    ///
    /// This converts two-character escape sequences like `\n` into their
    /// actual character equivalents. The input is the raw string content
    /// *without* the surrounding quotes.
    ///
    /// ```text
    /// Input:    hello\nworld
    /// Output:   hello
    ///           world
    /// ```
    fn process_escapes(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                let next = chars[i + 1];
                match next {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    other => result.push(other),
                }
                i += 2;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }

        result
    }

    // -----------------------------------------------------------------------
    // Main tokenization loop
    // -----------------------------------------------------------------------

    /// Tokenize the source code according to the grammar's patterns.
    ///
    /// Returns a vector of tokens ending with EOF, or a `LexerError` if
    /// the source contains a character sequence that does not match any
    /// pattern.
    ///
    /// # Algorithm
    ///
    /// ```text
    /// while not at EOF:
    ///     if current char is whitespace (space/tab/CR): skip it
    ///     if current char is newline: emit NEWLINE token
    ///     otherwise:
    ///         try each compiled pattern against remaining input
    ///         if one matches:
    ///             emit the corresponding token
    ///             advance past the matched text
    ///         if none match:
    ///             return error
    /// emit EOF
    /// ```
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while self.char_pos < self.chars.len() {
            let ch = self.chars[self.char_pos];

            // --- Skip whitespace (not newlines) ---
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
                continue;
            }

            // --- Newlines are significant ---
            if ch == '\n' {
                tokens.push(Token {
                    type_: TokenType::Newline,
                    value: "\\n".to_string(),
                    line: self.line,
                    column: self.column,
                });
                self.advance();
                continue;
            }

            // --- Try each pattern against the remaining input ---
            let remaining = &self.source[self.byte_pos..];
            let mut matched = false;

            for p in &self.patterns {
                if let Some(m) = p.pattern.find(remaining) {
                    let value = m.as_str();
                    let start_line = self.line;
                    let start_col = self.column;

                    // Resolve the token type from the grammar name.
                    let token_type = self.resolve_token_type(&p.name, value);

                    // For STRING tokens, strip quotes and process escapes.
                    let final_value = if p.name == "STRING" && value.len() >= 2 {
                        let inner = &value[1..value.len() - 1];
                        Self::process_escapes(inner)
                    } else {
                        value.to_string()
                    };

                    tokens.push(Token {
                        type_: token_type,
                        value: final_value,
                        line: start_line,
                        column: start_col,
                    });

                    // Advance past the matched text.
                    // Count the number of characters in the matched text.
                    let char_count = value.chars().count();
                    self.advance_n(char_count);

                    matched = true;
                    break;
                }
            }

            if !matched {
                return Err(LexerError {
                    message: format!("Unexpected sequence {:?}", ch),
                    line: self.line,
                    column: self.column,
                });
            }
        }

        // Always end with EOF.
        tokens.push(Token {
            type_: TokenType::Eof,
            value: String::new(),
            line: self.line,
            column: self.column,
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
    use grammar_tools::token_grammar::parse_token_grammar;

    // -----------------------------------------------------------------------
    // Helper: build a Python-like grammar for testing
    // -----------------------------------------------------------------------

    /// A minimal Python-like token grammar for testing.
    ///
    /// This grammar defines the same tokens that the hand-written lexer
    /// supports, so we can verify that both lexers produce identical output
    /// for the same input.
    fn python_grammar() -> TokenGrammar {
        parse_token_grammar(
            r#"
# Token definitions for a subset of Python
NAME          = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER        = /[0-9]+/
STRING        = /"([^"\\]|\\.)*"/
EQUALS_EQUALS = "=="
EQUALS        = "="
PLUS          = "+"
MINUS         = "-"
STAR          = "*"
SLASH         = "/"
LPAREN        = "("
RPAREN        = ")"
COMMA         = ","
COLON         = ":"
SEMICOLON     = ";"
LBRACE        = "{"
RBRACE        = "}"
LBRACKET      = "["
RBRACKET      = "]"
DOT           = "."
BANG          = "!"
keywords:
  if
  else
  while
  def
  return
"#,
        )
        .unwrap()
    }

    fn tokenize(source: &str) -> Vec<Token> {
        let grammar = python_grammar();
        GrammarLexer::new(source, &grammar).tokenize().unwrap()
    }

    // -----------------------------------------------------------------------
    // Basic arithmetic
    // -----------------------------------------------------------------------

    #[test]
    fn test_math_expression() {
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
    fn test_keyword_promotion() {
        // "if" should be promoted from NAME to KEYWORD because it is
        // listed in the grammar's keywords section.
        let tokens = tokenize("if x == 5");

        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "if");
        assert_eq!(tokens[2].type_, TokenType::EqualsEquals);
        assert_eq!(tokens[2].value, "==");
    }

    #[test]
    fn test_non_keyword_is_name() {
        // "foo" is not in the keyword list, so it stays as NAME.
        let tokens = tokenize("foo");
        assert_eq!(tokens[0].type_, TokenType::Name);
    }

    // -----------------------------------------------------------------------
    // String literals
    // -----------------------------------------------------------------------

    #[test]
    fn test_string_literal() {
        let tokens = tokenize(r#""hello""#);
        assert_eq!(tokens[0].type_, TokenType::String);
        assert_eq!(tokens[0].value, "hello");
    }

    #[test]
    fn test_string_escape_newline() {
        let tokens = tokenize(r#"print("Hello\n")"#);
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

    // -----------------------------------------------------------------------
    // All single-character tokens
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_operators_and_delimiters() {
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
    // Equals vs EqualsEquals (first-match-wins ordering)
    // -----------------------------------------------------------------------

    #[test]
    fn test_equals_single() {
        let tokens = tokenize("x = 5");
        assert_eq!(tokens[1].type_, TokenType::Equals);
    }

    #[test]
    fn test_equals_double() {
        // EQUALS_EQUALS is defined *before* EQUALS in the grammar,
        // so "==" matches EQUALS_EQUALS rather than two EQUALS tokens.
        let tokens = tokenize("x == 5");
        assert_eq!(tokens[1].type_, TokenType::EqualsEquals);
        assert_eq!(tokens[1].value, "==");
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
    // Position tracking
    // -----------------------------------------------------------------------

    #[test]
    fn test_position_tracking() {
        let tokens = tokenize("x = 1");

        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].column, 1);

        assert_eq!(tokens[1].line, 1);
        assert_eq!(tokens[1].column, 3);

        assert_eq!(tokens[2].line, 1);
        assert_eq!(tokens[2].column, 5);
    }

    #[test]
    fn test_multiline_position() {
        let tokens = tokenize("x\ny");

        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].column, 1);

        assert_eq!(tokens[2].line, 2);
        assert_eq!(tokens[2].column, 1);
    }

    // -----------------------------------------------------------------------
    // Empty input
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_input() {
        let tokens = tokenize("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Error: unexpected character
    // -----------------------------------------------------------------------

    #[test]
    fn test_unexpected_character() {
        let grammar = python_grammar();
        let result = GrammarLexer::new("x @ y", &grammar).tokenize();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unexpected"));
    }

    // -----------------------------------------------------------------------
    // Complex expression
    // -----------------------------------------------------------------------

    #[test]
    fn test_function_definition() {
        // A realistic Python-like function definition.
        let tokens = tokenize("def add(x, y):\n    return x + y");

        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "def");
        assert_eq!(tokens[1].type_, TokenType::Name);
        assert_eq!(tokens[1].value, "add");
        assert_eq!(tokens[2].type_, TokenType::LParen);

        // Find the return keyword
        let return_tok = tokens.iter().find(|t| t.value == "return").unwrap();
        assert_eq!(return_tok.type_, TokenType::Keyword);
    }

    // -----------------------------------------------------------------------
    // Process escapes (unit test for the helper)
    // -----------------------------------------------------------------------

    #[test]
    fn test_process_escapes_newline() {
        assert_eq!(GrammarLexer::process_escapes(r"hello\nworld"), "hello\nworld");
    }

    #[test]
    fn test_process_escapes_tab() {
        assert_eq!(GrammarLexer::process_escapes(r"a\tb"), "a\tb");
    }

    #[test]
    fn test_process_escapes_backslash() {
        assert_eq!(GrammarLexer::process_escapes(r"a\\b"), "a\\b");
    }

    #[test]
    fn test_process_escapes_quote() {
        assert_eq!(GrammarLexer::process_escapes(r#"a\"b"#), "a\"b");
    }

    #[test]
    fn test_process_escapes_unknown() {
        // Unknown escape sequences pass through the character after the backslash.
        assert_eq!(GrammarLexer::process_escapes(r"a\xb"), "axb");
    }

    #[test]
    fn test_process_escapes_no_escapes() {
        assert_eq!(GrammarLexer::process_escapes("plain text"), "plain text");
    }

    // -----------------------------------------------------------------------
    // Grammar with no keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_grammar_without_keywords() {
        // A grammar with no keywords section — all identifiers are NAME.
        let grammar = parse_token_grammar(
            "NAME = /[a-zA-Z_]+/\nNUMBER = /[0-9]+/\nPLUS = \"+\"\n",
        )
        .unwrap();

        let tokens = GrammarLexer::new("if 42", &grammar).tokenize().unwrap();
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "if");
        assert_eq!(tokens[1].type_, TokenType::Number);
        assert_eq!(tokens[1].value, "42");
    }

    // -----------------------------------------------------------------------
    // Consistency: grammar lexer matches hand-written lexer
    // -----------------------------------------------------------------------

    #[test]
    fn test_consistency_with_hand_written_lexer() {
        // Both lexers should produce the same token types and values
        // for the same input.
        use crate::tokenizer::Lexer;

        let source = "x = 1 + 2 * 3";

        // Hand-written lexer
        let hand_tokens = Lexer::new(source, None).tokenize().unwrap();

        // Grammar-driven lexer
        let grammar = python_grammar();
        let grammar_tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        assert_eq!(hand_tokens.len(), grammar_tokens.len());
        for (i, (h, g)) in hand_tokens.iter().zip(grammar_tokens.iter()).enumerate() {
            assert_eq!(h.type_, g.type_, "token {} type mismatch", i);
            assert_eq!(h.value, g.value, "token {} value mismatch", i);
        }
    }

    #[test]
    fn test_consistency_with_keywords() {
        use crate::tokenizer::{Lexer, LexerConfig};

        let source = "if x == 5";

        let config = LexerConfig {
            keywords: vec!["if".to_string()],
        };
        let hand_tokens = Lexer::new(source, Some(config)).tokenize().unwrap();

        let grammar = python_grammar();
        let grammar_tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        assert_eq!(hand_tokens.len(), grammar_tokens.len());
        for (i, (h, g)) in hand_tokens.iter().zip(grammar_tokens.iter()).enumerate() {
            assert_eq!(h.type_, g.type_, "token {} type mismatch", i);
            assert_eq!(h.value, g.value, "token {} value mismatch", i);
        }
    }

    #[test]
    fn test_consistency_with_strings() {
        use crate::tokenizer::Lexer;

        let source = r#"print("Hello\n")"#;

        let hand_tokens = Lexer::new(source, None).tokenize().unwrap();

        let grammar = python_grammar();
        let grammar_tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        assert_eq!(hand_tokens.len(), grammar_tokens.len());
        for (i, (h, g)) in hand_tokens.iter().zip(grammar_tokens.iter()).enumerate() {
            assert_eq!(h.type_, g.type_, "token {} type mismatch", i);
            assert_eq!(h.value, g.value, "token {} value mismatch", i);
        }
    }
}
