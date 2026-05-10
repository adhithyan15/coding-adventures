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
//! ## LANG23 PR 23-E — refinement type annotations
//!
//! [`TypeAnnotation`] represents the LANG23 v1 predicate vocabulary as
//! parsed from Twig source.  Annotations appear in two positions:
//!
//! - **Value bindings**: `(define x : (Int 0 128) 42)` — the annotation is
//!   carried on [`Define::type_annotation`].
//! - **Function parameters**: `(define (f (x : (Int 0 128))) ...)` — each
//!   element of [`Lambda::param_annotations`] corresponds to the same-index
//!   element of [`Lambda::params`].
//! - **Return types**: `(define (f x -> (Int 0 256)) ...)` — the annotation
//!   is on [`Lambda::return_annotation`].
//!
//! All annotation fields default to `None`, so unannotated code is unchanged.
//!
//! Source positions (`line` / `column`) are carried on every node so the
//! IR compiler can emit position-tagged error messages.
//!
//! [`GrammarASTNode`]: parser::grammar_parser::GrammarASTNode
//! [`Token`]: lexer::token::Token

// ---------------------------------------------------------------------------
// LANG23 PR 23-E — type annotations
// ---------------------------------------------------------------------------

/// A LANG23 v1 type annotation parsed from Twig source.
///
/// The annotation vocabulary is a strict subset of `lang_refined_types::Predicate`
/// that the Twig parser can express syntactically.  The IR compiler converts
/// these to [`lang_refined_types::RefinedType`] values when populating
/// `IIRFunction::param_refinements` / `IIRFunction::return_refinement`.
///
/// # Syntax
///
/// | Twig syntax          | Variant                              | Semantics              |
/// |----------------------|--------------------------------------|------------------------|
/// | `int`                | `UnrefinedInt`                       | any integer            |
/// | `any`                | `Any`                                | any value              |
/// | `bool`               | `UnrefinedBool`                      | any boolean            |
/// | `(Int lo hi)`        | `RangeInt { lo, hi }`                | `lo ≤ x < hi`          |
/// | `(Member int (v…))`  | `MembershipInt { values }`           | `x ∈ {v₀, v₁, …}`    |
///
/// `RangeInt` always uses an *exclusive* upper bound (matching the spec's
/// `(Int 0 256)` = `[0, 256)` convention).  No other bound combination is
/// expressible in the v1 syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeAnnotation {
    /// Unrefined integer — any `int`-kinded value.
    ///
    /// Written as `int` in Twig source.  Lowers to
    /// `RefinedType::unrefined(Kind::Int)`.
    UnrefinedInt,

    /// Unrefined `any` — the top type, admits any value.
    ///
    /// Written as `any` in Twig source.  Lowers to
    /// `RefinedType::unrefined(Kind::Any)`.
    Any,

    /// Unrefined boolean — any `bool`-kinded value.
    ///
    /// Written as `bool` in Twig source.  Lowers to
    /// `RefinedType::unrefined(Kind::Bool)`.
    UnrefinedBool,

    /// Integer range annotation: `(Int lo hi)`.
    ///
    /// Semantics: `lo ≤ x` and `x < hi` (exclusive upper bound).
    /// Lowers to `RefinedType::refined(Kind::Int, Predicate::Range { lo: Some(lo), hi: Some(hi), inclusive_hi: false })`.
    RangeInt { lo: i128, hi: i128 },

    /// Integer membership annotation: `(Member int (v0 v1 ...))`.
    ///
    /// Semantics: `x ∈ {values}`.
    /// Lowers to `RefinedType::refined(Kind::Int, Predicate::Membership { values })`.
    MembershipInt { values: Vec<i128> },
}

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
///
/// For anonymous lambdas (from the `(lambda ...)` form), `param_annotations`
/// is all `None` and `return_annotation` is `None` — the v1 annotation
/// syntax only applies to top-level `define` function sugar.  Fields are
/// kept on the struct so the IR compiler can use the same lowering path for
/// both annotated defines and unannotated lambdas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lambda {
    pub params: Vec<String>,
    /// Per-parameter type annotation, in lockstep with `params`.
    ///
    /// `None` at position `i` means parameter `i` is unannotated.
    /// Populated by the AST extractor when it encounters
    /// `(define (f (x : TypeAnnotation) ...) ...)` function sugar.
    pub param_annotations: Vec<Option<TypeAnnotation>>,
    /// Optional return-type annotation.
    ///
    /// Populated by the extractor when the signature contains
    /// `-> type_annotation` inside the parameter list parentheses.
    pub return_annotation: Option<TypeAnnotation>,
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
///
/// For LANG23 annotated value bindings like `(define x : (Int 0 128) 42)`,
/// `type_annotation` holds the parsed annotation.  For function defines
/// like `(define (f (x : (Int 0 128))) body)`, the annotation is embedded
/// in the `Lambda` node (see [`Lambda::param_annotations`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Define {
    pub name: String,
    /// LANG23 PR 23-E: optional type annotation for value bindings.
    ///
    /// `Some(ann)` when the source reads `(define x : ann value)`.
    /// `None` for all unannotated defines and for function defines
    /// (which carry their annotations in the nested `Lambda` node).
    pub type_annotation: Option<TypeAnnotation>,
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
