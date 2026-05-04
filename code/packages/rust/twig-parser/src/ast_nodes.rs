//! Typed AST nodes for Twig.
//!
//! ## Why a typed AST on top of the generic `GrammarASTNode`?
//!
//! The grammar-driven parser ([`parser::grammar_parser::GrammarParser`])
//! produces a generic [`GrammarASTNode`] tree.  Each node carries a
//! `rule_name` string and a heterogeneous `children` list mixing nested
//! nodes with raw [`Token`]s.  Walking that tree directly in a downstream
//! compiler means a sea of `rule_name == "..."` checks, lossy dispatch,
//! and no static guarantees about which fields each form actually has.
//!
//! Twig has eight semantic forms (`if` / `let` / `begin` / `lambda` /
//! `quote` / `apply` / `define` / value-`define`).  Lifting the generic
//! AST into the dataclass-style structs here ([`If`], [`Lambda`],
//! [`Apply`], …) gives the IR compiler an exhaustive `match` over a
//! small set of variants — each variant carries exactly the fields it
//! needs.  This is the same pattern used by the Python `twig` package's
//! `ast_nodes.py` and `ast_extract.py`.
//!
//! Source positions (`line` / `column`) are carried on every node so the
//! IR compiler can emit position-tagged error messages.
//!
//! [`GrammarASTNode`]: parser::grammar_parser::GrammarASTNode
//! [`Token`]: lexer::token::Token

// ---------------------------------------------------------------------------
// Atoms
// ---------------------------------------------------------------------------

/// An integer literal: `42`, `-7`, `0`.
///
/// The lexer guarantees the source matches `-?[0-9]+`; the extractor
/// parses into `i64` and surfaces overflow as a [`crate::TwigParseError`].
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

/// A quoted symbol: `'foo` or `(quote foo)`.  Both surface forms collapse
/// to this one variant — the IR compiler only sees the resulting symbol
/// name, never the syntactic form that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymLit {
    pub name: String,
    pub line: usize,
    pub column: usize,
}

/// A bare name reference: `x`, `length`, `+`, `cons`.  Resolution to
/// local / global / builtin happens at compile time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarRef {
    pub name: String,
    pub line: usize,
    pub column: usize,
}

// ---------------------------------------------------------------------------
// Compound forms
// ---------------------------------------------------------------------------

/// `(if cond then else)` — always ternary.  Twig has no two-arm `if`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct If {
    pub cond: Box<Expr>,
    pub then_branch: Box<Expr>,
    pub else_branch: Box<Expr>,
    pub line: usize,
    pub column: usize,
}

/// `(let ((x e1) ...) body+)` with mutually-independent bindings —
/// Scheme `let`, not `let*`.  Each RHS evaluates in the *outer* scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Let {
    pub bindings: Vec<(String, Expr)>,
    pub body: Vec<Expr>,
    pub line: usize,
    pub column: usize,
}

/// `(begin e1 e2 ...)` — sequencing.  Returns the value of the final expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Begin {
    pub exprs: Vec<Expr>,
    pub line: usize,
    pub column: usize,
}

/// `(lambda (params*) body+)` — anonymous function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lambda {
    pub params: Vec<String>,
    pub body: Vec<Expr>,
    pub line: usize,
    pub column: usize,
}

/// `(fn arg0 arg1 ...)` — function application.
///
/// The function position can itself be any expression, so higher-order
/// calls like `((compose f g) x)` parse without special-casing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Apply {
    pub fn_expr: Box<Expr>,
    pub args: Vec<Expr>,
    pub line: usize,
    pub column: usize,
}

// ---------------------------------------------------------------------------
// Top-level forms
// ---------------------------------------------------------------------------

/// `(define name expr)` — value or function binding.
///
/// The function-sugar form `(define (f x) body)` is lowered to
/// `Define { name: "f", expr: Lambda { ... } }` during AST extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Define {
    pub name: String,
    pub expr: Expr,
    pub line: usize,
    pub column: usize,
}

/// Every Twig expression — variants line up 1:1 with the grammar's
/// `expr | atom | quoted | compound` productions.
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

/// A top-level form — either a `(define ...)` or a bare expression.
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
