//! # `mccarthy-lisp-parser` — S-expression parser for McCarthy Lisp 1.0.
//!
//! Turns the token stream from
//! [`mccarthy_lisp_lexer::tokenize`] into an [`LispExpr`] AST that
//! matches McCarthy's 1960 paper definition exactly:
//!
//! ```text
//! e ::= NIL                            ; the empty list
//!     | Symbol                         ; FOO, CAR, X
//!     | Integer                        ; 42, -1, 0
//!     | (e . e)                        ; dotted pair (cons cell)
//! ```
//!
//! A "list" in McCarthy's paper is a *nested cons cell terminated
//! by NIL*: the sequence `(A B C)` is sugar for
//! `(A . (B . (C . NIL)))`.  Our parser performs that desugaring at
//! parse time — every list literal materialises as `Cons` cells
//! with a NIL terminator.
//!
//! Two more sugar expansions happen here:
//!
//! 1. **`'X` → `(QUOTE X)`** — McCarthy 1960 §3 introduces `QUOTE`
//!    as the way to write a literal S-expression; the apostrophe
//!    is the standard reader macro since.
//! 2. **Dotted pair `(A . B)`** parses as `Cons(A, B)` directly,
//!    *without* a NIL terminator — that's the whole point of the
//!    dot notation.
//!
//! ## A program is a sequence of forms
//!
//! Like Scheme `(define …) (define …) (main)`, a McCarthy program
//! is a `Vec<LispExpr>` — top-level forms evaluated in order.  The
//! parser returns that vector.
//!
//! ## Quick start
//!
//! ```
//! use mccarthy_lisp_parser::{parse, LispExpr};
//!
//! // (CAR '(A B C)) → take CAR of a literal list
//! let prog = parse("(CAR '(A B C))").expect("parse");
//! assert_eq!(prog.len(), 1);
//!
//! let expected = LispExpr::list([
//!     LispExpr::sym("CAR"),
//!     LispExpr::quote(LispExpr::list([
//!         LispExpr::sym("A"),
//!         LispExpr::sym("B"),
//!         LispExpr::sym("C"),
//!     ])),
//! ]);
//! assert_eq!(prog[0], expected);
//! ```

use std::fmt;

use mccarthy_lisp_lexer::{tokenize, LexError, Loc, Token, TokenWithLoc};

// ===========================================================================
// AST
// ===========================================================================

/// McCarthy 1960 Lisp S-expression.
///
/// Four cases — matches the abstract grammar of the 1960 paper
/// exactly.  Modern Lisps add strings, vectors, hash tables, etc.;
/// McCarthy 1.0 has only these four.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LispExpr {
    /// `()` — the empty list, also the false value (any other
    /// value is true under McCarthy's `COND`).
    Nil,
    /// `FOO` — an all-uppercase identifier (interned later as a
    /// symbol pointer at runtime).
    Symbol(String),
    /// `42` — a signed 64-bit integer.
    Int(i64),
    /// `(car . cdr)` — a cons cell, the only compound shape.
    Cons(Box<LispExpr>, Box<LispExpr>),
}

impl LispExpr {
    /// Convenience: build a `Symbol`.
    pub fn sym(name: impl Into<String>) -> Self {
        LispExpr::Symbol(name.into())
    }

    /// Build a McCarthy-style list (`a b c`) as nested `Cons` cells
    /// terminated by `Nil`: `(a . (b . (c . NIL)))`.
    pub fn list<I: IntoIterator<Item = LispExpr>>(items: I) -> Self {
        let v: Vec<LispExpr> = items.into_iter().collect();
        let mut acc = LispExpr::Nil;
        for x in v.into_iter().rev() {
            acc = LispExpr::Cons(Box::new(x), Box::new(acc));
        }
        acc
    }

    /// Wrap an expression in `(QUOTE …)` — the standard
    /// expansion of the `'` reader macro.
    pub fn quote(inner: LispExpr) -> Self {
        LispExpr::list([LispExpr::sym("QUOTE"), inner])
    }
}

impl fmt::Display for LispExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LispExpr::Nil => write!(f, "NIL"),
            LispExpr::Symbol(s) => write!(f, "{s}"),
            LispExpr::Int(n) => write!(f, "{n}"),
            LispExpr::Cons(car, cdr) => write!(f, "({} . {})", car, cdr),
        }
    }
}

// ===========================================================================
// Errors
// ===========================================================================

/// Errors the parser can report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The lexer rejected the source.
    Lex(LexError),
    /// Hit end-of-input mid-form (unbalanced open paren).
    UnexpectedEof {
        /// Where the open paren started.
        opened_at: Loc,
        /// What we were looking for next.
        expected: &'static str,
    },
    /// Saw a token where it doesn't belong.
    UnexpectedToken {
        /// The offending token.
        token: Token,
        /// Where it appeared.
        loc: Loc,
        /// What we expected instead.
        expected: &'static str,
    },
    /// A `.` outside a `(A . B)` form.
    StrayDot {
        /// Where the dot appeared.
        loc: Loc,
    },
    /// More than one `.` in a single list — McCarthy 1960 only
    /// allows one dotted-tail per list.
    MultipleDotsInList {
        /// Where the second dot appeared.
        loc: Loc,
    },
    /// `(A . )` — dot without a cdr term.
    DotWithoutCdr {
        /// Where the dot appeared.
        loc: Loc,
    },
    /// `(A . B C)` — dotted-tail followed by extra elements.
    ExtraAfterDottedTail {
        /// Where the extra token appeared.
        loc: Loc,
    },
    /// Recursive-descent depth exceeded [`MAX_NESTING`].
    ///
    /// Guards against a stack overflow on pathological inputs like
    /// `((((((((((…))))))))))` with thousands of nested parens, or
    /// `'''''…X` quote chains.  In practice McCarthy 1960 programs
    /// never approach this depth; the guard is purely a DoS
    /// hardening for untrusted source.
    NestingTooDeep {
        /// Where the parser tipped over the depth limit.
        loc: Loc,
    },
}

/// Maximum recursive-descent depth.
///
/// Picked to be:
///
/// 1. **Far above** anything a hand-written McCarthy program would
///    reach (the deepest example in the 1960 paper is ~10 levels —
///    256 leaves a 25× headroom for generated code and nested
///    macros).
/// 2. **Far below** the Windows 1 MB default test-thread stack
///    ceiling.  Each `(` requires *two* stack frames
///    (`parse_list` + the inner `parse_expr`), so 256 levels =
///    ~512 frames of recursive parser code, well inside even the
///    most constrained Rust thread stack.
///
/// Matches `serde-json`'s default nesting limit, which has the
/// same trade-off.
pub const MAX_NESTING: usize = 256;

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Lex(e) => write!(f, "parse error: {e}"),
            ParseError::UnexpectedEof { opened_at, expected } => write!(
                f,
                "parse error: unexpected end of input (expected {expected}); \
                 form opened at {opened_at}"
            ),
            ParseError::UnexpectedToken { token, loc, expected } => write!(
                f,
                "parse error at {loc}: unexpected {token:?} (expected {expected})"
            ),
            ParseError::StrayDot { loc } => write!(
                f,
                "parse error at {loc}: stray `.` outside a (A . B) form"
            ),
            ParseError::MultipleDotsInList { loc } => write!(
                f,
                "parse error at {loc}: more than one `.` in a list; \
                 McCarthy 1960 allows only one dotted-tail per list"
            ),
            ParseError::DotWithoutCdr { loc } => write!(
                f,
                "parse error at {loc}: `.` must be followed by exactly one expression (the cdr)"
            ),
            ParseError::ExtraAfterDottedTail { loc } => write!(
                f,
                "parse error at {loc}: extra tokens after dotted tail; \
                 `(A . B C)` is not a valid Lisp 1.0 form"
            ),
            ParseError::NestingTooDeep { loc } => write!(
                f,
                "parse error at {loc}: nesting depth exceeds {} \
                 (stack-overflow DoS guard)",
                MAX_NESTING
            ),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        ParseError::Lex(e)
    }
}

// ===========================================================================
// Public API
// ===========================================================================

/// Tokenize and parse a McCarthy 1960 Lisp source string.
///
/// Returns the top-level form sequence (a program is zero-or-more
/// S-expressions).
pub fn parse(src: &str) -> Result<Vec<LispExpr>, ParseError> {
    let toks = tokenize(src)?;
    parse_tokens(&toks)
}

/// Parse a pre-tokenized stream into the top-level form sequence.
pub fn parse_tokens(toks: &[TokenWithLoc]) -> Result<Vec<LispExpr>, ParseError> {
    let mut p = Parser { toks, pos: 0 };
    let mut forms = Vec::new();
    while p.pos < p.toks.len() {
        forms.push(p.parse_expr(0)?);
    }
    Ok(forms)
}

// ===========================================================================
// Recursive-descent parser
// ===========================================================================

struct Parser<'a> {
    toks: &'a [TokenWithLoc],
    pos: usize,
}

impl<'a> Parser<'a> {
    /// Parse one expression at the current position.
    ///
    /// `depth` is the current recursion level; we trip
    /// [`ParseError::NestingTooDeep`] before any pathological input
    /// can blow the stack.
    fn parse_expr(&mut self, depth: usize) -> Result<LispExpr, ParseError> {
        let twl = self.peek_or_eof("an expression")?;
        if depth >= MAX_NESTING {
            return Err(ParseError::NestingTooDeep { loc: twl.loc });
        }
        match &twl.tok {
            Token::LParen => self.parse_list(twl.loc, depth + 1),
            Token::Quote => self.parse_quote(twl.loc, depth + 1),
            Token::Symbol(name) => {
                let n = name.clone();
                self.pos += 1;
                Ok(LispExpr::Symbol(n))
            }
            Token::Int(n) => {
                let v = *n;
                self.pos += 1;
                Ok(LispExpr::Int(v))
            }
            Token::Dot => Err(ParseError::StrayDot { loc: twl.loc }),
            Token::RParen => Err(ParseError::UnexpectedToken {
                token: Token::RParen,
                loc: twl.loc,
                expected: "an expression",
            }),
        }
    }

    /// Parse a `(…)` form.  Caller has already peeked the `(`; we
    /// consume it here.
    fn parse_list(&mut self, opened_at: Loc, depth: usize) -> Result<LispExpr, ParseError> {
        // Consume `(`.
        self.pos += 1;

        let mut items: Vec<LispExpr> = Vec::new();
        let mut dotted_tail: Option<LispExpr> = None;

        loop {
            let twl = self.peek().ok_or(ParseError::UnexpectedEof {
                opened_at,
                expected: "`)` or another expression",
            })?;

            match &twl.tok {
                Token::RParen => {
                    self.pos += 1;
                    return Ok(build_list(items, dotted_tail));
                }
                Token::Dot => {
                    if dotted_tail.is_some() {
                        return Err(ParseError::MultipleDotsInList { loc: twl.loc });
                    }
                    if items.is_empty() {
                        // `(. X)` — dot must come after at least one car.
                        return Err(ParseError::UnexpectedToken {
                            token: Token::Dot,
                            loc: twl.loc,
                            expected: "the head of a dotted pair before `.`",
                        });
                    }
                    let dot_loc = twl.loc;
                    self.pos += 1; // consume `.`

                    // Must be followed by EXACTLY one expression, then `)`.
                    let after = self.peek_or_eof("the cdr of the dotted pair")?;
                    if matches!(after.tok, Token::RParen) {
                        return Err(ParseError::DotWithoutCdr { loc: dot_loc });
                    }
                    let cdr = self.parse_expr(depth)?;
                    dotted_tail = Some(cdr);

                    // Anything other than `)` next is a parse error.
                    let nxt = self.peek_or_eof("`)` after dotted tail")?;
                    if !matches!(nxt.tok, Token::RParen) {
                        return Err(ParseError::ExtraAfterDottedTail { loc: nxt.loc });
                    }
                    // Loop iterates once more and consumes `)`.
                }
                _ => {
                    items.push(self.parse_expr(depth)?);
                }
            }
        }
    }

    /// Parse `'X` — sugar for `(QUOTE X)`.
    fn parse_quote(&mut self, _quote_loc: Loc, depth: usize) -> Result<LispExpr, ParseError> {
        // Consume `'`.
        self.pos += 1;
        let inner = self.parse_expr(depth)?;
        Ok(LispExpr::quote(inner))
    }

    fn peek(&self) -> Option<&'a TokenWithLoc> {
        self.toks.get(self.pos)
    }

    fn peek_or_eof(&self, expected: &'static str) -> Result<&'a TokenWithLoc, ParseError> {
        self.peek().ok_or(ParseError::UnexpectedEof {
            opened_at: Loc::START,
            expected,
        })
    }
}

/// Build a McCarthy-style list from collected items + optional dotted tail.
///
/// `(A B C)` → `Cons(A, Cons(B, Cons(C, Nil)))`.
/// `(A B . C)` → `Cons(A, Cons(B, C))`.
fn build_list(items: Vec<LispExpr>, dotted_tail: Option<LispExpr>) -> LispExpr {
    let mut acc = dotted_tail.unwrap_or(LispExpr::Nil);
    for x in items.into_iter().rev() {
        acc = LispExpr::Cons(Box::new(x), Box::new(acc));
    }
    acc
}
