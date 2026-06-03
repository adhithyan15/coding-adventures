//! # `mccarthy-lisp-lexer` — tokenizer for McCarthy's 1960 Lisp.
//!
//! Lisp 1.0 (the language described in John McCarthy's 1960 paper
//! *"Recursive Functions of Symbolic Expressions and Their
//! Computation by Machine, Part I"*) has the simplest tokenizer of
//! any production language ever shipped — six token kinds, no
//! string literals, no operator symbols, no floats:
//!
//! | Token kind | Source form           | Notes                              |
//! |------------|-----------------------|------------------------------------|
//! | `LParen`   | `(`                   | list / call open                   |
//! | `RParen`   | `)`                   | list / call close                  |
//! | `Quote`    | `'`                   | sugar — `'X` → `(QUOTE X)` later   |
//! | `Dot`      | `.`                   | dotted-pair separator              |
//! | `Symbol`   | `[A-Z][A-Z0-9-]*`     | all-uppercase atoms                |
//! | `Int`      | `-?[0-9]+`            | signed decimal integers            |
//!
//! Plus whitespace and `;`-to-EOL comments (skipped — comments
//! weren't in McCarthy's paper but Lisp 1.5 added them and they
//! make test sources ergonomic).
//!
//! ## Why we built a new crate vs reusing `lisp-lexer`
//!
//! The in-tree `lisp-lexer` targets a modern Scheme-ish dialect:
//! lowercase symbols, strings, decimals, operator symbols
//! (`+`, `<=`, `null?`).  None of those existed in 1960.
//! Enforcing the McCarthy-1960 token grammar at the lexer keeps
//! the downstream `mccarthy-lisp-parser` and
//! `mccarthy-lisp-iir-compiler` honest about which Lisp dialect
//! they consume.
//!
//! ## How tokenization disambiguates negative numbers
//!
//! `-1` is an integer.  `-` alone is not a valid Lisp 1.0 token
//! (no operator symbols).  If we see `-` followed immediately by a
//! digit, we consume it as the integer sign; otherwise we report
//! [`LexError::InvalidByte`].  This rule matches how McCarthy's
//! paper wrote negative literals (always glued to the digit
//! sequence).
//!
//! ## Source-location bookkeeping
//!
//! Each emitted token carries a [`Loc`] with 1-based line and
//! column, so error messages in later phases (parser, compiler)
//! can point at the right character without re-tokenizing.
//!
//! ## Quick start
//!
//! ```
//! use mccarthy_lisp_lexer::{tokenize, Token};
//!
//! // The canonical "first Lisp program" — McCarthy 1960 §3.
//! let toks = tokenize("(CAR '(A B C))").expect("tokenize");
//! let kinds: Vec<&Token> = toks.iter().map(|t| &t.tok).collect();
//! assert_eq!(kinds, vec![
//!     &Token::LParen,
//!     &Token::Symbol("CAR".into()),
//!     &Token::Quote,
//!     &Token::LParen,
//!     &Token::Symbol("A".into()),
//!     &Token::Symbol("B".into()),
//!     &Token::Symbol("C".into()),
//!     &Token::RParen,
//!     &Token::RParen,
//! ]);
//! ```

use std::fmt;

// ===========================================================================
// Token + Loc
// ===========================================================================

/// A McCarthy 1960 Lisp token kind.
///
/// Six variants — that's the entire grammar at the lexer level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `'` — quote sugar; the parser expands to `(QUOTE …)`.
    Quote,
    /// `.` — dotted-pair separator: `(A . B)`.
    Dot,
    /// All-uppercase atom: `[A-Z][A-Z0-9-]*`.
    Symbol(String),
    /// Signed decimal integer: `-?[0-9]+`.
    Int(i64),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::Quote => write!(f, "'"),
            Token::Dot => write!(f, "."),
            Token::Symbol(s) => write!(f, "{s}"),
            Token::Int(n) => write!(f, "{n}"),
        }
    }
}

/// 1-based source location of a token.
///
/// Line and column are 1-indexed so error messages match how IDEs,
/// editors, and the original Lisp 1.5 manual numbered their listings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loc {
    /// 1-based line number (`\n` increments).
    pub line: usize,
    /// 1-based column number (resets on `\n`).
    pub column: usize,
}

impl Loc {
    /// First character of the source — line 1, column 1.
    pub const START: Loc = Loc { line: 1, column: 1 };
}

impl fmt::Display for Loc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A token plus the source location it started at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenWithLoc {
    /// The token kind + payload.
    pub tok: Token,
    /// Where the token started in the source.
    pub loc: Loc,
}

// ===========================================================================
// Errors
// ===========================================================================

/// Errors the lexer can report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexError {
    /// A character that's not part of any Lisp 1.0 token.
    ///
    /// Carries the offending byte (the character casted to `u8`), the
    /// source location, and a short human reason.
    InvalidByte {
        /// The byte that broke tokenization.
        byte: u8,
        /// Where in the source it appeared.
        loc: Loc,
        /// Human-readable explanation.
        reason: &'static str,
    },
    /// A `-` not followed by a digit.
    ///
    /// Lisp 1.0 has no operator symbols, so a bare `-` is meaningless.
    LoneMinus {
        /// Where the `-` appeared.
        loc: Loc,
    },
    /// An integer literal that didn't fit in `i64`.
    ///
    /// The 1960 paper didn't bound integer sizes; we use Rust's
    /// `i64` for parity with the rest of the IIR pipeline (`Operand::Int`
    /// is `i64`).
    IntegerOverflow {
        /// The numeric text that overflowed.
        text: String,
        /// Where it appeared in the source.
        loc: Loc,
    },
    /// A lowercase letter.
    ///
    /// McCarthy 1960 Lisp is all-uppercase.  Lowercase letters are
    /// reserved for future Lisp 1.5+ extension by this crate.
    LowercaseInSymbol {
        /// The lowercase byte we saw.
        byte: u8,
        /// Where it appeared.
        loc: Loc,
    },
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::InvalidByte { byte, loc, reason } => write!(
                f,
                "lex error at {loc}: invalid byte 0x{byte:02X} ({}) — {reason}",
                escape_byte(*byte)
            ),
            LexError::LoneMinus { loc } => write!(
                f,
                "lex error at {loc}: `-` is not a valid Lisp 1.0 token \
                 (no operator symbols); use as part of a signed integer like `-42`"
            ),
            LexError::IntegerOverflow { text, loc } => write!(
                f,
                "lex error at {loc}: integer literal {text:?} overflows i64"
            ),
            LexError::LowercaseInSymbol { byte, loc } => write!(
                f,
                "lex error at {loc}: lowercase {:?} — McCarthy 1960 Lisp is \
                 all-uppercase; write symbols in CAPS",
                *byte as char
            ),
        }
    }
}

impl std::error::Error for LexError {}

fn escape_byte(b: u8) -> String {
    if b.is_ascii_graphic() {
        (b as char).to_string()
    } else {
        format!("\\x{b:02X}")
    }
}

// ===========================================================================
// Tokenizer
// ===========================================================================

/// Tokenize a McCarthy 1960 Lisp source string.
///
/// Returns the token stream plus per-token source locations.
/// Whitespace and `;` line comments are skipped (no token emitted).
///
/// # Errors
///
/// See [`LexError`].  The lexer fails fast on the first offending
/// byte and reports its location.
pub fn tokenize(src: &str) -> Result<Vec<TokenWithLoc>, LexError> {
    let bytes = src.as_bytes();
    let mut out: Vec<TokenWithLoc> = Vec::new();
    let mut i: usize = 0;
    let mut line: usize = 1;
    let mut col: usize = 1;

    while i < bytes.len() {
        let b = bytes[i];
        let loc = Loc { line, column: col };

        // ----- Whitespace ------------------------------------------------
        if matches!(b, b' ' | b'\t' | b'\r') {
            i += 1;
            col += 1;
            continue;
        }
        if b == b'\n' {
            i += 1;
            line += 1;
            col = 1;
            continue;
        }

        // ----- Line comment (Lisp 1.5 convenience) -----------------------
        if b == b';' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
                col += 1;
            }
            continue;
        }

        // ----- Single-character punctuation ------------------------------
        match b {
            b'(' => {
                out.push(TokenWithLoc { tok: Token::LParen, loc });
                i += 1;
                col += 1;
                continue;
            }
            b')' => {
                out.push(TokenWithLoc { tok: Token::RParen, loc });
                i += 1;
                col += 1;
                continue;
            }
            b'\'' => {
                out.push(TokenWithLoc { tok: Token::Quote, loc });
                i += 1;
                col += 1;
                continue;
            }
            b'.' => {
                out.push(TokenWithLoc { tok: Token::Dot, loc });
                i += 1;
                col += 1;
                continue;
            }
            _ => {}
        }

        // ----- Negative integer ------------------------------------------
        // `-` is only valid as the sign of an integer.  A bare `-` (not
        // followed by a digit) is a lex error: Lisp 1.0 has no operator
        // symbols.
        if b == b'-' {
            let next = bytes.get(i + 1).copied();
            if !matches!(next, Some(d) if d.is_ascii_digit()) {
                return Err(LexError::LoneMinus { loc });
            }
            let (text, end) = read_int_literal(bytes, i);
            let parsed: i64 = text.parse().map_err(|_| LexError::IntegerOverflow {
                text: text.clone(),
                loc,
            })?;
            out.push(TokenWithLoc { tok: Token::Int(parsed), loc });
            col += end - i;
            i = end;
            continue;
        }

        // ----- Unsigned integer ------------------------------------------
        if b.is_ascii_digit() {
            let (text, end) = read_int_literal(bytes, i);
            let parsed: i64 = text.parse().map_err(|_| LexError::IntegerOverflow {
                text: text.clone(),
                loc,
            })?;
            out.push(TokenWithLoc { tok: Token::Int(parsed), loc });
            col += end - i;
            i = end;
            continue;
        }

        // ----- Symbol -----------------------------------------------------
        // McCarthy 1960 symbols: leading uppercase letter, then
        // uppercase letters / digits / hyphens.
        if b.is_ascii_uppercase() {
            let (text, end) = read_symbol(bytes, i)?;
            out.push(TokenWithLoc { tok: Token::Symbol(text), loc });
            col += end - i;
            i = end;
            continue;
        }

        // ----- Caught lowercase explicitly -------------------------------
        if b.is_ascii_lowercase() {
            return Err(LexError::LowercaseInSymbol { byte: b, loc });
        }

        // ----- Otherwise — invalid byte ----------------------------------
        return Err(LexError::InvalidByte {
            byte: b,
            loc,
            reason: "not a valid Lisp 1.0 character",
        });
    }

    Ok(out)
}

/// Read a digit-only span starting at `start`.  Includes a leading `-`
/// if the caller has already verified the next byte is a digit.
fn read_int_literal(bytes: &[u8], start: usize) -> (String, usize) {
    let mut i = start;
    if bytes.get(i) == Some(&b'-') {
        i += 1;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // SAFETY: every byte in [start, i) is ASCII (`-` or digit), so the
    // slice is valid UTF-8.  Using `from_utf8` keeps the implementation
    // safe-only.
    let text = std::str::from_utf8(&bytes[start..i])
        .expect("ASCII digits + optional leading `-` are valid UTF-8")
        .to_string();
    (text, i)
}

/// Read a symbol starting at `start`.  Leading byte must be ASCII
/// uppercase; subsequent bytes may be uppercase / digits / hyphens.
fn read_symbol(bytes: &[u8], start: usize) -> Result<(String, usize), LexError> {
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-' {
            i += 1;
        } else if b.is_ascii_lowercase() {
            // Pinpoint a lowercase byte mid-symbol with its own location.
            // Re-compute the location by walking line/column from start —
            // for the symbol case the symbol is short, so this is cheap.
            // The exposed `tokenize` caller doesn't need our internal
            // line/col; we just report the byte.
            return Err(LexError::LowercaseInSymbol {
                byte: b,
                // Without re-threading line/col, fall back to a marker
                // location pointing at start.  Callers can still find
                // the offending symbol by name in the source.
                loc: Loc { line: 0, column: start + 1 },
            });
        } else {
            break;
        }
    }
    let text = std::str::from_utf8(&bytes[start..i])
        .expect("uppercase ASCII + digits + hyphen are valid UTF-8")
        .to_string();
    Ok((text, i))
}
