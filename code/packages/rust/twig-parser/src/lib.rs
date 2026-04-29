//! # Twig Parser — token stream → typed AST.
//!
//! Twig's grammar fits on one screen (eight production rules):
//!
//! ```text
//! program     = { form } ;
//! form        = define | expr ;
//! define      = LPAREN "define" name_or_signature expr { expr } RPAREN ;
//! name_or_signature = NAME | LPAREN NAME { NAME } RPAREN ;
//! expr        = atom | quoted | compound ;
//! atom        = INTEGER | BOOL_TRUE | BOOL_FALSE | "nil" | NAME ;
//! quoted      = QUOTE NAME ;
//! compound    = if_form | let_form | begin_form | lambda_form | quote_form | apply ;
//! if_form     = LPAREN "if" expr expr expr RPAREN ;
//! let_form    = LPAREN "let" LPAREN { binding } RPAREN expr { expr } RPAREN ;
//! binding     = LPAREN NAME expr RPAREN ;
//! begin_form  = LPAREN "begin" expr { expr } RPAREN ;
//! lambda_form = LPAREN "lambda" LPAREN { NAME } RPAREN expr { expr } RPAREN ;
//! quote_form  = LPAREN "quote" NAME RPAREN ;
//! apply       = LPAREN expr { expr } RPAREN ;
//! ```
//!
//! ## What this crate produces
//!
//! Where the Python implementation walks a generic `ASTNode` tree and uses
//! a separate `ast_extract.py` pass to lift it into typed dataclasses,
//! the Rust parser builds the typed AST directly.  Every node is one of
//! a small, exhaustive set of variants from [`Expr`] / [`Form`] /
//! [`Program`], so the downstream IR compiler dispatches on enum
//! discriminants without `isinstance`-style checks.
//!
//! Each AST node carries 1-indexed `line` / `column` fields so the IR
//! compiler can emit source-position-tagged error messages.
//!
//! ## Define-sugar
//!
//! `(define (f x y) body+)` and `(define f (lambda (x y) body+))` both
//! parse to the same `Define { name: "f", expr: Lambda { ... } }` AST.
//! The sugar form is handled at parse time so the IR compiler sees just
//! one shape.
//!
//! ## Pipeline
//!
//! ```text
//! Twig source
//!     │
//!     ▼  twig_lexer::tokenize
//! Vec<Token>
//!     │
//!     ▼  parse()                   ← THIS CRATE
//! Program (typed AST)
//!     │
//!     ▼  twig-ir-compiler
//! IIRModule
//! ```
//!
//! ## Example
//!
//! ```
//! use twig_parser::{parse, Form, Expr};
//!
//! let program = parse("(define x 42)").unwrap();
//! assert_eq!(program.forms.len(), 1);
//! match &program.forms[0] {
//!     Form::Define(d) => {
//!         assert_eq!(d.name, "x");
//!         assert!(matches!(d.expr, Expr::IntLit(_)));
//!     }
//!     _ => panic!("expected Define"),
//! }
//! ```

use std::fmt;

use twig_lexer::{tokenize, LexerError, Token, TokenKind};

// ---------------------------------------------------------------------------
// Section 1: AST node types
// ---------------------------------------------------------------------------
//
// The AST is a small, hand-rolled set of structs — one per semantic form.
// Every node carries `line` / `column` so error reporting can name the
// exact source location.  Boxed children (e.g. `If::cond`) are required
// because Rust enum variants would otherwise be infinitely sized.
// ---------------------------------------------------------------------------

/// An integer literal: `42`, `-7`, `0`.
///
/// The lexer guarantees the value fits the `-?[0-9]+` regex; we parse
/// into `i64` here and surface overflow as a `TwigParseError` so the
/// caller doesn't get a panic deep inside the compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntLit {
    pub value: i64,
    pub line: usize,
    pub column: usize,
}

/// A boolean literal: `#t` or `#f`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoolLit {
    pub value: bool,
    pub line: usize,
    pub column: usize,
}

/// The `nil` literal — empty list / null heap reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NilLit {
    pub line: usize,
    pub column: usize,
}

/// A quoted symbol: `'foo` or `(quote foo)`.
///
/// Both surface forms parse to the same node — the IR compiler only
/// sees the resulting symbol name, never the syntactic form that
/// produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymLit {
    pub name: String,
    pub line: usize,
    pub column: usize,
}

/// A bare name reference: `x`, `length`, `+`, `cons`.
///
/// At compile time this resolves to one of: a local (parameter or
/// `let` binding), a top-level function, a top-level value-global, or
/// a builtin.  Unresolved names raise a `TwigCompileError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarRef {
    pub name: String,
    pub line: usize,
    pub column: usize,
}

/// A `(if cond then else)` conditional.
///
/// Always ternary — Twig has no two-arm `if`.  Truthiness follows
/// Scheme semantics (only `#f` and `nil` are false; everything else
/// — including `0` and `nil`'s symbol counterpart — is true).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct If {
    pub cond: Box<Expr>,
    pub then_branch: Box<Expr>,
    pub else_branch: Box<Expr>,
    pub line: usize,
    pub column: usize,
}

/// A `(let ((x e1) (y e2) ...) body+)` form.
///
/// Bindings are mutually independent — Scheme `let`, not `let*`.  Each
/// RHS is evaluated in the *enclosing* scope, so peer binding names
/// are not yet visible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Let {
    pub bindings: Vec<(String, Expr)>,
    pub body: Vec<Expr>,
    pub line: usize,
    pub column: usize,
}

/// A `(begin e1 e2 ...)` sequencing form.  Returns the value of the
/// final expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Begin {
    pub exprs: Vec<Expr>,
    pub line: usize,
    pub column: usize,
}

/// An anonymous function: `(lambda (params*) body+)`.
///
/// The Rust parser does not perform free-variable analysis — that's
/// the IR compiler's job.  Here we just record the parameter list and
/// the body expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lambda {
    pub params: Vec<String>,
    pub body: Vec<Expr>,
    pub line: usize,
    pub column: usize,
}

/// A function application: `(fn arg0 arg1 ...)`.
///
/// The function position can itself be any expression, so higher-order
/// calls like `((compose f g) x)` parse without special-casing.  Zero
/// arguments is fine — `(thunk)` is a legitimate call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Apply {
    pub fn_expr: Box<Expr>,
    pub args: Vec<Expr>,
    pub line: usize,
    pub column: usize,
}

/// A top-level value or function binding: `(define name expr)`.
///
/// The function-sugar form `(define (f x) body)` parses to a `Define`
/// whose `expr` is a `Lambda`.  Downstream code therefore only ever
/// sees this single shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Define {
    pub name: String,
    pub expr: Expr,
    pub line: usize,
    pub column: usize,
}

/// Every Twig expression.
///
/// The variants line up 1:1 with the grammar's `expr | atom | quoted |
/// compound` productions.  Lifting these into a single enum lets the
/// compiler exhaustively `match` and have the type system flag any
/// unhandled variant on grammar growth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    IntLit(IntLit),
    BoolLit(BoolLit),
    NilLit(NilLit),
    SymLit(SymLit),
    VarRef(VarRef),
    If(If),
    Let(Let),
    Begin(Begin),
    Lambda(Lambda),
    Apply(Apply),
}

impl Expr {
    /// Return the source position `(line, column)` of this expression.
    pub fn pos(&self) -> (usize, usize) {
        match self {
            Expr::IntLit(n) => (n.line, n.column),
            Expr::BoolLit(b) => (b.line, b.column),
            Expr::NilLit(n) => (n.line, n.column),
            Expr::SymLit(s) => (s.line, s.column),
            Expr::VarRef(v) => (v.line, v.column),
            Expr::If(i) => (i.line, i.column),
            Expr::Let(l) => (l.line, l.column),
            Expr::Begin(b) => (b.line, b.column),
            Expr::Lambda(l) => (l.line, l.column),
            Expr::Apply(a) => (a.line, a.column),
        }
    }
}

/// A top-level form — either a `define` or a bare expression.
///
/// Bare top-level expressions accumulate into the synthesised `main`
/// function during compilation; the value of the *last* one becomes
/// the program's return value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Form {
    Define(Define),
    Expr(Expr),
}

/// A whole compilation unit — the ordered list of top-level forms.
///
/// An empty `Program` is valid; it compiles to a module whose `main`
/// returns `nil`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub forms: Vec<Form>,
}

// ---------------------------------------------------------------------------
// Section 2: Errors
// ---------------------------------------------------------------------------

/// Parse-time error.  Wraps either a malformed shape detected by the
/// parser or a lexer error bubbled up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwigParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for TwigParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TwigParseError at {}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for TwigParseError {}

impl From<LexerError> for TwigParseError {
    fn from(e: LexerError) -> Self {
        TwigParseError {
            message: format!("lexer error: {}", e.message),
            line: e.line,
            column: e.column,
        }
    }
}

// ---------------------------------------------------------------------------
// Section 3: Public API
// ---------------------------------------------------------------------------

/// Parse Twig source text into a [`Program`].
///
/// Performs lexing internally, so callers don't have to drag in
/// `twig-lexer` themselves.  Empty input returns a `Program` with
/// `forms = []` (still valid).
///
/// # Errors
///
/// Returns [`TwigParseError`] for any of: lexer error, unexpected
/// token, unmatched paren, malformed `define` / `let` / `lambda` /
/// `if` / `begin` / `quote` / binding, integer overflow.
///
/// # Example
///
/// ```
/// use twig_parser::{parse, Form};
///
/// let p = parse("(+ 1 2)").unwrap();
/// assert_eq!(p.forms.len(), 1);
/// assert!(matches!(&p.forms[0], Form::Expr(_)));
/// ```
pub fn parse(source: &str) -> Result<Program, TwigParseError> {
    let tokens = tokenize(source)?;
    parse_tokens(tokens)
}

/// Parse a pre-tokenised stream into a [`Program`].
///
/// Useful when the caller already has a token list (e.g. from a
/// language-server-style incremental lexer) and wants to skip
/// re-tokenisation.
///
/// The input must be terminated by exactly one [`TokenKind::Eof`] token
/// (as `tokenize()` always produces).  An empty vector or one missing
/// the trailing `Eof` is rejected with a `TwigParseError` rather than
/// panicking on out-of-bounds index — important for embedders that
/// expose this entry point to untrusted inputs.
pub fn parse_tokens(tokens: Vec<Token>) -> Result<Program, TwigParseError> {
    if !tokens.last().is_some_and(|t| t.kind == TokenKind::Eof) {
        return Err(TwigParseError {
            message: "token stream must be terminated by an Eof token".into(),
            line: 1,
            column: 1,
        });
    }
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

/// Maximum source-nesting depth the parser will accept.
///
/// Twig is recursive-descent: each `(` introduces one stack frame in
/// `parse_compound` / `parse_apply` / `parse_let` / etc.  Pathological
/// input like `(((...)))` with tens of thousands of opens would
/// exhaust the OS thread stack and abort the process — Rust does not
/// catch stack overflow.  We hard-cap depth at a value far larger
/// than any realistic Twig program but small enough to fit in a
/// default thread stack.
///
/// 256 levels easily cover hand-written code (the deepest hand-written
/// program in this repo is single digits) while keeping per-call
/// overhead negligible.  Increasing this requires verifying the
/// parser still fits inside macOS's 2 MiB non-main-thread stack.
pub const MAX_NESTING_DEPTH: usize = 256;

// ---------------------------------------------------------------------------
// Section 4: The recursive-descent parser
// ---------------------------------------------------------------------------
//
// One method per non-terminal, each consuming a contiguous token range
// and returning the typed AST node.  The parser carries a `pos` index
// into `tokens`; helpers `peek`, `advance`, `expect_*` and `at_eof`
// keep the cursor disciplined.
//
// Order in `parse_compound` matters: the keyword-led forms (`if`,
// `let`, …) are tried *before* the generic `apply`, otherwise a
// `(define ...)` would parse as an application of the `define`
// keyword to the rest.  We dispatch on the second token's kind +
// value to decide quickly.
// ---------------------------------------------------------------------------

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Current source-nesting depth.  Incremented on each recursive
    /// descent into a compound form; checked against [`MAX_NESTING_DEPTH`]
    /// to prevent stack-overflow DoS on adversarial input.
    depth: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0, depth: 0 }
    }

    /// Increment the depth counter; return an error if it exceeds the
    /// hard cap.  Pair every successful entry with a matching
    /// `self.depth -= 1` on the way out (or a scope guard).  We do
    /// this by hand rather than RAII so the error path can include
    /// the offending token's line/column.
    fn enter(&mut self) -> Result<(), TwigParseError> {
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            let t = self.peek();
            return Err(TwigParseError {
                message: format!(
                    "source nesting exceeds MAX_NESTING_DEPTH ({MAX_NESTING_DEPTH}) — \
                     refusing to recurse further to avoid stack overflow"
                ),
                line: t.line,
                column: t.column,
            });
        }
        Ok(())
    }

    fn leave(&mut self) {
        // Saturating to defend against accidental double-leave; a real
        // mismatch would already have been caught by the matching enter.
        self.depth = self.depth.saturating_sub(1);
    }

    // ------------------------------------------------------------------
    // Cursor helpers
    // ------------------------------------------------------------------

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        tok
    }

    fn at_eof(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    /// Consume a token of the given kind, or fail with a message that
    /// names what was expected and what was found.
    fn expect_kind(&mut self, kind: TokenKind, ctx: &str) -> Result<Token, TwigParseError> {
        if self.peek().kind == kind {
            Ok(self.advance())
        } else {
            let t = self.peek();
            Err(TwigParseError {
                message: format!(
                    "expected {kind} ({ctx}), got {} {:?}",
                    t.kind, t.value
                ),
                line: t.line,
                column: t.column,
            })
        }
    }

    /// Consume a `Keyword` token whose `value` exactly matches `word`.
    fn expect_keyword(&mut self, word: &str) -> Result<Token, TwigParseError> {
        let t = self.peek().clone();
        if t.kind == TokenKind::Keyword && t.value == word {
            Ok(self.advance())
        } else {
            Err(TwigParseError {
                message: format!("expected keyword {word:?}, got {} {:?}", t.kind, t.value),
                line: t.line,
                column: t.column,
            })
        }
    }

    // ------------------------------------------------------------------
    // Top level
    // ------------------------------------------------------------------

    fn parse_program(&mut self) -> Result<Program, TwigParseError> {
        let mut forms = Vec::new();
        while !self.at_eof() {
            forms.push(self.parse_form()?);
        }
        Ok(Program { forms })
    }

    fn parse_form(&mut self) -> Result<Form, TwigParseError> {
        // Look 1 token ahead: a `define` form must start with `(define ...)`.
        // Anything else is a bare expression.
        if self.peek().kind == TokenKind::LParen
            && self
                .peek_at(1)
                .is_some_and(|t| t.kind == TokenKind::Keyword && t.value == "define")
        {
            Ok(Form::Define(self.parse_define()?))
        } else {
            Ok(Form::Expr(self.parse_expr()?))
        }
    }

    // ------------------------------------------------------------------
    // (define ...)
    // ------------------------------------------------------------------

    fn parse_define(&mut self) -> Result<Define, TwigParseError> {
        let lparen = self.expect_kind(TokenKind::LParen, "start of (define ...)")?;
        let line = lparen.line;
        let column = lparen.column;
        self.expect_keyword("define")?;

        // The next token decides between value-define and function-sugar.
        match self.peek().kind {
            // (define name expr)
            TokenKind::Name => {
                let name_tok = self.advance();
                let expr = self.parse_expr()?;
                self.expect_kind(
                    TokenKind::RParen,
                    "(define name expr) takes exactly one body expression — \
                     use (define (name args...) body+) for multi-expression bodies",
                )?;
                Ok(Define { name: name_tok.value, expr, line, column })
            }

            // (define (name args*) body+)
            TokenKind::LParen => {
                let inner_lparen = self.advance();
                let fn_line = inner_lparen.line;
                let fn_col = inner_lparen.column;
                let fn_name_tok = self.expect_kind(
                    TokenKind::Name,
                    "function name in (define (name args...) ...)",
                )?;
                let mut params = Vec::new();
                while self.peek().kind == TokenKind::Name {
                    params.push(self.advance().value);
                }
                self.expect_kind(
                    TokenKind::RParen,
                    "end of parameter list in (define (name args...) ...)",
                )?;
                // body+ — at least one expression required.
                let mut body = vec![self.parse_expr()?];
                while self.peek().kind != TokenKind::RParen && !self.at_eof() {
                    body.push(self.parse_expr()?);
                }
                self.expect_kind(TokenKind::RParen, "end of (define ...)")?;
                let lam = Lambda {
                    params,
                    body,
                    line: fn_line,
                    column: fn_col,
                };
                Ok(Define {
                    name: fn_name_tok.value,
                    expr: Expr::Lambda(lam),
                    line,
                    column,
                })
            }

            _ => {
                let t = self.peek();
                Err(TwigParseError {
                    message: format!(
                        "(define ...) needs a name or (name args...), got {} {:?}",
                        t.kind, t.value
                    ),
                    line: t.line,
                    column: t.column,
                })
            }
        }
    }

    // ------------------------------------------------------------------
    // expr = atom | quoted | compound
    // ------------------------------------------------------------------

    fn parse_expr(&mut self) -> Result<Expr, TwigParseError> {
        // Single chokepoint for recursion: every compound form
        // bottoms out by calling parse_expr on its children, so
        // bounding depth here covers if/let/begin/lambda/quote/apply
        // and define-bodies in one place.
        self.enter()?;
        let result = self.parse_expr_inner();
        self.leave();
        result
    }

    fn parse_expr_inner(&mut self) -> Result<Expr, TwigParseError> {
        let tok = self.peek().clone();
        match tok.kind {
            // Atoms
            TokenKind::Integer => self.parse_integer(),
            TokenKind::BoolTrue => {
                self.advance();
                Ok(Expr::BoolLit(BoolLit { value: true, line: tok.line, column: tok.column }))
            }
            TokenKind::BoolFalse => {
                self.advance();
                Ok(Expr::BoolLit(BoolLit { value: false, line: tok.line, column: tok.column }))
            }
            TokenKind::Keyword if tok.value == "nil" => {
                self.advance();
                Ok(Expr::NilLit(NilLit { line: tok.line, column: tok.column }))
            }
            TokenKind::Name => {
                self.advance();
                Ok(Expr::VarRef(VarRef { name: tok.value, line: tok.line, column: tok.column }))
            }
            // Quoted: 'foo
            TokenKind::Quote => self.parse_quoted(),
            // Compound: (...)
            TokenKind::LParen => self.parse_compound(),
            // Anything else (RParen, Eof, an unexpected Keyword) is an error.
            _ => Err(TwigParseError {
                message: format!("unexpected token in expression: {} {:?}", tok.kind, tok.value),
                line: tok.line,
                column: tok.column,
            }),
        }
    }

    fn parse_integer(&mut self) -> Result<Expr, TwigParseError> {
        let tok = self.advance();
        let value: i64 = tok.value.parse().map_err(|_| TwigParseError {
            message: format!("integer literal {:?} does not fit in i64", tok.value),
            line: tok.line,
            column: tok.column,
        })?;
        Ok(Expr::IntLit(IntLit { value, line: tok.line, column: tok.column }))
    }

    fn parse_quoted(&mut self) -> Result<Expr, TwigParseError> {
        let q = self.expect_kind(TokenKind::Quote, "quote prefix")?;
        let name_tok = self.expect_kind(TokenKind::Name, "name following '")?;
        Ok(Expr::SymLit(SymLit {
            name: name_tok.value,
            line: q.line,
            column: q.column,
        }))
    }

    // ------------------------------------------------------------------
    // compound = if | let | begin | lambda | quote_form | apply
    // ------------------------------------------------------------------

    fn parse_compound(&mut self) -> Result<Expr, TwigParseError> {
        // We've peeked at LPAREN — peek one further to decide which compound
        // form we have.  The keyword forms each consume the LPAREN themselves.
        let next = self.peek_at(1).cloned();

        if let Some(t) = next {
            if t.kind == TokenKind::Keyword {
                match t.value.as_str() {
                    "if" => return self.parse_if(),
                    "let" => return self.parse_let(),
                    "begin" => return self.parse_begin(),
                    "lambda" => return self.parse_lambda(),
                    "quote" => return self.parse_quote_form(),
                    // `define` only appears at the top level — treat a
                    // nested one as a parse error rather than letting it
                    // fall through to apply.
                    "define" => {
                        return Err(TwigParseError {
                            message: "(define ...) is only allowed at the top level".into(),
                            line: t.line,
                            column: t.column,
                        });
                    }
                    "nil" => { /* fall through — nil-as-fn is just an error in apply */ }
                    other => {
                        return Err(TwigParseError {
                            message: format!("unexpected keyword {other:?} in compound form"),
                            line: t.line,
                            column: t.column,
                        });
                    }
                }
            }
        }

        // Generic application
        self.parse_apply()
    }

    fn parse_if(&mut self) -> Result<Expr, TwigParseError> {
        let lp = self.expect_kind(TokenKind::LParen, "start of (if ...)")?;
        self.expect_keyword("if")?;
        let cond = self.parse_expr()?;
        let then_branch = self.parse_expr()?;
        let else_branch = self.parse_expr()?;
        // Reject (if c t e extra) explicitly so users see a clear message.
        if self.peek().kind != TokenKind::RParen {
            let t = self.peek();
            return Err(TwigParseError {
                message: format!(
                    "(if ...) takes exactly 3 expressions; unexpected {} {:?}",
                    t.kind, t.value
                ),
                line: t.line,
                column: t.column,
            });
        }
        self.expect_kind(TokenKind::RParen, "end of (if ...)")?;
        Ok(Expr::If(If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
            line: lp.line,
            column: lp.column,
        }))
    }

    fn parse_let(&mut self) -> Result<Expr, TwigParseError> {
        let lp = self.expect_kind(TokenKind::LParen, "start of (let ...)")?;
        self.expect_keyword("let")?;
        // Bindings list: (LPAREN { binding } RPAREN)
        self.expect_kind(TokenKind::LParen, "start of (let ((x e)...) ...) bindings")?;
        let mut bindings = Vec::new();
        while self.peek().kind == TokenKind::LParen {
            bindings.push(self.parse_binding()?);
        }
        self.expect_kind(TokenKind::RParen, "end of (let ...) bindings")?;
        // body+ — at least one expression
        let mut body = vec![self.parse_expr()?];
        while self.peek().kind != TokenKind::RParen && !self.at_eof() {
            body.push(self.parse_expr()?);
        }
        self.expect_kind(TokenKind::RParen, "end of (let ...)")?;
        Ok(Expr::Let(Let { bindings, body, line: lp.line, column: lp.column }))
    }

    fn parse_binding(&mut self) -> Result<(String, Expr), TwigParseError> {
        self.expect_kind(TokenKind::LParen, "start of binding (name expr)")?;
        let name_tok = self.expect_kind(TokenKind::Name, "binding name")?;
        let expr = self.parse_expr()?;
        self.expect_kind(TokenKind::RParen, "end of binding")?;
        Ok((name_tok.value, expr))
    }

    fn parse_begin(&mut self) -> Result<Expr, TwigParseError> {
        let lp = self.expect_kind(TokenKind::LParen, "start of (begin ...)")?;
        self.expect_keyword("begin")?;
        let mut exprs = vec![self.parse_expr()?];
        while self.peek().kind != TokenKind::RParen && !self.at_eof() {
            exprs.push(self.parse_expr()?);
        }
        self.expect_kind(TokenKind::RParen, "end of (begin ...)")?;
        Ok(Expr::Begin(Begin { exprs, line: lp.line, column: lp.column }))
    }

    fn parse_lambda(&mut self) -> Result<Expr, TwigParseError> {
        let lp = self.expect_kind(TokenKind::LParen, "start of (lambda ...)")?;
        self.expect_keyword("lambda")?;
        // Parameter list
        self.expect_kind(TokenKind::LParen, "start of (lambda (params) ...)")?;
        let mut params = Vec::new();
        while self.peek().kind == TokenKind::Name {
            params.push(self.advance().value);
        }
        self.expect_kind(TokenKind::RParen, "end of (lambda (params) ...)")?;
        // body+ — at least one expression
        let mut body = vec![self.parse_expr()?];
        while self.peek().kind != TokenKind::RParen && !self.at_eof() {
            body.push(self.parse_expr()?);
        }
        self.expect_kind(TokenKind::RParen, "end of (lambda ...)")?;
        Ok(Expr::Lambda(Lambda { params, body, line: lp.line, column: lp.column }))
    }

    fn parse_quote_form(&mut self) -> Result<Expr, TwigParseError> {
        let lp = self.expect_kind(TokenKind::LParen, "start of (quote name)")?;
        self.expect_keyword("quote")?;
        let name_tok = self.expect_kind(TokenKind::Name, "name in (quote name)")?;
        self.expect_kind(TokenKind::RParen, "end of (quote name)")?;
        Ok(Expr::SymLit(SymLit { name: name_tok.value, line: lp.line, column: lp.column }))
    }

    fn parse_apply(&mut self) -> Result<Expr, TwigParseError> {
        let lp = self.expect_kind(TokenKind::LParen, "start of application")?;
        // Empty `()` is an error — `nil` should be used for empty list.
        if self.peek().kind == TokenKind::RParen {
            return Err(TwigParseError {
                message: "empty application '()' — use 'nil' for the empty list".into(),
                line: lp.line,
                column: lp.column,
            });
        }
        let fn_expr = self.parse_expr()?;
        let mut args = Vec::new();
        while self.peek().kind != TokenKind::RParen && !self.at_eof() {
            args.push(self.parse_expr()?);
        }
        self.expect_kind(TokenKind::RParen, "end of application")?;
        Ok(Expr::Apply(Apply {
            fn_expr: Box::new(fn_expr),
            args,
            line: lp.line,
            column: lp.column,
        }))
    }
}

// ---------------------------------------------------------------------------
// Section 5: Tests
// ---------------------------------------------------------------------------
//
// Tests cover the same surface as the Python `tests/test_parser.py`:
// every expression form, define-sugar, error paths.  Where useful we
// inspect AST shapes via direct match arms — there's no `as_dict`
// helper, so the tests double as documentation for the AST structure.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn first_expr(src: &str) -> Expr {
        let p = parse(src).unwrap();
        match p.forms.into_iter().next().unwrap() {
            Form::Expr(e) => e,
            Form::Define(_) => panic!("expected an expression form, got define"),
        }
    }

    // -- Empty input --

    #[test]
    fn empty_program() {
        let p = parse("").unwrap();
        assert!(p.forms.is_empty());
    }

    #[test]
    fn whitespace_only_program() {
        let p = parse("\n\n  ; comment\n").unwrap();
        assert!(p.forms.is_empty());
    }

    // -- Atoms --

    #[test]
    fn integer_literal() {
        match first_expr("42") {
            Expr::IntLit(IntLit { value, .. }) => assert_eq!(value, 42),
            other => panic!("expected IntLit, got {other:?}"),
        }
    }

    #[test]
    fn negative_integer_literal() {
        match first_expr("-7") {
            Expr::IntLit(IntLit { value, .. }) => assert_eq!(value, -7),
            other => panic!("expected IntLit, got {other:?}"),
        }
    }

    #[test]
    fn integer_overflow_errors() {
        let err = parse("99999999999999999999").unwrap_err();
        assert!(err.message.contains("does not fit in i64"));
    }

    #[test]
    fn bool_true_literal() {
        match first_expr("#t") {
            Expr::BoolLit(b) => assert!(b.value),
            other => panic!("expected BoolLit(true), got {other:?}"),
        }
    }

    #[test]
    fn bool_false_literal() {
        match first_expr("#f") {
            Expr::BoolLit(b) => assert!(!b.value),
            other => panic!("expected BoolLit(false), got {other:?}"),
        }
    }

    #[test]
    fn nil_literal() {
        assert!(matches!(first_expr("nil"), Expr::NilLit(_)));
    }

    #[test]
    fn name_reference() {
        match first_expr("foo") {
            Expr::VarRef(v) => assert_eq!(v.name, "foo"),
            other => panic!("expected VarRef, got {other:?}"),
        }
    }

    #[test]
    fn operator_name() {
        match first_expr("+") {
            Expr::VarRef(v) => assert_eq!(v.name, "+"),
            other => panic!("expected VarRef, got {other:?}"),
        }
    }

    // -- Quoted symbols --

    #[test]
    fn quoted_symbol_short_form() {
        match first_expr("'foo") {
            Expr::SymLit(s) => assert_eq!(s.name, "foo"),
            other => panic!("expected SymLit, got {other:?}"),
        }
    }

    #[test]
    fn quoted_symbol_long_form() {
        match first_expr("(quote bar)") {
            Expr::SymLit(s) => assert_eq!(s.name, "bar"),
            other => panic!("expected SymLit, got {other:?}"),
        }
    }

    // -- Apply --

    #[test]
    fn simple_application() {
        match first_expr("(+ 1 2)") {
            Expr::Apply(a) => {
                assert_eq!(a.args.len(), 2);
                assert!(matches!(*a.fn_expr, Expr::VarRef(_)));
            }
            other => panic!("expected Apply, got {other:?}"),
        }
    }

    #[test]
    fn nested_application() {
        let e = first_expr("(+ (* 2 3) 4)");
        match e {
            Expr::Apply(a) => {
                assert_eq!(a.args.len(), 2);
                assert!(matches!(a.args[0], Expr::Apply(_)));
                assert!(matches!(a.args[1], Expr::IntLit(_)));
            }
            other => panic!("expected Apply, got {other:?}"),
        }
    }

    #[test]
    fn higher_order_call() {
        // ((make-adder 5) 3) — first position is itself an Apply
        match first_expr("((make-adder 5) 3)") {
            Expr::Apply(a) => {
                assert!(matches!(*a.fn_expr, Expr::Apply(_)));
                assert_eq!(a.args.len(), 1);
            }
            other => panic!("expected Apply, got {other:?}"),
        }
    }

    #[test]
    fn empty_application_is_error() {
        let err = parse("()").unwrap_err();
        assert!(err.message.contains("empty application"));
    }

    // -- if --

    #[test]
    fn if_form() {
        match first_expr("(if #t 1 2)") {
            Expr::If(i) => {
                assert!(matches!(*i.cond, Expr::BoolLit(_)));
                assert!(matches!(*i.then_branch, Expr::IntLit(_)));
                assert!(matches!(*i.else_branch, Expr::IntLit(_)));
            }
            other => panic!("expected If, got {other:?}"),
        }
    }

    #[test]
    fn if_with_too_few_args_errors() {
        let err = parse("(if #t 1)").unwrap_err();
        // Either reports the trailing RParen as unexpected in the else
        // position, or as the missing third arg — either is acceptable.
        assert!(!err.message.is_empty());
    }

    #[test]
    fn if_with_too_many_args_errors() {
        let err = parse("(if #t 1 2 3)").unwrap_err();
        assert!(err.message.contains("exactly 3 expressions"));
    }

    // -- let --

    #[test]
    fn let_with_one_binding() {
        match first_expr("(let ((x 1)) x)") {
            Expr::Let(l) => {
                assert_eq!(l.bindings.len(), 1);
                assert_eq!(l.bindings[0].0, "x");
                assert_eq!(l.body.len(), 1);
            }
            other => panic!("expected Let, got {other:?}"),
        }
    }

    #[test]
    fn let_with_two_bindings_and_multi_body() {
        match first_expr("(let ((x 1) (y 2)) (+ x y) (* x y))") {
            Expr::Let(l) => {
                assert_eq!(l.bindings.len(), 2);
                assert_eq!(l.body.len(), 2);
            }
            other => panic!("expected Let, got {other:?}"),
        }
    }

    #[test]
    fn let_with_zero_bindings_and_body_is_legal() {
        match first_expr("(let () 1)") {
            Expr::Let(l) => {
                assert!(l.bindings.is_empty());
                assert_eq!(l.body.len(), 1);
            }
            other => panic!("expected Let, got {other:?}"),
        }
    }

    // -- begin --

    #[test]
    fn begin_form() {
        match first_expr("(begin 1 2 3)") {
            Expr::Begin(b) => assert_eq!(b.exprs.len(), 3),
            other => panic!("expected Begin, got {other:?}"),
        }
    }

    // -- lambda --

    #[test]
    fn lambda_with_no_params() {
        match first_expr("(lambda () 42)") {
            Expr::Lambda(l) => {
                assert!(l.params.is_empty());
                assert_eq!(l.body.len(), 1);
            }
            other => panic!("expected Lambda, got {other:?}"),
        }
    }

    #[test]
    fn lambda_with_params() {
        match first_expr("(lambda (x y) (+ x y))") {
            Expr::Lambda(l) => {
                assert_eq!(l.params, vec!["x".to_string(), "y".to_string()]);
                assert_eq!(l.body.len(), 1);
            }
            other => panic!("expected Lambda, got {other:?}"),
        }
    }

    // -- define --

    #[test]
    fn define_value() {
        let p = parse("(define x 42)").unwrap();
        match &p.forms[0] {
            Form::Define(d) => {
                assert_eq!(d.name, "x");
                assert!(matches!(d.expr, Expr::IntLit(_)));
            }
            other => panic!("expected Define, got {other:?}"),
        }
    }

    #[test]
    fn define_function_sugar_lowers_to_lambda() {
        let p = parse("(define (square x) (* x x))").unwrap();
        match &p.forms[0] {
            Form::Define(d) => {
                assert_eq!(d.name, "square");
                match &d.expr {
                    Expr::Lambda(l) => {
                        assert_eq!(l.params, vec!["x".to_string()]);
                        assert_eq!(l.body.len(), 1);
                    }
                    other => panic!("expected Lambda inside Define, got {other:?}"),
                }
            }
            other => panic!("expected Define, got {other:?}"),
        }
    }

    #[test]
    fn define_function_with_multi_expr_body() {
        let p = parse("(define (f x) (+ x 1) (* x 2))").unwrap();
        match &p.forms[0] {
            Form::Define(d) => match &d.expr {
                Expr::Lambda(l) => assert_eq!(l.body.len(), 2),
                _ => panic!("expected Lambda"),
            },
            _ => panic!("expected Define"),
        }
    }

    #[test]
    fn define_value_rejects_extra_body_expr() {
        let err = parse("(define x 1 2)").unwrap_err();
        assert!(err.message.contains("(define name expr)"));
    }

    #[test]
    fn nested_define_is_an_error() {
        // (define) inside an expression is rejected — defines are top-level only.
        let err = parse("(let () (define y 1))").unwrap_err();
        assert!(err.message.contains("only allowed at the top level"));
    }

    // -- Multiple top-level forms --

    #[test]
    fn multiple_top_level_forms() {
        let p = parse("(define x 1) (define y 2) (+ x y)").unwrap();
        assert_eq!(p.forms.len(), 3);
        assert!(matches!(&p.forms[0], Form::Define(_)));
        assert!(matches!(&p.forms[1], Form::Define(_)));
        assert!(matches!(&p.forms[2], Form::Expr(_)));
    }

    // -- Position tracking --

    #[test]
    fn integer_position_recorded() {
        let e = first_expr("\n\n  42");
        match e {
            Expr::IntLit(i) => {
                assert_eq!(i.line, 3);
                assert_eq!(i.column, 3);
            }
            other => panic!("expected IntLit, got {other:?}"),
        }
    }

    #[test]
    fn apply_position_at_lparen() {
        let e = first_expr("  (+ 1 2)");
        match e {
            Expr::Apply(a) => {
                assert_eq!(a.line, 1);
                assert_eq!(a.column, 3);
            }
            other => panic!("expected Apply, got {other:?}"),
        }
    }

    // -- Errors --

    #[test]
    fn unmatched_lparen_errors() {
        assert!(parse("(+ 1 2").is_err());
    }

    #[test]
    fn lone_rparen_errors() {
        assert!(parse(")").is_err());
    }

    #[test]
    fn lexer_error_propagates() {
        let err = parse("@").unwrap_err();
        assert!(err.message.contains("lexer error"));
    }

    // -- Realistic full program --

    #[test]
    fn factorial_program_parses() {
        let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))\n(fact 5)";
        let p = parse(src).unwrap();
        assert_eq!(p.forms.len(), 2);
        match &p.forms[0] {
            Form::Define(d) => {
                assert_eq!(d.name, "fact");
                assert!(matches!(d.expr, Expr::Lambda(_)));
            }
            _ => panic!("expected Define"),
        }
    }

    // -- Display --

    #[test]
    fn parse_error_display_includes_position() {
        let err = parse(")").unwrap_err();
        let s = format!("{err}");
        assert!(s.contains("TwigParseError"));
        assert!(s.contains("1:1"));
    }

    // -- Defense in depth: stack-overflow guard --

    #[test]
    fn extreme_nesting_returns_error_not_panic() {
        // Build "(((((...)))))" with depth far above MAX_NESTING_DEPTH.
        // Without the depth cap this would blow the OS thread stack;
        // with it, we get a clean TwigParseError.
        let src = format!(
            "{open}{close}",
            open = "(".repeat(MAX_NESTING_DEPTH + 10),
            close = ")".repeat(MAX_NESTING_DEPTH + 10),
        );
        let err = parse(&src).unwrap_err();
        assert!(
            err.message.contains("MAX_NESTING_DEPTH"),
            "expected depth-cap error, got: {err}"
        );
    }

    #[test]
    fn nesting_below_cap_still_works() {
        // 50 levels — well under the cap.  This parses as nested
        // zero-arg applies, all the way down to `(+ 1)` at the bottom.
        // Sanity check: the depth gate is not over-aggressive on
        // well-formed but heavily-nested input.
        let src = format!(
            "{open}+ 1{close}",
            open = "(".repeat(50),
            close = ")".repeat(50),
        );
        let p = parse(&src).expect("50-deep nest should parse fine");
        assert_eq!(p.forms.len(), 1);
    }

    // -- parse_tokens validates Eof terminator --

    #[test]
    fn parse_tokens_rejects_empty_vector() {
        let err = parse_tokens(Vec::new()).unwrap_err();
        assert!(err.message.contains("Eof"));
    }

    #[test]
    fn parse_tokens_rejects_missing_eof() {
        // A single LParen with no Eof — without validation this would
        // panic on out-of-bounds index when peek runs off the end.
        let toks = vec![Token { kind: TokenKind::LParen, value: "(".into(), line: 1, column: 1 }];
        let err = parse_tokens(toks).unwrap_err();
        assert!(err.message.contains("Eof"));
    }

    #[test]
    fn parse_tokens_accepts_well_formed_input() {
        // A complete tokenisation goes through without complaint.
        let toks = twig_lexer::tokenize("(+ 1 2)").unwrap();
        assert!(parse_tokens(toks).is_ok());
    }
}
