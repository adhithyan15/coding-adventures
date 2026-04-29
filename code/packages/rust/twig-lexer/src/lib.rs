//! # Twig Lexer — tokenises Twig (Lisp-precursor) source code.
//!
//! Twig is a tiny, purely-functional, S-expression language designed as a
//! precursor to a full Lisp implementation.  Its lexical structure is
//! correspondingly minimal:
//!
//! ```text
//! Punctuation:  (   )   '
//! Booleans:    #t  #f
//! Integer:     -?[0-9]+         (signed)
//! Name:        [A-Za-z+\-*/=<>!?_$][A-Za-z+\-*/=<>!?_$0-9]*
//! Keywords:    define lambda let if begin quote nil
//! Comments:    ;...newline      (skipped)
//! Whitespace:  spaces, tabs, CR, LF (skipped)
//! ```
//!
//! ## Why a Twig-specific lexer?
//!
//! The repo already has a generic [`lisp-lexer`](../lisp-lexer) and a
//! grammar-driven `GrammarLexer`.  Twig differs from generic Lisp in three
//! small but semantically important ways:
//!
//! 1. **Booleans are dedicated tokens.**  `#t` and `#f` are not symbols —
//!    they tokenise to `BoolTrue` / `BoolFalse`, not to a `Symbol("#t")`.
//! 2. **Reserved words are promoted to `Keyword`.**  When a name would
//!    otherwise lex as `Name` but its text matches one of the seven
//!    keywords (`define`, `lambda`, `let`, `if`, `begin`, `quote`, `nil`),
//!    the lexer emits `Keyword` instead.  This lets the parser dispatch
//!    on token type without an extra string comparison.
//! 3. **No string literals, no dotted-pair `.`.**  Twig v1 has no strings;
//!    cons cells are constructed via `(cons a b)`, never `(a . b)`.
//!
//! These choices match `code/grammars/twig.tokens` (the canonical token
//! grammar consumed by the Python `twig` package's `GrammarLexer`).
//!
//! ## Position tracking
//!
//! Each [`Token`] carries the 1-indexed `line` and `column` where it
//! starts.  This enables LSP-style error messages downstream — the parser
//! and IR compiler bubble these positions up into AST nodes and into
//! `TwigParseError` / `TwigCompileError` so the user can find the source
//! of a problem.
//!
//! ## Pipeline
//!
//! ```text
//! Twig source
//!     │
//!     ▼  tokenize()                 ← THIS CRATE
//! Vec<Token>  (terminates with Eof)
//!     │
//!     ▼  twig-parser
//! Program (typed AST)
//!     │
//!     ▼  twig-ir-compiler
//! IIRModule
//! ```
//!
//! ## Example
//!
//! ```
//! use twig_lexer::{tokenize, TokenKind};
//!
//! let tokens = tokenize("(define x 42)").unwrap();
//! assert_eq!(tokens[0].kind, TokenKind::LParen);
//! assert_eq!(tokens[1].kind, TokenKind::Keyword);
//! assert_eq!(tokens[1].value, "define");
//! assert_eq!(tokens[2].kind, TokenKind::Name);
//! assert_eq!(tokens[3].kind, TokenKind::Integer);
//! assert_eq!(tokens[4].kind, TokenKind::RParen);
//! assert_eq!(tokens[5].kind, TokenKind::Eof);
//! ```

use std::fmt;

// ---------------------------------------------------------------------------
// Section 1: Token kinds
// ---------------------------------------------------------------------------
//
// Twig has eight token kinds (plus EOF).  Compare to Lisp (7), CSS (~20),
// JavaScript (~15) — Twig's set is small because S-expressions impose almost
// no lexical structure beyond delimiters and atoms.
//
// We split off `BoolTrue` / `BoolFalse` from `Name` so the parser can match
// them with a single check, and we promote keywords from `Name` so reserved
// words like `define` and `if` cannot be shadowed by a typo at lex time.
// ---------------------------------------------------------------------------

/// The kind of a Twig token.
///
/// Each variant is a structural category produced by the lexer.  The
/// parser dispatches on this enum to decide which production rule applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// `(` — start of a list / compound form.
    LParen,
    /// `)` — end of a list.
    RParen,
    /// `'` — quote prefix; sugar for `(quote ...)`.
    Quote,
    /// `#t` — the true boolean literal.
    BoolTrue,
    /// `#f` — the false boolean literal.
    BoolFalse,
    /// A signed-integer literal, e.g. `42`, `-7`.  The token's `value`
    /// holds the source text; convert with `value.parse::<i64>()`.
    Integer,
    /// A reserved word: one of `define`, `lambda`, `let`, `if`, `begin`,
    /// `quote`, `nil`.  The exact word is stored in `value`.
    Keyword,
    /// A plain identifier — variable name, builtin (`+`, `cons`, …),
    /// predicate (`null?`, `pair?`), or any user-introduced symbol.
    Name,
    /// End of input.  Always emitted exactly once, as the last token.
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::LParen => write!(f, "LPAREN"),
            TokenKind::RParen => write!(f, "RPAREN"),
            TokenKind::Quote => write!(f, "QUOTE"),
            TokenKind::BoolTrue => write!(f, "BOOL_TRUE"),
            TokenKind::BoolFalse => write!(f, "BOOL_FALSE"),
            TokenKind::Integer => write!(f, "INTEGER"),
            TokenKind::Keyword => write!(f, "KEYWORD"),
            TokenKind::Name => write!(f, "NAME"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

/// A single token emitted by [`tokenize`].
///
/// Carries source text plus 1-indexed `line` / `column` of the token's
/// starting character.  EOF gets the position immediately after the last
/// real character (one past the end), which is what most editors render
/// for caret positions at end-of-file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Structural category (see [`TokenKind`]).
    pub kind: TokenKind,
    /// The source text that produced this token.
    ///
    /// - `LParen` / `RParen` / `Quote`: the single character.
    /// - `BoolTrue` / `BoolFalse`: `"#t"` / `"#f"`.
    /// - `Integer`: the numeric text including any leading `-`.
    /// - `Keyword` / `Name`: the identifier text.
    /// - `Eof`: empty.
    pub value: String,
    /// 1-indexed source line where this token begins.
    pub line: usize,
    /// 1-indexed source column where this token begins.
    pub column: usize,
}

impl Token {
    /// Convenience constructor used internally by the lexer.
    fn new(kind: TokenKind, value: impl Into<String>, line: usize, column: usize) -> Self {
        Token { kind, value: value.into(), line, column }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Token({}, {:?}, {}:{})", self.kind, self.value, self.line, self.column)
    }
}

// ---------------------------------------------------------------------------
// Section 2: Errors
// ---------------------------------------------------------------------------

/// Lexer error — produced when the input contains a character or sequence
/// the lexer doesn't recognise (e.g. a stray `#` not followed by `t`/`f`,
/// or a non-ASCII control character).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    /// Human-readable description.
    pub message: String,
    /// 1-indexed source line of the offending character.
    pub line: usize,
    /// 1-indexed source column of the offending character.
    pub column: usize,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LexerError at {}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for LexerError {}

// ---------------------------------------------------------------------------
// Section 3: Character classification
// ---------------------------------------------------------------------------
//
// Twig's NAME regex from `code/grammars/twig.tokens`:
//
//     NAME = /[A-Za-z+\-*/=<>!?_$][A-Za-z+\-*/=<>!?_$0-9]*/
//
// The character class is:
//   - letters (A-Z / a-z)
//   - operator characters: + - * / = < > ! ?
//   - underscore _ and dollar $
// Plus digits in the continuation position only — `42` is INTEGER, but
// `f42` is NAME.
// ---------------------------------------------------------------------------

/// True if `ch` may appear as the first character of a `Name` / `Keyword`.
fn is_name_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
        || ch == '_'
        || ch == '$'
        || ch == '+'
        || ch == '-'
        || ch == '*'
        || ch == '/'
        || ch == '='
        || ch == '<'
        || ch == '>'
        || ch == '!'
        || ch == '?'
}

/// True if `ch` may appear in a `Name` / `Keyword` after the first character.
///
/// Same as [`is_name_start`] plus ASCII digits.
fn is_name_continue(ch: char) -> bool {
    is_name_start(ch) || ch.is_ascii_digit()
}

/// The seven reserved words that lex as `Keyword` instead of `Name`.
///
/// Listed in the same order as `code/grammars/twig.tokens` so a maintainer
/// editing one file can mechanically check the other.
const KEYWORDS: &[&str] = &[
    "define", "lambda", "let", "if", "begin", "quote", "nil",
];

fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word)
}

// ---------------------------------------------------------------------------
// Section 4: The tokenizer
// ---------------------------------------------------------------------------
//
// Hand-written scanner over a `Vec<char>` (so we get byte-position-free
// indexing into the source).  Each step:
//
//   1. Skip whitespace + `;`-to-EOL comments
//   2. Look at the current char; dispatch to a specialised reader
//   3. Push a `Token` and continue
//
// Single-character delimiters short-circuit; multi-character forms (`#t`,
// `#f`, integers, names) recurse into a helper.  The number-vs-name
// disambiguation for a leading `-` mirrors the regex contract: `-` is a
// number iff it is followed immediately by a digit.
// ---------------------------------------------------------------------------

/// Tokenise Twig source into a `Vec<Token>` ending with `TokenKind::Eof`.
///
/// Whitespace and `;`-to-end-of-line comments are silently consumed —
/// they never appear in the output.  Position tracking starts at line 1,
/// column 1 and increments per character (newline → column resets to 1).
///
/// # Errors
///
/// Returns [`LexerError`] when the input contains a character outside
/// every token's valid set — most often a stray `#` not followed by
/// `t`/`f`, or a literal Unicode character outside ASCII.
///
/// # Example
///
/// ```
/// use twig_lexer::{tokenize, TokenKind};
///
/// let tokens = tokenize("(+ 1 -2)").unwrap();
/// // [LParen, Name(+), Integer(1), Integer(-2), RParen, Eof]
/// assert_eq!(tokens.len(), 6);
/// assert_eq!(tokens[3].kind, TokenKind::Integer);
/// assert_eq!(tokens[3].value, "-2");
/// ```
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut pos = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    while pos < chars.len() {
        let ch = chars[pos];

        // -----------------------------------------------------------------
        // Whitespace — newlines reset the column counter
        // -----------------------------------------------------------------
        if ch == '\n' {
            pos += 1;
            line += 1;
            col = 1;
            continue;
        }
        if ch.is_ascii_whitespace() {
            pos += 1;
            col += 1;
            continue;
        }

        // -----------------------------------------------------------------
        // ;-to-end-of-line comments
        // -----------------------------------------------------------------
        // The newline itself is not consumed here — the loop's whitespace
        // arm handles it on the next iteration so the line counter stays
        // accurate.
        if ch == ';' {
            while pos < chars.len() && chars[pos] != '\n' {
                pos += 1;
                col += 1;
            }
            continue;
        }

        // -----------------------------------------------------------------
        // Single-character delimiters
        // -----------------------------------------------------------------
        match ch {
            '(' => {
                tokens.push(Token::new(TokenKind::LParen, "(", line, col));
                pos += 1;
                col += 1;
                continue;
            }
            ')' => {
                tokens.push(Token::new(TokenKind::RParen, ")", line, col));
                pos += 1;
                col += 1;
                continue;
            }
            '\'' => {
                tokens.push(Token::new(TokenKind::Quote, "'", line, col));
                pos += 1;
                col += 1;
                continue;
            }
            _ => {}
        }

        // -----------------------------------------------------------------
        // Boolean literals — `#t` and `#f`
        // -----------------------------------------------------------------
        // The `#` character has no other valid use in Twig v1, so any `#`
        // not followed by `t` or `f` is a hard error.  We capture the
        // original position for the error so the caller can point at the
        // `#`, not the unexpected follow-up.
        if ch == '#' {
            let start_line = line;
            let start_col = col;
            if pos + 1 >= chars.len() {
                return Err(LexerError {
                    message: "expected '#t' or '#f', got '#' at end of input".into(),
                    line: start_line,
                    column: start_col,
                });
            }
            let next = chars[pos + 1];
            let kind = match next {
                't' => TokenKind::BoolTrue,
                'f' => TokenKind::BoolFalse,
                other => {
                    return Err(LexerError {
                        message: format!("expected '#t' or '#f', got '#{other}'"),
                        line: start_line,
                        column: start_col,
                    });
                }
            };
            let lex = if next == 't' { "#t" } else { "#f" };
            tokens.push(Token::new(kind, lex, start_line, start_col));
            pos += 2;
            col += 2;
            continue;
        }

        // -----------------------------------------------------------------
        // Integers — `-?[0-9]+`
        // -----------------------------------------------------------------
        // `-` is ambiguous: bare `-` is a NAME (the subtraction operator),
        // but `-42` is one INTEGER token.  The regex requires *at least
        // one* digit, so we only treat `-` as the start of a number when
        // the next character is a digit.  Everything else falls through
        // to the name reader.
        let starts_number = ch.is_ascii_digit()
            || (ch == '-' && pos + 1 < chars.len() && chars[pos + 1].is_ascii_digit());
        if starts_number {
            let start_line = line;
            let start_col = col;
            let start = pos;
            if ch == '-' {
                pos += 1;
                col += 1;
            }
            while pos < chars.len() && chars[pos].is_ascii_digit() {
                pos += 1;
                col += 1;
            }
            let value: String = chars[start..pos].iter().collect();
            tokens.push(Token::new(TokenKind::Integer, value, start_line, start_col));
            continue;
        }

        // -----------------------------------------------------------------
        // Names + keyword promotion
        // -----------------------------------------------------------------
        // Read a maximal NAME, then check whether the resulting text is
        // one of the seven reserved words.  Keyword promotion happens
        // here (not in a separate post-pass) so the parser sees a single
        // unambiguous stream.
        if is_name_start(ch) {
            let start_line = line;
            let start_col = col;
            let start = pos;
            pos += 1;
            col += 1;
            while pos < chars.len() && is_name_continue(chars[pos]) {
                pos += 1;
                col += 1;
            }
            let value: String = chars[start..pos].iter().collect();
            let kind = if is_keyword(&value) {
                TokenKind::Keyword
            } else {
                TokenKind::Name
            };
            tokens.push(Token::new(kind, value, start_line, start_col));
            continue;
        }

        // -----------------------------------------------------------------
        // Unrecognised character
        // -----------------------------------------------------------------
        return Err(LexerError {
            message: format!("unexpected character {ch:?}"),
            line,
            column: col,
        });
    }

    // EOF token: position is one past the last consumed character.
    tokens.push(Token::new(TokenKind::Eof, "", line, col));
    Ok(tokens)
}

// ---------------------------------------------------------------------------
// Section 5: Tests
// ---------------------------------------------------------------------------
//
// The test suite mirrors the Python `tests/test_lexer.py` cases in coverage
// — atoms, parens, quotes, booleans, comments, multi-line position tracking,
// keyword promotion, error paths.  The `kinds` / `values` helpers strip
// the trailing EOF so tests can assert against the real token list directly.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        tokenize(source)
            .unwrap()
            .into_iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .map(|t| t.kind)
            .collect()
    }

    fn values(source: &str) -> Vec<String> {
        tokenize(source)
            .unwrap()
            .into_iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .map(|t| t.value)
            .collect()
    }

    // -- Punctuation --

    #[test]
    fn empty_input_is_just_eof() {
        let toks = tokenize("").unwrap();
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::Eof);
    }

    #[test]
    fn parens() {
        assert_eq!(kinds("()"), vec![TokenKind::LParen, TokenKind::RParen]);
    }

    #[test]
    fn quote_prefix() {
        assert_eq!(kinds("'foo"), vec![TokenKind::Quote, TokenKind::Name]);
        assert_eq!(values("'foo"), vec!["'", "foo"]);
    }

    // -- Booleans --

    #[test]
    fn bool_true_and_false_are_distinct_kinds() {
        assert_eq!(kinds("#t"), vec![TokenKind::BoolTrue]);
        assert_eq!(kinds("#f"), vec![TokenKind::BoolFalse]);
        assert_eq!(values("#t #f"), vec!["#t", "#f"]);
    }

    #[test]
    fn lone_hash_errors() {
        let err = tokenize("#").unwrap_err();
        assert!(err.message.contains("end of input"));
    }

    #[test]
    fn hash_with_unknown_letter_errors() {
        let err = tokenize("#x").unwrap_err();
        assert!(err.message.contains("'#x'"));
    }

    // -- Integers --

    #[test]
    fn positive_integer() {
        assert_eq!(kinds("42"), vec![TokenKind::Integer]);
        assert_eq!(values("42"), vec!["42"]);
    }

    #[test]
    fn negative_integer() {
        assert_eq!(kinds("-7"), vec![TokenKind::Integer]);
        assert_eq!(values("-7"), vec!["-7"]);
    }

    #[test]
    fn zero() {
        assert_eq!(kinds("0"), vec![TokenKind::Integer]);
    }

    #[test]
    fn bare_minus_is_a_name_not_an_integer() {
        // `(- 3 1)` — the `-` is a NAME because no digit follows it.
        let toks = tokenize("(- 3 1)").unwrap();
        assert_eq!(
            toks.iter().map(|t| t.kind).collect::<Vec<_>>(),
            vec![
                TokenKind::LParen,
                TokenKind::Name,
                TokenKind::Integer,
                TokenKind::Integer,
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
        assert_eq!(toks[1].value, "-");
    }

    #[test]
    fn negative_inside_call_position() {
        // Position-disambiguation again: `(+ -2 3)` lexes the `-2` as
        // a single integer because the `-` is followed by a digit.
        let toks = tokenize("(+ -2 3)").unwrap();
        assert_eq!(toks[2].kind, TokenKind::Integer);
        assert_eq!(toks[2].value, "-2");
    }

    // -- Names + keyword promotion --

    #[test]
    fn plain_name() {
        assert_eq!(kinds("foo"), vec![TokenKind::Name]);
    }

    #[test]
    fn operator_names_lex_as_name() {
        assert_eq!(kinds("+"), vec![TokenKind::Name]);
        assert_eq!(kinds("*"), vec![TokenKind::Name]);
        assert_eq!(kinds("="), vec![TokenKind::Name]);
        assert_eq!(kinds("<"), vec![TokenKind::Name]);
        assert_eq!(kinds(">"), vec![TokenKind::Name]);
    }

    #[test]
    fn predicate_name_with_question_mark() {
        assert_eq!(kinds("null?"), vec![TokenKind::Name]);
        assert_eq!(values("null?"), vec!["null?"]);
        assert_eq!(values("pair?"), vec!["pair?"]);
    }

    #[test]
    fn name_with_digits_after_letter() {
        // `f42` is a NAME, not an INTEGER.
        assert_eq!(kinds("f42"), vec![TokenKind::Name]);
    }

    #[test]
    fn keywords_are_promoted() {
        // Each of the seven reserved words gets its own KEYWORD token.
        for kw in ["define", "lambda", "let", "if", "begin", "quote", "nil"] {
            let toks = tokenize(kw).unwrap();
            assert_eq!(toks[0].kind, TokenKind::Keyword, "{kw} should be KEYWORD");
            assert_eq!(toks[0].value, kw);
        }
    }

    #[test]
    fn keywords_inside_expression_keep_their_kind() {
        let toks = tokenize("(define x 42)").unwrap();
        assert_eq!(toks[1].kind, TokenKind::Keyword);
        assert_eq!(toks[1].value, "define");
    }

    #[test]
    fn similar_but_not_keyword_lexes_as_name() {
        // `defines` (with trailing `s`) is not a reserved word.
        assert_eq!(kinds("defines"), vec![TokenKind::Name]);
        assert_eq!(kinds("if?"), vec![TokenKind::Name]); // trailing `?`
        assert_eq!(kinds("nilly"), vec![TokenKind::Name]);
    }

    // -- Whitespace + comments --

    #[test]
    fn whitespace_skipped() {
        assert_eq!(kinds("   42   "), vec![TokenKind::Integer]);
        assert_eq!(kinds("\t\t1\n\n2"), vec![TokenKind::Integer, TokenKind::Integer]);
    }

    #[test]
    fn line_comment_skipped() {
        assert_eq!(
            kinds("; comment\n42"),
            vec![TokenKind::Integer]
        );
    }

    #[test]
    fn comment_to_end_of_input_without_newline() {
        // No trailing newline — the comment runs to EOF.
        assert_eq!(kinds("; the end"), Vec::<TokenKind>::new());
    }

    #[test]
    fn inline_comment_after_expression() {
        let toks = tokenize("(+ 1 2) ; add").unwrap();
        assert_eq!(toks[0].kind, TokenKind::LParen);
        assert_eq!(toks[4].kind, TokenKind::RParen);
    }

    // -- Position tracking --

    #[test]
    fn first_token_is_at_1_1() {
        let toks = tokenize("foo").unwrap();
        assert_eq!(toks[0].line, 1);
        assert_eq!(toks[0].column, 1);
    }

    #[test]
    fn columns_advance_per_character() {
        let toks = tokenize("(+ 1 2)").unwrap();
        // (=1, +=2, 1=4, 2=6, )=7
        assert_eq!(toks[0].column, 1);
        assert_eq!(toks[1].column, 2);
        assert_eq!(toks[2].column, 4);
        assert_eq!(toks[3].column, 6);
        assert_eq!(toks[4].column, 7);
    }

    #[test]
    fn newline_resets_column_and_advances_line() {
        let toks = tokenize("a\nb").unwrap();
        assert_eq!((toks[0].line, toks[0].column), (1, 1));
        assert_eq!((toks[1].line, toks[1].column), (2, 1));
    }

    #[test]
    fn comment_does_not_break_line_tracking() {
        let toks = tokenize("1 ; trailing\n2").unwrap();
        assert_eq!(toks[0].line, 1);
        assert_eq!(toks[1].line, 2);
        assert_eq!(toks[1].column, 1);
    }

    // -- Eof --

    #[test]
    fn eof_is_always_last() {
        let toks = tokenize("(define x 42)").unwrap();
        assert_eq!(toks.last().unwrap().kind, TokenKind::Eof);
    }

    #[test]
    fn eof_position_is_after_last_real_char() {
        let toks = tokenize("ab").unwrap();
        let eof = toks.last().unwrap();
        assert_eq!(eof.line, 1);
        assert_eq!(eof.column, 3);
    }

    // -- Full programs (smoke tests) --

    #[test]
    fn factorial_definition_tokenises() {
        let src = "(define (fact n)\n  (if (= n 0) 1 (* n (fact (- n 1)))))";
        let toks = tokenize(src).unwrap();
        // Sanity: starts with LParen + KEYWORD "define".
        assert_eq!(toks[0].kind, TokenKind::LParen);
        assert_eq!(toks[1].kind, TokenKind::Keyword);
        assert_eq!(toks[1].value, "define");
        // Must contain at least one IF keyword.
        assert!(toks.iter().any(|t| t.kind == TokenKind::Keyword && t.value == "if"));
    }

    #[test]
    fn quoted_symbol_in_program() {
        let toks = tokenize("(eq? 'foo 'bar)").unwrap();
        // Quote tokens precede the names.
        let kinds: Vec<TokenKind> = toks.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::LParen,
                TokenKind::Name,    // eq?
                TokenKind::Quote,
                TokenKind::Name,    // foo
                TokenKind::Quote,
                TokenKind::Name,    // bar
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn nested_if_with_booleans() {
        let toks = tokenize("(if #t #f #t)").unwrap();
        let kinds: Vec<TokenKind> = toks.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::LParen,
                TokenKind::Keyword, // if
                TokenKind::BoolTrue,
                TokenKind::BoolFalse,
                TokenKind::BoolTrue,
                TokenKind::RParen,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn let_with_bindings() {
        let toks = tokenize("(let ((x 1) (y 2)) (+ x y))").unwrap();
        // Two KEYWORD lookups: `let`. (`x`, `y`, `+` are all NAME.)
        assert_eq!(toks[1].kind, TokenKind::Keyword);
        assert_eq!(toks[1].value, "let");
    }

    // -- Errors --

    #[test]
    fn non_ascii_unicode_errors_with_position() {
        let err = tokenize("(+ 1 €)").unwrap_err();
        assert!(err.message.contains("unexpected character"));
        assert_eq!(err.line, 1);
    }

    #[test]
    fn display_lexer_error_includes_position() {
        let err = tokenize("@").unwrap_err();
        let s = format!("{err}");
        assert!(s.contains("LexerError"));
        assert!(s.contains("1:1"));
    }

    #[test]
    fn display_token_kind_uppercase() {
        assert_eq!(format!("{}", TokenKind::LParen), "LPAREN");
        assert_eq!(format!("{}", TokenKind::Eof), "EOF");
    }
}
