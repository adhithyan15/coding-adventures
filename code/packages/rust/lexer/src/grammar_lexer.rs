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
//! # Extensions for Starlark-like languages
//!
//! This lexer supports several extensions beyond basic grammar-driven tokenization:
//!
//! - **Skip patterns**: Patterns from the `skip:` section consume input without
//!   emitting tokens (e.g. whitespace, comments).
//!
//! - **Type aliases**: When a definition has `-> ALIAS`, the emitted token uses
//!   the alias name instead (e.g. `STRING_DQ -> STRING`).
//!
//! - **Reserved keywords**: Keywords from the `reserved:` section cause a lexer
//!   error if encountered in source code.
//!
//! - **Indentation mode**: When `mode: indentation` is set, the lexer tracks
//!   indentation levels and emits synthetic INDENT/DEDENT/NEWLINE tokens,
//!   following the Python/Starlark whitespace rules.

use regex::Regex;
use std::collections::HashSet;

use grammar_tools::token_grammar::{TokenGrammar, TokenDefinition};

use crate::token::{LexerError, Token, TokenType, string_to_token_type};

// ===========================================================================
// Compiled pattern — a pre-compiled regex ready for matching
// ===========================================================================

/// A single token pattern, compiled and ready to match against source text.
struct CompiledPattern {
    /// The token name from the grammar (e.g., "NAME", "NUMBER", "PLUS").
    name: String,

    /// The compiled regex, anchored to the start of the string.
    pattern: Regex,

    /// Optional alias — when set, tokens matching this pattern are emitted
    /// with the alias as their type name instead of `name`.
    alias: Option<String>,
}

/// Compile a list of token definitions into anchored regex patterns.
fn compile_patterns(definitions: &[TokenDefinition]) -> Vec<CompiledPattern> {
    definitions
        .iter()
        .map(|defn| {
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
                alias: defn.alias.clone(),
            }
        })
        .collect()
}

// ===========================================================================
// GrammarLexer
// ===========================================================================

/// A lexer that tokenizes source code according to a [`TokenGrammar`].
///
/// Supports standard mode (simple pattern matching with whitespace skipping)
/// and indentation mode (Python/Starlark-style significant whitespace with
/// synthetic INDENT/DEDENT/NEWLINE tokens).
pub struct GrammarLexer<'a> {
    /// The source code as character vector for indexed access.
    chars: Vec<char>,

    /// The original source string (for regex matching on slices).
    source: &'a str,

    /// Current byte position in the source string.
    byte_pos: usize,

    /// Current character position.
    char_pos: usize,

    /// Current line number (1-based).
    line: usize,

    /// Current column number (1-based).
    column: usize,

    /// Keywords for keyword promotion (NAME -> KEYWORD).
    keyword_set: HashSet<String>,

    /// Reserved keywords that cause errors if encountered.
    reserved_set: HashSet<String>,

    /// Pre-compiled token patterns in priority order.
    patterns: Vec<CompiledPattern>,

    /// Pre-compiled skip patterns (whitespace, comments).
    skip_patterns: Vec<CompiledPattern>,

    /// Whether indentation mode is active.
    indent_mode: bool,
}

impl<'a> GrammarLexer<'a> {
    /// Create a new grammar-driven lexer for the given source code.
    pub fn new(source: &'a str, grammar: &TokenGrammar) -> Self {
        let keyword_set: HashSet<String> = grammar.keywords.iter().cloned().collect();
        let reserved_set: HashSet<String> = grammar.reserved_keywords.iter().cloned().collect();
        let patterns = compile_patterns(&grammar.definitions);
        let skip_patterns = compile_patterns(&grammar.skip_definitions);
        let indent_mode = grammar.mode.as_deref() == Some("indentation");

        GrammarLexer {
            chars: source.chars().collect(),
            source,
            byte_pos: 0,
            char_pos: 0,
            line: 1,
            column: 1,
            keyword_set,
            reserved_set,
            patterns,
            skip_patterns,
            indent_mode,
        }
    }

    // -----------------------------------------------------------------------
    // Cursor operations
    // -----------------------------------------------------------------------

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

    fn advance_n(&mut self, n: usize) {
        for _ in 0..n {
            self.advance();
        }
    }

    #[allow(dead_code)]
    fn current_char(&self) -> Option<char> {
        self.chars.get(self.char_pos).copied()
    }

    // -----------------------------------------------------------------------
    // Token type resolution
    // -----------------------------------------------------------------------

    /// Resolve a grammar token name to a TokenType and optional string type name.
    ///
    /// Handles keyword promotion, reserved keyword rejection, alias resolution,
    /// and fallback to string-based type names for custom token types.
    fn resolve_token_type(
        &self,
        token_name: &str,
        alias: Option<&str>,
        value: &str,
    ) -> Result<(TokenType, Option<String>), LexerError> {
        // Check reserved keywords first — if a NAME matches a reserved word,
        // that's an error.
        if token_name == "NAME" && self.reserved_set.contains(value) {
            return Err(LexerError {
                message: format!("Reserved keyword '{}' cannot be used as an identifier", value),
                line: self.line,
                column: self.column,
            });
        }

        // Keyword promotion: NAME tokens whose value is in the keyword set.
        if token_name == "NAME" && self.keyword_set.contains(value) {
            return Ok((TokenType::Keyword, Some("KEYWORD".to_string())));
        }

        // Determine the effective type name (alias takes precedence).
        let effective_name = alias.unwrap_or(token_name);

        // Try to map to a known TokenType enum variant.
        let token_type = string_to_token_type(effective_name);

        // If string_to_token_type returned Name but the effective name is not
        // "NAME", it means we have a custom type — store it as type_name.
        if token_type == TokenType::Name && effective_name != "NAME" {
            Ok((token_type, Some(effective_name.to_string())))
        } else {
            Ok((token_type, None))
        }
    }

    // -----------------------------------------------------------------------
    // Escape processing
    // -----------------------------------------------------------------------

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
    // Skip pattern matching
    // -----------------------------------------------------------------------

    /// Try to match and consume a skip pattern at the current position.
    /// Returns true if something was skipped.
    fn try_skip(&mut self) -> bool {
        let remaining = &self.source[self.byte_pos..];
        for p in &self.skip_patterns {
            if let Some(m) = p.pattern.find(remaining) {
                let char_count = m.as_str().chars().count();
                self.advance_n(char_count);
                return true;
            }
        }
        false
    }

    /// Try to match a token pattern at the current position.
    /// Returns (name, alias, matched_text) or None.
    fn try_match_token(&self) -> Option<(String, Option<String>, String)> {
        let remaining = &self.source[self.byte_pos..];
        for p in &self.patterns {
            if let Some(m) = p.pattern.find(remaining) {
                return Some((
                    p.name.clone(),
                    p.alias.clone(),
                    m.as_str().to_string(),
                ));
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // Main tokenization — standard mode
    // -----------------------------------------------------------------------

    fn tokenize_standard(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while self.char_pos < self.chars.len() {
            let ch = self.chars[self.char_pos];

            // --- Skip whitespace (not newlines) in standard mode ---
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
                continue;
            }

            // --- Try skip patterns ---
            if self.try_skip() {
                continue;
            }

            // --- Newlines ---
            if ch == '\n' {
                tokens.push(Token {
                    type_: TokenType::Newline,
                    value: "\\n".to_string(),
                    line: self.line,
                    column: self.column,
                    type_name: None,
                });
                self.advance();
                continue;
            }

            // --- Try each pattern ---
            if let Some((name, alias, matched)) = self.try_match_token() {
                let start_line = self.line;
                let start_col = self.column;

                let (token_type, type_name) = self.resolve_token_type(
                    &name,
                    alias.as_deref(),
                    &matched,
                )?;

                // For STRING tokens, strip quotes and process escapes.
                let effective_name = alias.as_deref().unwrap_or(&name);
                let final_value = if effective_name == "STRING" && matched.len() >= 2 {
                    let inner = &matched[1..matched.len() - 1];
                    Self::process_escapes(inner)
                } else {
                    matched.clone()
                };

                tokens.push(Token {
                    type_: token_type,
                    value: final_value,
                    line: start_line,
                    column: start_col,
                    type_name,
                });

                let char_count = matched.chars().count();
                self.advance_n(char_count);
                continue;
            }

            return Err(LexerError {
                message: format!("Unexpected sequence {:?}", ch),
                line: self.line,
                column: self.column,
            });
        }

        tokens.push(Token {
            type_: TokenType::Eof,
            value: String::new(),
            line: self.line,
            column: self.column,
            type_name: None,
        });

        Ok(tokens)
    }

    // -----------------------------------------------------------------------
    // Main tokenization — indentation mode
    // -----------------------------------------------------------------------

    /// Tokenize with indentation tracking (Python/Starlark style).
    ///
    /// In indentation mode, the lexer:
    /// - Tracks an indent stack (starts at [0])
    /// - At each logical line start, counts leading spaces and emits
    ///   INDENT/DEDENT tokens as needed
    /// - Suppresses NEWLINE/INDENT/DEDENT inside brackets
    /// - Skips blank lines and comment-only lines
    /// - Rejects tabs in leading indentation
    fn tokenize_indentation(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        let mut indent_stack: Vec<usize> = vec![0];
        let mut bracket_depth: usize = 0;
        let mut at_line_start = true;

        while self.char_pos < self.chars.len() {
            // --- Line start: handle indentation ---
            if at_line_start && bracket_depth == 0 {
                at_line_start = false;

                // Count leading spaces (reject tabs).
                let mut spaces = 0;
                while self.char_pos < self.chars.len() {
                    let ch = self.chars[self.char_pos];
                    if ch == ' ' {
                        spaces += 1;
                        self.advance();
                    } else if ch == '\t' {
                        return Err(LexerError {
                            message: "Tabs are not allowed in indentation (use spaces)".to_string(),
                            line: self.line,
                            column: self.column,
                        });
                    } else {
                        break;
                    }
                }

                // Check for blank line or comment-only line.
                let is_blank_or_comment = if self.char_pos >= self.chars.len() {
                    true
                } else {
                    let ch = self.chars[self.char_pos];
                    ch == '\n' || ch == '\r' || ch == '#'
                };

                // Handle blank/comment lines — skip them without emitting
                // NEWLINE, but consume through the end of line.
                if is_blank_or_comment {
                    // Try skip patterns (for comments).
                    self.try_skip();
                    // Consume the newline if present.
                    if self.char_pos < self.chars.len() {
                        let ch = self.chars[self.char_pos];
                        if ch == '\n' {
                            self.advance();
                        } else if ch == '\r' {
                            self.advance();
                            if self.char_pos < self.chars.len() && self.chars[self.char_pos] == '\n' {
                                self.advance();
                            }
                        }
                    }
                    at_line_start = true;
                    continue;
                }

                // Compare indentation with the current stack top.
                let current_indent = *indent_stack.last().unwrap();
                let indent_line = self.line;
                let indent_col = self.column;

                if spaces > current_indent {
                    indent_stack.push(spaces);
                    tokens.push(Token {
                        type_: TokenType::Indent,
                        value: String::new(),
                        line: indent_line,
                        column: indent_col,
                        type_name: None,
                    });
                } else if spaces < current_indent {
                    // Emit DEDENT for each level we're leaving.
                    while indent_stack.len() > 1 && *indent_stack.last().unwrap() > spaces {
                        indent_stack.pop();
                        tokens.push(Token {
                            type_: TokenType::Dedent,
                            value: String::new(),
                            line: indent_line,
                            column: indent_col,
                            type_name: None,
                        });
                    }
                    // Check that we landed on a valid indentation level.
                    if *indent_stack.last().unwrap() != spaces {
                        return Err(LexerError {
                            message: "Indentation does not match any outer level".to_string(),
                            line: indent_line,
                            column: indent_col,
                        });
                    }
                }
                // If spaces == current_indent, no INDENT/DEDENT needed.

                continue;
            }

            let ch = self.chars[self.char_pos];

            // --- Skip whitespace (not newlines) ---
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
                continue;
            }

            // --- Try skip patterns ---
            if self.try_skip() {
                continue;
            }

            // --- Newlines ---
            if ch == '\n' {
                if bracket_depth == 0 {
                    tokens.push(Token {
                        type_: TokenType::Newline,
                        value: "\\n".to_string(),
                        line: self.line,
                        column: self.column,
                        type_name: None,
                    });
                    at_line_start = true;
                }
                self.advance();
                continue;
            }

            // --- Try each pattern ---
            if let Some((name, alias, matched)) = self.try_match_token() {
                let start_line = self.line;
                let start_col = self.column;

                let (token_type, type_name) = self.resolve_token_type(
                    &name,
                    alias.as_deref(),
                    &matched,
                )?;

                // Track bracket depth.
                match matched.as_str() {
                    "(" | "[" | "{" => bracket_depth += 1,
                    ")" | "]" | "}" => {
                        if bracket_depth > 0 {
                            bracket_depth -= 1;
                        }
                    }
                    _ => {}
                }

                let effective_name = alias.as_deref().unwrap_or(&name);
                let final_value = if effective_name == "STRING" && matched.len() >= 2 {
                    let inner = &matched[1..matched.len() - 1];
                    Self::process_escapes(inner)
                } else {
                    matched.clone()
                };

                tokens.push(Token {
                    type_: token_type,
                    value: final_value,
                    line: start_line,
                    column: start_col,
                    type_name,
                });

                let char_count = matched.chars().count();
                self.advance_n(char_count);
                continue;
            }

            return Err(LexerError {
                message: format!("Unexpected sequence {:?}", ch),
                line: self.line,
                column: self.column,
            });
        }

        // At EOF: emit remaining DEDENTs.
        if bracket_depth == 0 {
            // Emit a final NEWLINE if the last token isn't one.
            let need_newline = tokens.last().map_or(false, |t| t.type_ != TokenType::Newline);
            if need_newline {
                tokens.push(Token {
                    type_: TokenType::Newline,
                    value: "\\n".to_string(),
                    line: self.line,
                    column: self.column,
                    type_name: None,
                });
            }

            while indent_stack.len() > 1 {
                indent_stack.pop();
                tokens.push(Token {
                    type_: TokenType::Dedent,
                    value: String::new(),
                    line: self.line,
                    column: self.column,
                    type_name: None,
                });
            }
        }

        tokens.push(Token {
            type_: TokenType::Eof,
            value: String::new(),
            line: self.line,
            column: self.column,
            type_name: None,
        });

        Ok(tokens)
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Tokenize the source code according to the grammar's patterns.
    ///
    /// Dispatches to either standard or indentation mode based on the
    /// grammar's mode directive.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        if self.indent_mode {
            self.tokenize_indentation()
        } else {
            self.tokenize_standard()
        }
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
        let tokens = tokenize("if x == 5");
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "if");
        assert_eq!(tokens[2].type_, TokenType::EqualsEquals);
    }

    #[test]
    fn test_non_keyword_is_name() {
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
    // Equals vs EqualsEquals
    // -----------------------------------------------------------------------

    #[test]
    fn test_equals_single() {
        let tokens = tokenize("x = 5");
        assert_eq!(tokens[1].type_, TokenType::Equals);
    }

    #[test]
    fn test_equals_double() {
        let tokens = tokenize("x == 5");
        assert_eq!(tokens[1].type_, TokenType::EqualsEquals);
    }

    // -----------------------------------------------------------------------
    // Newlines
    // -----------------------------------------------------------------------

    #[test]
    fn test_newline_tokens() {
        let tokens = tokenize("x\ny");
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[1].type_, TokenType::Newline);
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
        assert_eq!(tokens[2].line, 2);
    }

    // -----------------------------------------------------------------------
    // Empty input / error
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_input() {
        let tokens = tokenize("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].type_, TokenType::Eof);
    }

    #[test]
    fn test_unexpected_character() {
        let grammar = python_grammar();
        let result = GrammarLexer::new("x @ y", &grammar).tokenize();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Unexpected"));
    }

    // -----------------------------------------------------------------------
    // Complex expression
    // -----------------------------------------------------------------------

    #[test]
    fn test_function_definition() {
        let tokens = tokenize("def add(x, y):\n    return x + y");
        assert_eq!(tokens[0].type_, TokenType::Keyword);
        assert_eq!(tokens[0].value, "def");
        let return_tok = tokens.iter().find(|t| t.value == "return").unwrap();
        assert_eq!(return_tok.type_, TokenType::Keyword);
    }

    // -----------------------------------------------------------------------
    // Process escapes (unit test)
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
        assert_eq!(GrammarLexer::process_escapes(r"a\xb"), "axb");
    }

    #[test]
    fn test_process_escapes_no_escapes() {
        assert_eq!(GrammarLexer::process_escapes("plain text"), "plain text");
    }

    // -----------------------------------------------------------------------
    // Grammar without keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_grammar_without_keywords() {
        let grammar = parse_token_grammar(
            "NAME = /[a-zA-Z_]+/\nNUMBER = /[0-9]+/\nPLUS = \"+\"\n",
        )
        .unwrap();
        let tokens = GrammarLexer::new("if 42", &grammar).tokenize().unwrap();
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "if");
    }

    // -----------------------------------------------------------------------
    // Consistency: grammar lexer matches hand-written lexer
    // -----------------------------------------------------------------------

    #[test]
    fn test_consistency_with_hand_written_lexer() {
        use crate::tokenizer::Lexer;
        let source = "x = 1 + 2 * 3";
        let hand_tokens = Lexer::new(source, None).tokenize().unwrap();
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

    // -----------------------------------------------------------------------
    // Skip patterns
    // -----------------------------------------------------------------------

    #[test]
    fn test_skip_patterns() {
        let grammar = parse_token_grammar(
            "NAME = /[a-zA-Z_]+/\nNUMBER = /[0-9]+/\nskip:\n  WHITESPACE = /[ \\t]+/\n  COMMENT = /#[^\\n]*/",
        ).unwrap();
        let tokens = GrammarLexer::new("x 42 # comment", &grammar).tokenize().unwrap();
        // Should see: NAME("x"), NUMBER("42"), EOF — comment and whitespace skipped
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].value, "x");
        assert_eq!(tokens[1].value, "42");
        assert_eq!(tokens[2].type_, TokenType::Eof);
    }

    // -----------------------------------------------------------------------
    // Aliases
    // -----------------------------------------------------------------------

    #[test]
    fn test_alias_resolution() {
        let grammar = parse_token_grammar(
            r#"INT = /[0-9]+/ -> NUMBER
PLUS = "+""#,
        ).unwrap();
        let tokens = GrammarLexer::new("42 + 5", &grammar).tokenize().unwrap();
        // The token should be resolved to NUMBER type via the alias.
        assert_eq!(tokens[0].type_, TokenType::Number);
        assert_eq!(tokens[0].value, "42");
    }

    // -----------------------------------------------------------------------
    // Reserved keywords
    // -----------------------------------------------------------------------

    #[test]
    fn test_reserved_keyword_error() {
        let grammar = parse_token_grammar(
            "NAME = /[a-zA-Z_]+/\nreserved:\n  class\n  import",
        ).unwrap();
        let result = GrammarLexer::new("class", &grammar).tokenize();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Reserved keyword"));
    }

    #[test]
    fn test_reserved_keyword_allows_non_reserved() {
        let grammar = parse_token_grammar(
            "NAME = /[a-zA-Z_]+/\nreserved:\n  class",
        ).unwrap();
        let tokens = GrammarLexer::new("foo", &grammar).tokenize().unwrap();
        assert_eq!(tokens[0].type_, TokenType::Name);
        assert_eq!(tokens[0].value, "foo");
    }

    // -----------------------------------------------------------------------
    // String-based type names for custom tokens
    // -----------------------------------------------------------------------

    #[test]
    fn test_string_type_for_custom_tokens() {
        let grammar = parse_token_grammar(
            "IDENTIFIER = /[a-zA-Z_]+/\nINT = /[0-9]+/",
        ).unwrap();
        let tokens = GrammarLexer::new("foo 42", &grammar).tokenize().unwrap();
        // Custom types should have type_name set.
        assert_eq!(tokens[0].type_name, Some("IDENTIFIER".to_string()));
        assert_eq!(tokens[1].type_name, Some("INT".to_string()));
    }

    // -----------------------------------------------------------------------
    // Indentation mode
    // -----------------------------------------------------------------------

    #[test]
    fn test_indent_basic() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/\nCOLON = \":\"",
        ).unwrap();
        let source = "foo:\n    bar\n";
        let tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        // Expected: NAME("foo"), COLON, NEWLINE, INDENT, NAME("bar"), NEWLINE, DEDENT, EOF
        let types: Vec<TokenType> = tokens.iter().map(|t| t.type_).collect();
        assert!(types.contains(&TokenType::Indent));
        assert!(types.contains(&TokenType::Dedent));
    }

    #[test]
    fn test_indent_nested() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/\nCOLON = \":\"",
        ).unwrap();
        let source = "a:\n    b:\n        c\n";
        let tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        // Count INDENTs and DEDENTs.
        let indent_count = tokens.iter().filter(|t| t.type_ == TokenType::Indent).count();
        let dedent_count = tokens.iter().filter(|t| t.type_ == TokenType::Dedent).count();
        assert_eq!(indent_count, 2);
        assert_eq!(dedent_count, 2);
    }

    #[test]
    fn test_indent_blank_lines_skipped() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/",
        ).unwrap();
        let source = "a\n\n    \nb\n";
        let tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        // Blank lines should not produce NEWLINE tokens — just a, NEWLINE, b, NEWLINE, EOF
        let names: Vec<&str> = tokens.iter()
            .filter(|t| t.type_ == TokenType::Name)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn test_indent_tab_rejected() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/",
        ).unwrap();
        let source = "a\n\tb\n";
        let result = GrammarLexer::new(source, &grammar).tokenize();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Tabs"));
    }

    #[test]
    fn test_indent_bracket_suppression() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/\nLPAREN = \"(\"\nRPAREN = \")\"",
        ).unwrap();
        let source = "a(\n    b\n)\n";
        let tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        // Inside brackets, no INDENT/DEDENT should be emitted.
        let indent_count = tokens.iter().filter(|t| t.type_ == TokenType::Indent).count();
        assert_eq!(indent_count, 0);
    }

    // -----------------------------------------------------------------------
    // Indentation mode with skip patterns
    // -----------------------------------------------------------------------

    #[test]
    fn test_indent_with_skip_patterns() {
        let grammar = parse_token_grammar(
            "mode: indentation\nNAME = /[a-zA-Z_]+/\nCOLON = \":\"\nskip:\n  WHITESPACE = /[ \\t]+/\n  COMMENT = /#[^\\n]*/",
        ).unwrap();
        let source = "foo:\n    # comment\n    bar\n";
        let tokens = GrammarLexer::new(source, &grammar).tokenize().unwrap();

        let names: Vec<&str> = tokens.iter()
            .filter(|t| t.type_ == TokenType::Name)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(names, vec!["foo", "bar"]);
    }
}
