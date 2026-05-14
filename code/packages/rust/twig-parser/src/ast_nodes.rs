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
//! ## LANG48 / TW05-A — typed syntax extensions
//!
//! LANG48 adds first-class typed-Twig forms to the AST:
//!
//! - [`TypeExpr`] — a general s-expression type representation (name, integer,
//!   or parenthesised list of sub-expressions).  Covers all TW05 annotation
//!   vocabulary that [`TypeAnnotation`]'s LANG23 subset doesn't handle.
//! - [`TypeAnnotation::Opaque`] — fallback for type expressions not in the
//!   LANG23 restricted set; stores a [`TypeExpr`] for TW05-B to interpret.
//! - [`TypedMode`] / [`ModuleInfo`] — capture the `(typed strict/lenient/off)`
//!   module clause.
//! - [`TypeAlias`], [`RecordDef`], [`UnionDef`] — new top-level declaration
//!   forms; accessible as [`Form::TypeAlias`], [`Form::RecordDef`],
//!   [`Form::UnionDef`].
//! - [`Match`] / [`MatchArm`] / [`MatchPat`] — pattern-matching expression;
//!   accessible as [`Expr::Match`].
//! - [`Program::module_info`] — optional module metadata (name, typed mode,
//!   exports, imports).
//!
//! Source positions (`line` / `column`) are carried on every node so the
//! IR compiler can emit position-tagged error messages.
//!
//! [`GrammarASTNode`]: parser::grammar_parser::GrammarASTNode
//! [`Token`]: lexer::token::Token

// ---------------------------------------------------------------------------
// LANG48 / TW05-A — general type expression
// ---------------------------------------------------------------------------

/// A general s-expression type representation (LANG48 / TW05-A).
///
/// Covers any type expression that appears in typed Twig source but is not
/// one of the LANG23 narrowly-shaped [`TypeAnnotation`] variants.  The
/// TW05-B type checker will interpret these; TW05-A erases them.
///
/// # Structure
///
/// A `TypeExpr` mirrors the grammar rule:
/// ```text
/// type_annotation = LPAREN { type_annotation } RPAREN | NAME | INTEGER ;
/// ```
///
/// Examples:
/// - `Nat`                       → `Name("Nat")`
/// - `0`                         → `Int(0)`
/// - `(Int 0 128)`               → `List([Name("Int"), Int(0), Int(128)])`
/// - `(Index source-len)`        → `List([Name("Index"), Name("source-len")])`
/// - `(fn (n) (Int 0 n))`        → `List([Name("fn"), List([Name("n")]), List([Name("Int"), Int(0), Name("n")])])`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr {
    /// A bare name reference in type position: `Nat`, `String`, `_`, etc.
    Name(String),
    /// An integer literal in type position: `0`, `128`, etc.
    Int(i64),
    /// A parenthesised list of sub-expressions: `(Int 0 128)`, `(List T)`, …
    List(Vec<TypeExpr>),
}

// ---------------------------------------------------------------------------
// LANG23 PR 23-E — type annotations (extended for LANG48)
// ---------------------------------------------------------------------------

/// A LANG23 v1 type annotation parsed from Twig source, extended in LANG48
/// with an [`Opaque`] fallback for TW05-A type expressions.
///
/// The annotation vocabulary is a strict subset of `lang_refined_types::Predicate`
/// that the Twig parser can express syntactically.  The IR compiler converts
/// these to [`lang_refined_types::RefinedType`] values when populating
/// `IIRFunction::param_refinements` / `IIRFunction::return_refinement`.
///
/// # LANG48 extension
///
/// The LANG23 grammar only accepted three forms.  LANG48 replaces the grammar
/// with a fully general recursive s-expression grammar.  The extractor still
/// recognises the three LANG23 forms and produces the specific variants below;
/// any other type expression is stored as [`Opaque`] for TW05-B.
///
/// # Syntax
///
/// | Twig syntax                   | Variant                              | Semantics                       |
/// |-------------------------------|--------------------------------------|----------------------------------|
/// | `int`                         | `UnrefinedInt`                       | any integer                      |
/// | `any`                         | `Any`                                | any value                        |
/// | `bool`                        | `UnrefinedBool`                      | any boolean                      |
/// | `(Int lo hi)`                 | `RangeInt { lo, hi }`                | `lo ≤ x < hi`                   |
/// | `(Member int (v…))`           | `MembershipInt { values }`           | `x ∈ {v₀, v₁, …}`              |
/// | anything else (TW05-A)        | `Opaque(TypeExpr)`                   | stored for TW05-B interpretation |
///
/// [`Opaque`]: TypeAnnotation::Opaque
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

    /// Opaque TW05-A type expression — anything not in the LANG23 subset.
    ///
    /// Stored as a [`TypeExpr`] tree for the TW05-B type checker to
    /// interpret.  Erased (treated as `Any`) during TW05-A IIR lowering.
    Opaque(TypeExpr),
}

// ---------------------------------------------------------------------------
// LANG48 / TW05-A — module metadata
// ---------------------------------------------------------------------------

/// The typing mode for a Twig module (TW05-A / LANG48).
///
/// Set by the `(typed …)` clause inside a `(module …)` declaration:
///
/// | Source         | Variant   | Enforcement                                             |
/// |----------------|-----------|--------------------------------------------------------|
/// | `(typed off)`  | `Off`     | Dynamic Twig — current behaviour (no type checking).   |
/// | `(typed lenient)` | `Lenient` | Type + refinement annotations checked; unknown refinement outcomes → runtime checks. |
/// | `(typed strict)` | `Strict` | No public `any`, unknown refinements are compile errors.|
///
/// In TW05-A this is recorded but not enforced — enforcement comes in TW05-H.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedMode {
    /// No typing — current dynamic Twig behaviour.
    Off,
    /// Type-annotated; unknown refinements become runtime checks.
    Lenient,
    /// Strict typed mode; unknown refinements are compile errors.
    Strict,
}

/// Module-level metadata extracted from a `(module name …)` declaration.
///
/// Populated by the AST extractor when the source starts with a
/// `(module …)` form.  Programs without a module declaration have
/// `module_info = None` and get an implicit "default" module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInfo {
    /// Module path name, e.g. `"compiler/lexer"`.
    pub name: String,
    /// Optional `(typed …)` mode — `None` if no `(typed …)` clause.
    pub typed_mode: Option<TypedMode>,
    /// Names listed in `(export …)` clauses.
    pub exports: Vec<String>,
    /// Module paths listed in `(import …)` clauses.
    pub imports: Vec<String>,
}

// ---------------------------------------------------------------------------
// LANG48 / TW05-A — type alias
// ---------------------------------------------------------------------------

/// `(type Name type_expr)` — a compile-time type alias (TW05-A / LANG48).
///
/// The alias expands before IIR emission — `twig-ir-compiler` treats
/// `Form::TypeAlias` as a no-op in TW05-A.  The TW05-B type checker uses
/// the alias to expand names before checking.
///
/// Examples:
/// - `(type Nat (Int 0 _))`
/// - `(type Index (fn (len) (Int 0 len)))`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAlias {
    /// The alias name, e.g. `"Nat"`, `"Index"`.
    pub name: String,
    /// The type expression being aliased, stored as a [`TypeExpr`].
    pub expr: TypeExpr,
    pub line: usize,
    pub column: usize,
}

// ---------------------------------------------------------------------------
// LANG48 / TW05-A — records
// ---------------------------------------------------------------------------

/// A single field in a record or union-variant definition.
///
/// From the grammar `(name : type_annotation)`.
///
/// Examples:
/// - `(source-id : SourceId)` → `RecordField { name: "source-id", … }`
/// - `(start : (Index source-len))` → `RecordField { name: "start", type_annotation: Opaque(…) }`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordField {
    /// Field name.
    pub name: String,
    /// Field type annotation (LANG23 subset or opaque TW05-A expression).
    pub type_annotation: TypeAnnotation,
}

/// `(record Name (field : Type) …)` — a named product type (TW05-A / LANG48).
///
/// In TW05-A, a record declaration is **erased** into a set of IIR functions:
/// a constructor, positional field accessors, and a crude type predicate.
///
/// Example:
/// ```scheme
/// (record Span
///   (source-id : SourceId)
///   (start     : Nat)
///   (end       : Nat))
/// ```
/// Generates: `Span(source-id, start, end)`,
///             `span-source-id(r)`, `span-start(r)`, `span-end(r)`,
///             `span?(v)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordDef {
    /// Record type name, e.g. `"Span"`, `"Token"`.
    pub name: String,
    /// Ordered field declarations.
    pub fields: Vec<RecordField>,
    pub line: usize,
    pub column: usize,
}

// ---------------------------------------------------------------------------
// LANG48 / TW05-A — tagged unions
// ---------------------------------------------------------------------------

/// A single variant in a tagged union declaration.
///
/// From the grammar `(VariantName (field : T) …)`.
///
/// Example:
/// - `(IntLit (value : Int) (span : Span))` →
///   `UnionVariant { name: "IntLit", fields: [RecordField{ "value", … }, RecordField{ "span", … }] }`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionVariant {
    /// Variant constructor name, e.g. `"IntLit"`, `"NameRef"`.
    pub name: String,
    /// Fields of this variant (may be empty for unit variants).
    pub fields: Vec<RecordField>,
}

/// `(union Name (Variant …) …)` — a tagged sum type (TW05-A / LANG48).
///
/// In TW05-A, each variant is erased into integer-tagged constructors.
/// The zero-based index of the variant in the declaration order is its
/// runtime tag.
///
/// Example:
/// ```scheme
/// (union Expr
///   (IntLit  (value : Int) (span : Span))  ; tag 0
///   (NameRef (name : Symbol) (span : Span))) ; tag 1
/// ```
/// Generates: `IntLit(value, span)`, `IntLit?(v)`, `intlit-value(v)`, `intlit-span(v)`,
///             `NameRef(name, span)`, `NameRef?(v)`, `nameref-name(v)`, `nameref-span(v)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionDef {
    /// Union type name, e.g. `"Expr"`, `"TokenKind"`.
    pub name: String,
    /// Variants in declaration order; index = runtime integer tag.
    pub variants: Vec<UnionVariant>,
    pub line: usize,
    pub column: usize,
}

// ---------------------------------------------------------------------------
// LANG48 / TW05-A — match patterns
// ---------------------------------------------------------------------------

/// A pattern in a `(match …)` arm (TW05-A / LANG48).
///
/// The grammar rule `match_pat` has two alternatives:
/// - `LPAREN NAME { NAME } RPAREN` — variant pattern with field bindings.
/// - `NAME` — either a wildcard (`_`) or a bare name binding.
///
/// The extractor promotes `_` to [`Wildcard`] and all other bare names to
/// [`Binding`].  The compiler additionally inspects the pattern name against
/// the variant-tag table to decide whether a [`Variant`] arm applies.
///
/// [`Wildcard`]: MatchPat::Wildcard
/// [`Binding`]: MatchPat::Binding
/// [`Variant`]: MatchPat::Variant
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchPat {
    /// `(VariantName b1 b2 … bn)` — matches a union variant; `bindings`
    /// are bound to the variant's fields left-to-right.
    Variant {
        /// Constructor name (e.g. `"IntLit"`).  Resolved to a tag at
        /// compile time via the variant-tag table.
        name: String,
        /// Names to bind to the variant's fields in left-to-right order.
        bindings: Vec<String>,
    },
    /// `_` — wildcard; no binding is introduced, body has no extra names.
    Wildcard,
    /// `name` — bare name binding; the entire scrutinee is bound to `name`.
    Binding(String),
}

/// One arm of a `(match …)` expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    /// The pattern this arm matches.
    pub pat: MatchPat,
    /// Body expressions (one or more); the last one is the arm's value.
    pub body: Vec<Expr>,
}

/// `(match scrutinee arm+)` — pattern-matching expression (TW05-A / LANG48).
///
/// In TW05-A, a `match` is **erased** to a chain of `if`/`let` expressions
/// by the IR compiler:
///
/// 1. Evaluate `scrutinee` once into a fresh register.
/// 2. For each `Variant` arm: check `(= (car scrutinee) tag)`, bind fields,
///    evaluate body.
/// 3. For a `Binding` arm: bind the scrutinee to the name, evaluate body.
/// 4. For a `Wildcard` arm: evaluate the body directly.
/// 5. After all arms: fall through to `nil`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// The expression being scrutinised.
    pub scrutinee: Box<Expr>,
    /// Arms in order; first match wins.
    pub arms: Vec<MatchArm>,
    pub line: usize,
    pub column: usize,
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

/// A string literal: `"hello"`, `""`, `"say \"hi\""`.
///
/// The `value` field holds the **decoded** string — escape sequences
/// (`\"`, `\\`, `\n`, `\t`, `\r`) are already converted to their byte
/// values by the AST extractor.  The IIR compiler emits a `const`
/// instruction with `Operand::Str(value)` which the VM materialises
/// as a [`lispy_runtime::heap::LangString`] heap object.
///
/// Added in LANG51 to enable writing the self-hosted Twig compiler in
/// Twig itself — the compiler source needs string constants for keyword
/// names, error messages, and token strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrLit {
    pub value: String,
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

/// `(let* ((x e1) (y e2) ...) body+)` — LANG52 sequential bindings.
///
/// Each RHS is evaluated in a scope that includes all *prior* bindings.
/// This is Scheme `let*`, not plain `let`.
///
/// Example: `(let* ((a 1) (b (+ a 1))) b)` → 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetStar {
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
    /// A double-quoted string literal: `"hello"`.  Added in LANG51.
    StrLit(StrLit),
    VarRef(VarRef),
    If(If),
    Let(Let),
    /// `(let* ((x e1) (y (+ x 1)) …) body+)` — sequential bindings (LANG52).
    LetStar(LetStar),
    Begin(Begin),
    Lambda(Lambda),
    Apply(Apply),
    /// `(match scrutinee arm+)` — pattern matching (TW05-A / LANG48).
    Match(Match),
}

impl Expr {
    /// Return the source position `(line, column)` of this expression.
    pub fn pos(&self) -> (usize, usize) {
        match self {
            Expr::IntLit(n) => (n.line, n.column),
            Expr::BoolLit(b) => (b.line, b.column),
            Expr::NilLit(n) => (n.line, n.column),
            Expr::SymLit(s) => (s.line, s.column),
            Expr::StrLit(s) => (s.line, s.column),
            Expr::VarRef(v) => (v.line, v.column),
            Expr::If(i) => (i.line, i.column),
            Expr::Let(l) => (l.line, l.column),
            Expr::LetStar(l) => (l.line, l.column),
            Expr::Begin(b) => (b.line, b.column),
            Expr::Lambda(l) => (l.line, l.column),
            Expr::Apply(a) => (a.line, a.column),
            Expr::Match(m) => (m.line, m.column),
        }
    }
}

/// A top-level form — either a `(define ...)`, a typed declaration, or a
/// bare expression.
///
/// Bare top-level expressions accumulate into the synthesised `main`
/// function during compilation; the value of the *last* one becomes
/// the program's return value.
///
/// TW05-A / LANG48 adds three typed-syntax top-level forms:
/// - [`Form::TypeAlias`] — erased in TW05-A (compile-time only).
/// - [`Form::RecordDef`] — erased to constructor + accessors in TW05-A.
/// - [`Form::UnionDef`]  — erased to tagged constructors in TW05-A.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Form {
    Define(Define),
    Expr(Expr),
    /// `(type Name type_expr)` — compile-time type alias.  Erased in TW05-A.
    TypeAlias(TypeAlias),
    /// `(record Name (field : T) …)` — record product type declaration.
    RecordDef(RecordDef),
    /// `(union Name (Variant …) …)` — tagged sum type declaration.
    UnionDef(UnionDef),
}

/// A whole compilation unit — the ordered list of top-level forms.
///
/// An empty `Program` is valid; it compiles to a module whose `main`
/// returns `nil`.
///
/// `module_info` is `None` for programs without a `(module …)` declaration.
/// Such programs get an implicit "default" module at compile time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub forms: Vec<Form>,
    /// Optional module metadata from the leading `(module …)` declaration.
    /// `None` for programs that don't declare a module.
    pub module_info: Option<ModuleInfo>,
}
