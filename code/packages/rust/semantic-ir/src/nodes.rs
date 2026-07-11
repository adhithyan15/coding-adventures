//! Node definitions for the narrow-waist Semantic IR.
//!
//! See SIR10 for the design discussion.  The goal of this module is
//! to define exactly one Rust type per semantic concept the IR
//! supports, so that backends can `match` on node kind and emit
//! code without ever having to ask "what did the programmer mean
//! here?".
//!
//! ## Reading guide
//!
//! Nodes form a tree rooted at [`Module`].  Each level introduces
//! one or two concepts:
//!
//! - [`Module`] — top-level compilation unit; collects functions,
//!   globals, imports/exports, manifest, metadata.
//! - [`Function`] — a callable with typed params, optional return
//!   type, captures (for closures), and a body block.
//! - [`Block`] — a list of statements followed by a *value
//!   expression*.  Every block produces a value; this rule lets the
//!   backend always emit an expression-position result.
//! - [`Stmt`] — three kinds: parallel `let`, sequential `let*`, and
//!   bare expression statements.
//! - [`Expr`] — the open-ended expression grammar; one variant per
//!   semantic concept (atoms, references, control flow, calls,
//!   closure construction, intrinsic escape hatch).

use crate::effects::EffectSet;
use crate::manifest::FeatureManifest;
use crate::metadata::Metadata;
use crate::span::Span;
use crate::types::{IntSpec, SirType};

// ---------------------------------------------------------------------------
// Module-level structure
// ---------------------------------------------------------------------------

/// A compilation unit.
///
/// ```text
/// Module
///   ├── name        : String                  — module identifier
///   ├── manifest    : FeatureManifest         — features used
///   ├── imports     : Vec<Import>             — referenced modules
///   ├── exports     : Vec<ExportName>         — names this module exposes
///   ├── functions   : Vec<Function>           — function table
///   ├── globals     : Vec<Global>             — top-level value bindings
///   ├── metadata    : Metadata                — advisory info
///   └── span        : Span                    — source position
/// ```
///
/// Note: `Eq` is intentionally omitted because [`Expr::FloatLit`] holds
/// a raw `f64`, which only implements `PartialEq` (NaN ≠ NaN).  All
/// types that transitively contain `Expr` follow the same rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub name: String,
    pub manifest: FeatureManifest,
    pub imports: Vec<Import>,
    pub exports: Vec<ExportName>,
    pub functions: Vec<Function>,
    pub globals: Vec<Global>,
    pub metadata: Metadata,
    pub span: Span,
}

/// An imported module reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub module_path: String,
    pub names: Vec<ImportName>,
    pub span: Span,
}

/// A name imported from another module.  `local_name` may differ
/// from `source_name` to support renaming on import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportName {
    pub source_name: String,
    pub local_name: String,
}

/// A name this module exposes to other modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportName {
    pub name: String,
    pub span: Span,
}

/// A top-level value binding.
///
/// The actual initialization expression lives in the synthesised
/// `_init` function (referenced by `init_function`).  This separation
/// keeps `Global` a pure declaration; backends can still emit
/// native top-level `let`/`var` declarations by recognising the
/// `global_set` calls inside `_init` (SIR12 covers the convention).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Global {
    pub name: String,
    pub sir_type: Option<SirType>,
    pub init_function: String,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Function
// ---------------------------------------------------------------------------

/// A callable with parameters, an optional return type, optional
/// captures (for closure bodies), an effect annotation, and a
/// `Block` body.
///
/// A function with non-empty `captures` is a closure body — it is
/// referenced by `MakeClosure { fn_name, ... }` rather than called
/// directly.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<SirType>,
    pub captures: Vec<Capture>,
    pub body: Block,
    pub effects: EffectSet,
    pub metadata: Metadata,
    pub span: Span,
}

impl Function {
    /// Number of *required* leading parameters — the call-arity floor.
    ///
    /// SIR adopts a Ruby/JS default-parameter model (SIR10 "Default
    /// parameters and call arity"): a default may be omitted by the
    /// caller, but only from the **trailing** edge of the positional
    /// list.  So the required count is the length of the longest leading
    /// run of plain positional params that have **no** default.  The
    /// first defaulted param (or the first `*rest` / `**opts` variadic,
    /// or the synthetic trailing block param) ends that run.
    ///
    /// ```text
    ///   def f(a, b, c = 1, d = 2)   →  required_param_count() == 2
    ///   def g(a, b)                 →  required_param_count() == 2
    ///   def h(a = 1, b = 2)         →  required_param_count() == 0
    /// ```
    ///
    /// Worked reasoning for the first example: `a` and `b` have no
    /// default, so the leading required run is `[a, b]` (length 2); `c`
    /// has a default and terminates the run.  A caller must therefore
    /// pass at least 2 positional arguments.
    ///
    /// Note: only an unbroken *leading* run counts.  A required param
    /// that follows a defaulted one (a "hole", e.g. `def f(a = 1, b)`)
    /// does **not** extend the required count — the validator forbids a
    /// caller from omitting `a` while passing `b`, so `b` is not freely
    /// omissible and `a` is not freely required.  Such a definition is
    /// legal (the callee always receives a `b`), but its arity floor is
    /// the leading run length (here `0`); the trailing-default rule in
    /// [`Self::missing_defaults`] handles the rest.
    pub fn required_param_count(&self) -> usize {
        self.params
            .iter()
            .take_while(|p| p.kind == ParamKind::Required && p.default.is_none())
            .count()
    }

    /// The trailing parameters a caller has **omitted** when it supplies
    /// `n_args` positional arguments — i.e. the params at positions
    /// `n_args .. params.len()`.
    ///
    /// Backends use this to know which trailing params they must fill
    /// with their default expressions at the call site (the per-backend
    /// default-param emission, a follow-up PR).  For a call the validator
    /// has accepted, every returned param is guaranteed to carry a
    /// `default` (that is precisely the arity rule), so a backend can
    /// emit each one's default unconditionally.
    ///
    /// ```text
    ///   f = def f(a, b, c = 1, d = 2)
    ///   f.missing_defaults(4)  →  []            // all args passed
    ///   f.missing_defaults(3)  →  [d]           // d omitted
    ///   f.missing_defaults(2)  →  [c, d]        // c, d omitted
    /// ```
    ///
    /// If `n_args >= params.len()` the slice is empty.  The method never
    /// panics on over-supply: it clamps to the param count.
    pub fn missing_defaults(&self, n_args: usize) -> &[Param] {
        let n = n_args.min(self.params.len());
        &self.params[n..]
    }

    /// The callee's *keyword* parameters — those with `kind == Keyword`
    /// (KW1).  Unlike positionals, a keyword param is matched by **name**
    /// at the call site, so the validator's call-side resolution consults
    /// this list (rather than a positional index) to decide whether a
    /// `KeywordArg` is accepted and which required keywords were supplied.
    ///
    /// ```text
    ///   def f(a, x:, y: 1, **rest)   →  keyword_params() == [x, y]
    /// ```
    ///
    /// (`a` is positional, `**rest` is `KwRest` — neither is a `Keyword`.)
    pub fn keyword_params(&self) -> Vec<&Param> {
        self.params
            .iter()
            .filter(|p| p.kind == ParamKind::Keyword)
            .collect()
    }

    /// The `Keyword` params whose name is **not** in `supplied` — i.e. the
    /// keyword parameters a caller left out (KW1).
    ///
    /// Backends use this the way [`Self::missing_defaults`] is used for
    /// trailing positionals: for a call the validator has **accepted**,
    /// every returned param is guaranteed to carry a `default`.  That is
    /// precisely the required-keyword rule: a *required* keyword (kind
    /// `Keyword`, `default == None`) that the caller omits is a validation
    /// error, so it can never survive into this list.  A backend may
    /// therefore emit each returned param's default unconditionally.
    ///
    /// ```text
    ///   def f(x:, y: 1, z: 2)
    ///   f.missing_keywords(&["x", "y"])  →  [z]        // z omitted, has default
    ///   f.missing_keywords(&["x"])       →  [y, z]     // y, z omitted, both have defaults
    /// ```
    pub fn missing_keywords(&self, supplied: &[&str]) -> Vec<&Param> {
        self.params
            .iter()
            .filter(|p| p.kind == ParamKind::Keyword)
            .filter(|p| !supplied.contains(&p.name.as_str()))
            .collect()
    }
}

/// How a parameter binds its arguments (M3 — see
/// `code/specs/sir-variadic-params.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParamKind {
    /// An ordinary positional parameter (`x`). The default for every
    /// parameter that is not a splat.
    #[default]
    Required,
    /// A rest parameter (`*rest`) — collects trailing positional arguments
    /// into a sequence.
    Rest,
    /// A named keyword parameter (`x:` / `x: 1` in Ruby, `x` / `x=1` after
    /// `*` in Python) — bound by *name* at the call site, never by
    /// position (KW1 — see `code/specs/sir-keyword-params.md`).
    ///
    /// Required-vs-optional rides on the **existing** `Param.default`
    /// field, exactly as a positional optional does — there is no separate
    /// "is-required" flag:
    ///
    /// ```text
    ///   Param { kind: Keyword, default: None    }  →  REQUIRED keyword: `def f(x:)`
    ///   Param { kind: Keyword, default: Some(e) }  →  OPTIONAL keyword: `def f(x: 1)`
    /// ```
    ///
    /// Why reuse `default` rather than add a flag?  Because the two axes
    /// (how the argument is *matched* — position vs. name — and whether it
    /// may be *omitted*) are orthogonal.  `ParamKind` already answers the
    /// first for the positional/`Rest`/`KwRest` cases; `default` already
    /// answers the second for positionals.  A `Keyword` param simply
    /// combines the name-matched axis with the same omissibility rule, so
    /// no new field (and, because `ParamKind` is `Copy` with
    /// `#[default] = Required`, no existing `Param { .. }` construction)
    /// changes.
    Keyword,
    /// A keyword-rest parameter (`**opts`) — collects trailing keyword
    /// arguments into a map.
    KwRest,
}

/// A function parameter.
///
/// The optional `default` carries a parameter's default-value
/// expression, e.g. the `1` in Ruby `def f(a = 1)` (and the Python /
/// JS equivalents).  `None` means an ordinary parameter with no
/// default; `Some(expr)` means the parameter binds to `expr` when the
/// caller omits the corresponding argument.
///
/// The default is boxed (`Option<Box<Expr>>`) to break the otherwise
/// infinitely-sized recursive type `Param → Expr → Function → Param`:
/// a default expression may contain a closure whose own parameters may
/// themselves have defaults.  `Box` puts the `Expr` behind a pointer so
/// the struct has a fixed size.
///
/// Note: because `Expr` contains an `f64` (`FloatLit`) it cannot derive
/// `Eq`, so neither can `Param` once it holds an `Expr`.  `Param`
/// therefore derives only `PartialEq` (structural equality is still
/// available; total `Eq` is not — consistent with `Expr`/`Block`).
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub sir_type: Option<SirType>,
    /// The binding kind (`Required` by default; `Rest`/`KwRest` for the
    /// `*rest`/`**opts` variadic forms — M3).
    pub kind: ParamKind,
    /// The default-value expression, if any.  `None` for an ordinary
    /// parameter; `Some(expr)` for `name = expr` (Ruby/Python/JS).
    /// Boxed to keep `Param` a fixed size despite the `Param → Expr →
    /// Function → Param` cycle.
    pub default: Option<Box<Expr>>,
    pub span: Span,
}

/// A capture binding.  Note: no `span` field — captures originate
/// from the call site (`MakeClosure`'s `CaptureValue`), not from a
/// source position inside the function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    pub name: String,
    pub sir_type: Option<SirType>,
}

// ---------------------------------------------------------------------------
// Blocks, statements, expressions
// ---------------------------------------------------------------------------

/// A list of statements followed by a value expression.
///
/// Every `Block` produces a value (`value`).  This means a SIR
/// program is fully expression-oriented at the block boundary: the
/// `body` of an `If`, a `Function`, or a let binding always yields
/// a typed value.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub value: Expr,
    pub span: Span,
}

/// One `rescue` clause of a [`Stmt::TryCatch`] (SIR17, Ruby Phase 16a).
///
/// A `begin … rescue … end` may carry several `rescue` clauses, each
/// matching a (possibly empty) set of exception classes and optionally
/// binding the caught exception to a local.
#[derive(Debug, Clone, PartialEq)]
pub struct RescueClause {
    /// Exception class names this clause matches (`rescue Foo, Bar`).
    /// **Empty** means a bare `rescue` (catch-all).  These are advisory
    /// names only: SIR v0 has no exception-class symbol table, so the
    /// validator does not resolve them (mirroring `ClassDef.superclass`).
    pub exception_types: Vec<String>,
    /// Optional binding for the caught exception (`rescue … => e`).
    /// When `Some`, the name is in scope as a `Scope::Local` within
    /// `body` only.
    pub binding: Option<String>,
    /// The clause body, a bare statement list (no trailing value slot,
    /// like `ClassDef.body`).
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// Statement kinds.
///
/// SIR v0 had only `LetBinding`, `LetStarBinding`, and `ExprStmt`.
/// SIR16 (Python/JS interop) extends this with mutation (`Assign`),
/// loops (`While`, `ForRange`, `ForEach`), and indexed assignment on
/// sequences and maps (`SeqSet`, `MapSet`).
// Variants differ in size because some statement kinds carry several boxed
// `Expr`s while others carry one. This is a core AST node matched exhaustively
// throughout the crate; boxing a variant to equalize sizes would churn every
// construction and pattern for no meaningful gain.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Parallel-let semantics.  Multiple consecutive `LetBinding`
    /// statements have their RHS evaluated in the scope outside the
    /// group; they may run in any order.
    LetBinding {
        name: String,
        sir_type: Option<SirType>,
        value: Expr,
        span: Span,
    },
    /// Sequential-let* semantics.  Each RHS sees prior `LetStarBinding`s
    /// in the same group.  Order is observable.
    LetStarBinding {
        name: String,
        sir_type: Option<SirType>,
        value: Expr,
        span: Span,
    },
    /// A bare expression evaluated for its side effects.
    ExprStmt { expr: Expr, span: Span },

    // ── SIR16: mutation ────────────────────────────────────────────
    /// Re-bind an already-declared name.  Frontends use `LetBinding`
    /// for first-occurrence binding and `Assign` for subsequent
    /// re-assignments.  `scope` is the same enum used by `VarRef`.
    Assign {
        name: String,
        scope: Scope,
        value: Expr,
        span: Span,
    },

    // ── SIR16: loops ────────────────────────────────────────────────
    /// `while <cond>: <body>` — body re-executes while `cond` is truthy.
    While { cond: Expr, body: Block, span: Span },
    /// `for var in range(start, stop, step): body`.  Half-open: `stop`
    /// is exclusive.  `step` is typically `IntLit(1)`.
    ForRange {
        var: String,
        start: Expr,
        stop: Expr,
        step: Expr,
        body: Block,
        span: Span,
    },
    /// `for var in iter: body` — iterates a Seq.
    ForEach {
        var: String,
        iter: Expr,
        body: Block,
        span: Span,
    },

    // ── SIR16: indexed assignment ──────────────────────────────────
    /// `seq[index] = value` — mutate a sequence element.
    SeqSet {
        seq: Expr,
        index: Expr,
        value: Expr,
        span: Span,
    },
    /// `map[key] = value` — set a map entry.
    MapSet {
        map: Expr,
        key: Expr,
        value: Expr,
        span: Span,
    },

    // ── SIR17: class declarations ──────────────────────────────────
    /// `class Name; body; end` — a class declaration.
    ///
    /// SIR v0 represents a class as a named declaration whose body is
    /// itself a list of statements.  Phase 14a (Ruby frontend) lands
    /// the *empty-body* case: `class Foo; end` lowers to
    /// `ClassDef { name: "Foo", body: vec![], span }`.  Method bodies
    /// are *not* nested here yet — they continue to be hoisted to
    /// top-level `Function`s by the Ruby lowerer's existing pass
    /// (see ruby-to-semantic-ir Phase 6f).  Later Ruby phases (14b)
    /// will populate `body` directly so methods nest under their
    /// owning class.
    ///
    /// Why a `Vec<Stmt>` body rather than `Block`?  A class body
    /// produces no value — it is a declaration, not an expression —
    /// so the per-Block trailing `value` field doesn't apply.
    /// Backends emit each statement in source order.
    ///
    /// `superclass` (SIR17, Ruby Phase 14c) carries the parent class
    /// name for `class Foo < Bar` — `Some("Bar")` — and is `None` for
    /// a base class (`class Foo`).  It is an advisory name only: SIR v0
    /// has no class symbol table, so the validator does not resolve it
    /// (mirroring how the class's own `name` is not bound as a local).
    ClassDef {
        name: String,
        superclass: Option<String>,
        body: Vec<Stmt>,
        span: Span,
    },

    /// `module Name; body; end` — a module (namespace / mixin)
    /// declaration.  Structurally a `ClassDef` without inheritance:
    /// a named declaration whose `body` is a list of statements.
    ///
    /// Introduced by the Ruby frontend's Phase 14d.  Like `ClassDef`,
    /// method `def`s inside the body are hoisted to top-level
    /// `Function`s by the lowerer (SIR v0 has no method-as-statement
    /// node); the `body` carries the module's *non-def* statements in
    /// source order.  A module has no superclass, so there is no
    /// `superclass` field.
    ModuleDef {
        name: String,
        body: Vec<Stmt>,
        span: Span,
    },

    /// `class << receiver; body; end` — a singleton-class (metaclass)
    /// declaration.  Introduced by the Ruby frontend's Phase 14e.
    ///
    /// `target` is the receiver whose singleton class is opened — the
    /// dominant idiom is `class << self` (`target = "self"`), but a
    /// bare object name is also accepted (`class << obj`).  Like
    /// `ClassDef`/`ModuleDef`, method `def`s inside the body are
    /// hoisted to top-level `Function`s by the lowerer; `body` carries
    /// the non-`def` statements.  Triggers `Feature::Classes` (a
    /// singleton class is a class-opening construct, not a new
    /// feature).
    SingletonClassDef {
        target: String,
        body: Vec<Stmt>,
        span: Span,
    },

    // ── SIR17: exception handling ──────────────────────────────────
    /// `begin; body; rescue …; ensure …; end` — structured exception
    /// handling.  Introduced by the Ruby frontend's Phase 16a, which
    /// replaces the earlier `__rescue_marker__` / `__ensure_marker__`
    /// inline `BuiltinCall` placeholders with this first-class node.
    ///
    /// - `body` runs first (a bare statement list, like `ClassDef.body`).
    /// - `rescues` are tried in order if `body` raises; each
    ///   [`RescueClause`] matches a set of exception classes and may
    ///   bind the exception.
    /// - `ensure_body`, when `Some`, runs unconditionally afterwards.
    ///
    /// Gated by `Feature::Exceptions`; backends that don't accept it
    /// reject the module at the capability check before emit.
    TryCatch {
        body: Vec<Stmt>,
        rescues: Vec<RescueClause>,
        ensure_body: Option<Vec<Stmt>>,
        span: Span,
    },

    // ── SIR22: array/matrix indexed assignment ─────────────────────
    /// `target[indices...] = value` — mutate an element, row, column,
    /// or sub-range of an `NDArray` (MATLAB/Octave `A(2, :) = v`).
    ///
    /// Introduced by [SIR22](../../../../specs/SIR22-array-matrix-semantic-ir.md)
    /// as the mutation-shaped counterpart of [`Expr::IndexGet`] — indexed
    /// *reads* are a value-producing `Expr`, but an indexed *write* has a
    /// mutation effect indistinguishable in shape from [`Stmt::Assign`], so
    /// it lives in `Stmt` rather than `Expr`, exactly as SIR16's `Assign`
    /// does relative to a plain `VarRef`.  This is the one exception noted
    /// in the SIR22 spec's "Effects" section: every other new SIR22 node is
    /// `Pure`.
    ///
    /// `indices` mirrors [`Expr::IndexGet`]'s `indices: Vec<IndexArg>` —
    /// the frontend has already resolved 1-based/`end`-relative MATLAB
    /// indexing down to concrete 0-based [`IndexArg`]s before emitting this
    /// node; the IR never sees `end`.
    IndexSet {
        target: Box<Expr>,
        indices: Vec<IndexArg>,
        value: Box<Expr>,
        span: Span,
    },
}

impl Stmt {
    pub fn span(&self) -> &Span {
        match self {
            Stmt::LetBinding { span, .. } => span,
            Stmt::LetStarBinding { span, .. } => span,
            Stmt::ExprStmt { span, .. } => span,
            Stmt::Assign { span, .. } => span,
            Stmt::While { span, .. } => span,
            Stmt::ForRange { span, .. } => span,
            Stmt::ForEach { span, .. } => span,
            Stmt::SeqSet { span, .. } => span,
            Stmt::MapSet { span, .. } => span,
            Stmt::ClassDef { span, .. } => span,
            Stmt::ModuleDef { span, .. } => span,
            Stmt::SingletonClassDef { span, .. } => span,
            Stmt::TryCatch { span, .. } => span,
            Stmt::IndexSet { span, .. } => span,
        }
    }
}

/// Scope tag attached to every variable reference.  Frontend commits
/// to a scope at lowering time; the backend never re-resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Bound by `let` / `let*` in the current scope chain.
    Local,
    /// Function parameter.
    Param,
    /// Captured from an enclosing scope (closure body only).
    Capture,
    /// Top-level value binding in this module.
    Global,
    /// Language built-in (`+`, `cons`, etc.).
    Builtin,
    /// Object instance variable (Ruby `@x`).  Unlike `Local`, an
    /// instance variable needs **no prior declaration**: reading an
    /// unset `@x` yields nil in Ruby, so the validator performs no
    /// scope-existence check for this kind.  The leading `@` sigil is
    /// preserved in the `VarRef` / `Assign` name.  Introduced by the
    /// Ruby frontend's Phase 15a; gated by `Feature::InstanceVars`.
    Instance,
    /// Class variable (Ruby `@@x`).  Like `Instance`, it needs **no
    /// prior declaration** (the validator performs no scope-existence
    /// check) — but it is shared across the class hierarchy rather than
    /// per-object.  The leading `@@` sigil is preserved in the name.
    /// Introduced by the Ruby frontend's Phase 15b; gated by
    /// `Feature::ClassVars`.
    ClassVar,
    /// Constant (Ruby `FOO`, `MyClass` — any name whose first letter is
    /// uppercase).  Like `Instance`/`ClassVar`, it needs **no prior
    /// declaration** in the SIR sense (the validator performs no
    /// scope-existence check): a constant is resolved against the
    /// enclosing lexical/constant scope at runtime, not against a `let`
    /// binding.  The name is preserved verbatim.  Introduced by the
    /// Ruby frontend's Phase 15c; gated by `Feature::Constants`.
    Const,
}

impl Scope {
    /// Kebab-case name used by the SIR text format.
    pub fn name(&self) -> &'static str {
        match self {
            Scope::Local => "local",
            Scope::Param => "param",
            Scope::Capture => "capture",
            Scope::Global => "global",
            Scope::Builtin => "builtin",
            Scope::Instance => "instance",
            Scope::ClassVar => "class-var",
            Scope::Const => "const",
        }
    }

    /// Inverse of [`name`].
    pub fn from_name(s: &str) -> Option<Scope> {
        Some(match s {
            "local" => Scope::Local,
            "param" => Scope::Param,
            "capture" => Scope::Capture,
            "global" => Scope::Global,
            "builtin" => Scope::Builtin,
            "instance" => Scope::Instance,
            "class-var" => Scope::ClassVar,
            "const" => Scope::Const,
            _ => return None,
        })
    }
}

/// The expression grammar.  Every variant is a distinct semantic
/// concept.  Backends `match` on the variant and emit code; the IR
/// guarantees this match is exhaustive.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // ── atomic literals ────────────────────────────────────────────
    IntLit {
        value: i64,
        span: Span,
    },
    BoolLit {
        value: bool,
        span: Span,
    },
    NilLit {
        span: Span,
    },
    SymLit {
        name: String,
        span: Span,
    },
    StrLit {
        value: String,
        span: Span,
    },

    // ── reference ───────────────────────────────────────────────────
    VarRef {
        name: String,
        scope: Scope,
        span: Span,
    },

    // ── control flow / sequencing ──────────────────────────────────
    If {
        cond: Box<Expr>,
        then_branch: Box<Block>,
        else_branch: Box<Block>,
        span: Span,
    },
    Block(Box<Block>),

    // ── calls (three distinct kinds) ───────────────────────────────
    /// A call to a top-level function known by name.
    DirectCall {
        fn_name: String,
        args: Vec<Expr>,
        effects: EffectSet,
        span: Span,
    },
    /// A call through a value (closure handle) at runtime.
    IndirectCall {
        target: Box<Expr>,
        args: Vec<Expr>,
        effects: EffectSet,
        span: Span,
    },
    /// A call to a language builtin.
    BuiltinCall {
        name: String,
        args: Vec<Expr>,
        effects: EffectSet,
        span: Span,
    },

    // ── closure construction ───────────────────────────────────────
    MakeClosure {
        fn_name: String,
        captures: Vec<CaptureValue>,
        span: Span,
    },

    // ── escape hatch ───────────────────────────────────────────────
    Intrinsic {
        targets: Vec<String>,
        name: String,
        args: Vec<Expr>,
        return_type: SirType,
        effects: EffectSet,
        span: Span,
    },

    // ── SIR16: floats ──────────────────────────────────────────────
    /// 64-bit floating-point literal.
    FloatLit {
        value: f64,
        span: Span,
    },

    // ── SIR16: sequences ───────────────────────────────────────────
    /// `[item0, item1, ...]` literal.
    SeqLit {
        items: Vec<Expr>,
        span: Span,
    },
    /// `seq[index]` — 0-indexed.  Out-of-bounds behaviour is target-
    /// language-defined.
    SeqIndex {
        seq: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// `len(seq)` — convenience operator distinct from `BuiltinCall("len", ...)`
    /// so backends can emit native length access (`xs.length` /
    /// `len(xs)` / `xs.len()`).  Frontends should prefer this node
    /// over the builtin form.
    SeqLen {
        seq: Box<Expr>,
        span: Span,
    },

    // ── SIR16: maps ────────────────────────────────────────────────
    /// `{key: value, ...}` literal.
    MapLit {
        entries: Vec<MapEntry>,
        span: Span,
    },
    /// `map[key]` — missing-key behaviour is target-language-defined.
    MapGet {
        map: Box<Expr>,
        key: Box<Expr>,
        span: Span,
    },

    // ── SIR16: short-circuit logical ───────────────────────────────
    /// `lhs && rhs` / `lhs and rhs` — short-circuits.  Distinct from
    /// `BuiltinCall("and", ...)` because the latter would eagerly
    /// evaluate both arguments before invoking the helper.
    LogicalAnd {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// `lhs || rhs` / `lhs or rhs` — short-circuits.
    LogicalOr {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },

    // ── SIR18: string interpolation ────────────────────────────────
    /// String concatenation of two or more `parts`, evaluated left to
    /// right and joined into a single string.  This is the first-class
    /// replacement for the v0 `BuiltinCall("string_concat", parts)`
    /// marker that Ruby's `"a#{x}b"` interpolation used to lower to —
    /// the same relationship `SeqLen` has to `BuiltinCall("len", ...)`
    /// or `TryCatch` has to the old `__rescue_marker__` builtin.
    ///
    /// Giving concatenation a dedicated node lets a backend emit native
    /// string building (`format!` / template literals / f-strings)
    /// instead of routing through a runtime helper, and lets the
    /// validator track interpolation usage via
    /// `Feature::StringInterpolation` distinctly from a plain `StrLit`.
    ///
    /// Invariant: `parts.len() >= 2`.  A zero- or one-part concat is
    /// degenerate — frontends emit a bare `StrLit` (empty string) or the
    /// single part directly rather than wrapping it.
    StrConcat {
        parts: Vec<Expr>,
        span: Span,
    },

    // ── KW1: keyword arguments ─────────────────────────────────────
    /// A keyword argument at a call site: `name: value` (Ruby `f(a: 1)`,
    /// Python `f(a=1)`).  Appears ONLY inside a call's `args` vec, and only
    /// AFTER all positional arguments.  The validator enforces both rules.
    ///
    /// Design note — why a `KeywordArg` *inside* `args` rather than a
    /// separate `kwargs` field on each call node?  Three call nodes
    /// (`DirectCall`, `IndirectCall`, `MakeClosure`) all take arguments;
    /// threading a parallel `kwargs: Vec<(String, Expr)>` through every one
    /// (plus the walker, printer, and every backend `match`) would triple
    /// the surface area.  Instead a keyword argument is just another
    /// `Expr` variant that may sit in the *existing* `args` vec, so
    /// `f(1, a: 2)` lowers to `args: [IntLit(1), KeywordArg { name: "a",
    /// value: IntLit(2) }]`.  Positional args stay bare; the validator
    /// guarantees every `KeywordArg` trails all positionals, so a backend
    /// can split `args` at the first `KeywordArg` without ambiguity.
    ///
    /// `value` is boxed for the same fixed-size reason as the other
    /// single-child expression variants (`SeqLen`, `MapGet`, …).
    KeywordArg {
        name: String,
        value: Box<Expr>,
        span: Span,
    },

    // ── SIR22: array/matrix literals, ranges, and operators ────────
    //
    // Every node kind below is deliberately mapped 1:1 onto an existing
    // `array_runtime::execute()` op shape (see the SIR22 spec's
    // "Motivation") so that a MATLAB/Octave frontend's job is picking the
    // right node, not inventing new semantics.  All are `Pure` (see
    // `effects.rs`'s SIR22 doc note) — array construction, indexing, and
    // arithmetic have no observable side effects distinct from the value
    // they compute.  `IndexSet` (the one mutation-shaped exception) is a
    // `Stmt`, not an `Expr` — see its doc comment above.
    /// `[1 2; 3 4]` — a matrix/array literal.  `rows` is row-major *in
    /// the literal syntax*; reconciling that with the column-major
    /// storage convention (`Feature::ArrayColumnMajor`, see the SIR22
    /// spec's "Storage convention") is the frontend's job, not the IR's.
    /// A 1-row literal (`rows.len() == 1`) is a row vector; a frontend
    /// wanting a column vector emits one single-element row per element.
    ArrayLit {
        rows: Vec<Vec<Expr>>,
        span: Span,
    },

    /// `start:stop` / `start:step:stop` — a numeric range (MATLAB `1:5`,
    /// `0:2:10`).  `step` of `None` means step = 1, matching the MATLAB
    /// two-argument colon form.  Half-open vs. closed is left to
    /// `array_runtime::execute(Range, …)`'s own semantics — the IR node
    /// only carries the three operand slots the runtime op already takes.
    Range {
        start: Box<Expr>,
        step: Option<Box<Expr>>,
        stop: Box<Expr>,
        span: Span,
    },

    /// Matrix multiplication (MATLAB `*` on two matrices) — distinct
    /// from [`Expr::ElementwiseOp`]`(Mul, …)` (MATLAB `.*`), which
    /// multiplies same-shape arrays element-by-element.  Shape
    /// compatibility is an `array_runtime` runtime concern, not a SIR
    /// validation rule (SIR10 "types carry, don't verify").
    MatMul {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },

    /// An elementwise (broadcast) binary arithmetic op — MATLAB's
    /// dotted operators `.+` `.-` `.*` `./` `.^` (plain `+`/`-` are
    /// already elementwise in MATLAB, since they don't have a
    /// non-elementwise reading the way `*` does).  `op` selects which
    /// of the five [`ElementwiseOpKind`]s applies.
    ElementwiseOp {
        op: ElementwiseOpKind,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },

    /// Matrix transpose.  `conjugate: true` is MATLAB `'` (conjugate /
    /// Hermitian transpose); `conjugate: false` is `.'` (plain
    /// transpose, no conjugation).  Both map onto the same
    /// `array_runtime::execute(Transpose, …)` op; the frontend picks the
    /// flag at lowering time (the `'`-transpose-vs-string lexer decision
    /// is orthogonal to and unaffected by this node — see the SIR22
    /// spec).
    Transpose {
        target: Box<Expr>,
        conjugate: bool,
        span: Span,
    },

    /// `target(indices...)` / `target[indices...]` — an indexed *read*
    /// (MATLAB `A(2, :)`, `A(1:3)`).  Each entry of `indices` is an
    /// [`IndexArg`]; the frontend has already resolved 1-based and
    /// `end`-relative MATLAB subscripts down to concrete 0-based
    /// `IndexArg`s (the IR never sees `end` — see the SIR22 spec's note
    /// on `end`-relative indices).  The mutating counterpart is
    /// [`Stmt::IndexSet`], not a variant here — see that type's doc
    /// comment for why indexed *write* is a statement while indexed
    /// *read* is this expression.
    IndexGet {
        target: Box<Expr>,
        indices: Vec<IndexArg>,
        span: Span,
    },

    // ── SIR26 (integer conversions) ──────────────────────────────────
    /// Convert an integer `value` to the target integer type `to` by
    /// two's-complement reinterpretation: reduce modulo `2^width` (mask to
    /// the low `width` bits), then sign-extend when `to.signed` and the top
    /// bit is set.  A target width of `Arbitrary` is the identity (a widen
    /// into the unbounded integer — no bits lost).
    ///
    /// This is exactly a C integer cast / implicit conversion under
    /// two's-complement (`-fwrapv`): `(uint8_t)300 == 44`,
    /// `(int32_t)4_000_000_000 == −294_967_296`.  A frontend inserts a
    /// `Convert` after each width-bounded operation and at each cast /
    /// assignment; arithmetic stays exact, so the width enforcement here
    /// reproduces the source's overflow behaviour at every step.  See
    /// [SIR26](../../../specs/SIR26-integer-conversions.md).  Gated by
    /// [`Feature::Conversions`](crate::Feature::Conversions).
    Convert {
        value: Box<Expr>,
        to: IntSpec,
        span: Span,
    },

    // ── SIR23: symbolic expression + pattern/rewrite nodes ─────────
    //
    // Every node kind below is mapped 1:1 onto `symbolic_ir::IRNode`'s
    // existing five-variant shape (`Symbol`/`Integer`/`Rational`/`Float`/
    // `Str`/`Apply`) — see the SIR23 spec's "Motivation" and "New `Expr`
    // variants" sections.  `IntLit`/`FloatLit`/`StrLit` above already
    // cover `IRNode::Integer`/`Float`/`Str`; the seven variants below
    // cover `Symbol`/`Rational`/`Apply` plus the pattern-matching and
    // rewrite-rule vocabulary a Wolfram-family CAS frontend needs.  All
    // are `Pure` (see `effects.rs`'s SIR23 doc note) — building,
    // matching, and substituting a symbolic term has no observable side
    // effect distinct from the value it computes; SIR23 adds no new
    // `Stmt` variant at all (unlike SIR22's `IndexSet`).
    /// A bare symbolic-expression symbol — Wolfram `x`, `Plus`, `f` used
    /// as *data* rather than evaluated as a variable reference.
    /// Distinct from [`Expr::VarRef`] (a host-language variable lookup)
    /// and [`Expr::SymLit`] (a Ruby-style interned `:symbol` literal) —
    /// `SymSymbol` is a leaf of a *symbolic-expression tree*, mirroring
    /// `symbolic_ir::IRNode::Symbol`.
    SymSymbol {
        name: String,
        span: Span,
    },

    /// An exact rational scalar in **reduced form** (numerator and
    /// denominator share no common factor; denominator positive) —
    /// Wolfram `1/3`, `Rational[1, 3]`.  The frontend normalizes exactly
    /// as `symbolic_ir::IRNode::rational` does; the IR itself does not
    /// reduce or validate the fraction (SIR10 "types carry, don't
    /// verify").  Mirrors `symbolic_ir::IRNode::Rational`.
    SymRational {
        numer: i64,
        denom: i64,
        span: Span,
    },

    /// `head[args…]` / `head(args…)` as **data** — the same expression
    /// may appear as a value, a pattern target, or a rewrite-rule
    /// left-hand side (the SIR23 spec's "fidelity decision": patterns
    /// and rules are first-class data, not a frontend-side
    /// evaluate-then-lower shortcut).  `head` is a full `Expr`, not a
    /// bare name, because a *computed* head is legal Wolfram (`f[x][y]`
    /// applies the result of `f[x]` to `y`) — usually it is a
    /// `SymSymbol`, but the IR does not narrow the type.  Mirrors
    /// `symbolic_ir::IRNode::Apply`.
    SymApply {
        head: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },

    /// A pattern blank — Wolfram `_` (`head: None`) or `_h` (`head:
    /// Some(SymSymbol("h"))`, a head-constrained blank that matches only
    /// a subtree whose own "head" — per Wolfram's `Head[]` convention —
    /// is structurally `h`).  Only meaningful inside a [`SymRule`]'s
    /// `lhs` (directly, or nested inside a [`SymPatternNamed`]); the
    /// validator does not itself enforce that placement restriction
    /// (mirrors how [`Expr::KeywordArg`] restricts its own placement via
    /// a runtime flag rather than a type-level rule) — a "wild" blank
    /// appearing outside a pattern position is a frontend bug, not a
    /// distinct IR shape.
    ///
    /// [`SymRule`]: Expr::SymRule
    /// [`SymPatternNamed`]: Expr::SymPatternNamed
    SymPatternBlank {
        head: Option<Box<Expr>>,
        span: Span,
    },

    /// A **named** pattern variable — Wolfram `x_` (desugars to
    /// `SymPatternNamed { name: "x", pattern: SymPatternBlank { head:
    /// None } }`) or `x_h` (`pattern: SymPatternBlank { head: Some(h) }`).
    /// Binds `name` to whatever subtree `pattern` matches, for the rest
    /// of that match attempt — the SIR23 spec's matcher contract: a
    /// repeated occurrence of the same `name` elsewhere in a rule's
    /// `lhs` requires structural equality with the first binding, not
    /// just any match.
    SymPatternNamed {
        name: String,
        pattern: Box<Expr>,
        span: Span,
    },

    /// A rewrite rule — Wolfram `lhs -> rhs` (`delayed: false`, `Rule`:
    /// the rhs is built once, at rule-construction time) or `lhs :> rhs`
    /// (`delayed: true`, `RuleDelayed`: the rhs is re-evaluated fresh
    /// per match).  Only the flag distinguishes the two; both share the
    /// same `lhs`/`rhs` shape.
    SymRule {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        delayed: bool,
        span: Span,
    },

    /// Apply a set of rewrite rules to `expr` — Wolfram `expr /. rules`
    /// (`repeated: false`, `ReplaceAll`: one top-down, left-to-right
    /// pass) or `expr //. rules` (`repeated: true`, `ReplaceRepeated`:
    /// reruns to a fixed point).  `rules` is typically a `Vec` of
    /// [`SymRule`](Expr::SymRule)s, though the spec allows an element
    /// that is itself a `SymApply` evaluating to a list of rules at
    /// runtime (a backend concern, not an IR shape).  See the SIR23
    /// spec's "Matcher semantics" for the full binding contract —
    /// notably that every backend implementing `repeated: true` **must**
    /// enforce an iteration cap: an unbounded `//.` is a guaranteed
    /// non-terminating program for some inputs, matching the DoS-cap
    /// convention every other unbounded SIR construct in this repo
    /// already follows.
    SymReplaceAll {
        expr: Box<Expr>,
        rules: Vec<Expr>,
        repeated: bool,
        span: Span,
    },
}

/// The five elementwise (broadcast) binary arithmetic operators
/// (SIR22) — MATLAB's dotted operators `.+ .- .* ./ .^`.  Carried by
/// [`Expr::ElementwiseOp`] rather than five separate `Expr` variants
/// so a backend's `match` has one arm to open and a `match op` inside
/// it, mirroring how `Scope`/`ParamKind` are small closed enums rather
/// than a variant explosion on their parent node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementwiseOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

impl ElementwiseOpKind {
    /// Kebab-case name used by the SIR text format (matches the
    /// convention set by [`Scope::name`] / [`Feature::name`]).
    pub fn name(&self) -> &'static str {
        match self {
            ElementwiseOpKind::Add => "add",
            ElementwiseOpKind::Sub => "sub",
            ElementwiseOpKind::Mul => "mul",
            ElementwiseOpKind::Div => "div",
            ElementwiseOpKind::Pow => "pow",
        }
    }

    /// Inverse of [`Self::name`].  Returns `None` for unknown names.
    pub fn from_name(s: &str) -> Option<ElementwiseOpKind> {
        Some(match s {
            "add" => ElementwiseOpKind::Add,
            "sub" => ElementwiseOpKind::Sub,
            "mul" => ElementwiseOpKind::Mul,
            "div" => ElementwiseOpKind::Div,
            "pow" => ElementwiseOpKind::Pow,
            _ => return None,
        })
    }
}

/// One subscript argument to [`Expr::IndexGet`] / [`Stmt::IndexSet`]
/// (SIR22).  A MATLAB index expression `A(i, :, 1:3)` lowers to three
/// `IndexArg`s, one per axis, in source order.
///
/// ```text
/// A(3)      →  [IndexArg::Scalar(IntLit(2))]     -- already 0-based
/// A(:, 2)   →  [IndexArg::Whole, IndexArg::Scalar(IntLit(1))]
/// A(1:5)    →  [IndexArg::Range(Range { .. })]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum IndexArg {
    /// A single-element subscript on this axis (MATLAB `A(3)`) —
    /// already translated to a 0-based index expression by the
    /// frontend (including any `end`-relative resolution; see the
    /// SIR22 spec).
    Scalar(Box<Expr>),
    /// `:` — every element on this axis (MATLAB `A(:, k)`).
    Whole,
    /// A range subscript on this axis (MATLAB `A(1:5)`) — reuses
    /// [`Expr::Range`] as the index argument rather than duplicating
    /// its three operand slots here.
    Range(Box<Expr>),
}

/// A single capture provided to `MakeClosure`.  The `name` matches a
/// `Capture` in the referenced Function; `value` is evaluated at the
/// call site and stored in the closure handle.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureValue {
    pub name: String,
    pub value: Expr,
}

/// A single key/value entry in a `MapLit`.
#[derive(Debug, Clone, PartialEq)]
pub struct MapEntry {
    pub key: Expr,
    pub value: Expr,
}

impl Expr {
    /// Source span of the expression.
    pub fn span(&self) -> &Span {
        match self {
            Expr::IntLit { span, .. } => span,
            Expr::BoolLit { span, .. } => span,
            Expr::NilLit { span } => span,
            Expr::SymLit { span, .. } => span,
            Expr::StrLit { span, .. } => span,
            Expr::VarRef { span, .. } => span,
            Expr::If { span, .. } => span,
            Expr::Block(b) => &b.span,
            Expr::DirectCall { span, .. } => span,
            Expr::IndirectCall { span, .. } => span,
            Expr::BuiltinCall { span, .. } => span,
            Expr::MakeClosure { span, .. } => span,
            Expr::Intrinsic { span, .. } => span,
            Expr::FloatLit { span, .. } => span,
            Expr::SeqLit { span, .. } => span,
            Expr::SeqIndex { span, .. } => span,
            Expr::SeqLen { span, .. } => span,
            Expr::MapLit { span, .. } => span,
            Expr::MapGet { span, .. } => span,
            Expr::LogicalAnd { span, .. } => span,
            Expr::LogicalOr { span, .. } => span,
            Expr::StrConcat { span, .. } => span,
            Expr::KeywordArg { span, .. } => span,
            Expr::ArrayLit { span, .. } => span,
            Expr::Range { span, .. } => span,
            Expr::MatMul { span, .. } => span,
            Expr::ElementwiseOp { span, .. } => span,
            Expr::Transpose { span, .. } => span,
            Expr::IndexGet { span, .. } => span,
            Expr::Convert { span, .. } => span,
            Expr::SymSymbol { span, .. } => span,
            Expr::SymRational { span, .. } => span,
            Expr::SymApply { span, .. } => span,
            Expr::SymPatternBlank { span, .. } => span,
            Expr::SymPatternNamed { span, .. } => span,
            Expr::SymRule { span, .. } => span,
            Expr::SymReplaceAll { span, .. } => span,
        }
    }

    /// A short discriminator string for diagnostics — the head
    /// keyword of the text format.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Expr::IntLit { .. } => "int",
            Expr::BoolLit { .. } => "bool",
            Expr::NilLit { .. } => "nil",
            Expr::SymLit { .. } => "sym",
            Expr::StrLit { .. } => "str",
            Expr::VarRef { .. } => "var-ref",
            Expr::If { .. } => "if",
            Expr::Block(_) => "block",
            Expr::DirectCall { .. } => "direct-call",
            Expr::IndirectCall { .. } => "indirect-call",
            Expr::BuiltinCall { .. } => "builtin-call",
            Expr::MakeClosure { .. } => "make-closure",
            Expr::Intrinsic { .. } => "intrinsic",
            Expr::FloatLit { .. } => "float",
            Expr::SeqLit { .. } => "seq",
            Expr::SeqIndex { .. } => "seq-index",
            Expr::SeqLen { .. } => "seq-len",
            Expr::MapLit { .. } => "map",
            Expr::MapGet { .. } => "map-get",
            Expr::LogicalAnd { .. } => "and",
            Expr::LogicalOr { .. } => "or",
            Expr::StrConcat { .. } => "str-concat",
            Expr::KeywordArg { .. } => "keyword-arg",
            Expr::ArrayLit { .. } => "array",
            Expr::Range { .. } => "range",
            Expr::MatMul { .. } => "matmul",
            Expr::ElementwiseOp { .. } => "elementwise-op",
            Expr::Transpose { .. } => "transpose",
            Expr::IndexGet { .. } => "index-get",
            Expr::Convert { .. } => "convert",
            Expr::SymSymbol { .. } => "sym-symbol",
            Expr::SymRational { .. } => "sym-rational",
            Expr::SymApply { .. } => "sym-apply",
            Expr::SymPatternBlank { .. } => "sym-pattern-blank",
            Expr::SymPatternNamed { .. } => "sym-pattern-named",
            Expr::SymRule { .. } => "sym-rule",
            Expr::SymReplaceAll { .. } => "sym-replace-all",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn s() -> Span {
        Span::point("<t>", 1, 1)
    }

    #[test]
    fn expr_span_helper() {
        let e = Expr::IntLit {
            value: 42,
            span: s(),
        };
        assert_eq!(e.span(), &s());
        assert_eq!(e.kind_name(), "int");
    }

    #[test]
    fn scope_name_round_trips() {
        for sc in [
            Scope::Local,
            Scope::Param,
            Scope::Capture,
            Scope::Global,
            Scope::Builtin,
        ] {
            assert_eq!(Scope::from_name(sc.name()), Some(sc));
        }
        assert_eq!(Scope::from_name("bogus"), None);
    }

    #[test]
    fn stmt_span_helper() {
        let st = Stmt::ExprStmt {
            expr: Expr::NilLit { span: s() },
            span: s(),
        };
        assert_eq!(st.span(), &s());
    }

    #[test]
    fn sir16_stmt_kinds_have_spans() {
        // Exercise span() for every new statement variant so a future
        // refactor that drops a span field gets caught here.
        let cases: Vec<Stmt> = vec![
            Stmt::Assign {
                name: "x".into(),
                scope: Scope::Local,
                value: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            },
            Stmt::While {
                cond: Expr::BoolLit {
                    value: true,
                    span: s(),
                },
                body: Block {
                    stmts: vec![],
                    value: Expr::NilLit { span: s() },
                    span: s(),
                },
                span: s(),
            },
            Stmt::ForRange {
                var: "i".into(),
                start: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                stop: Expr::IntLit {
                    value: 10,
                    span: s(),
                },
                step: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                body: Block {
                    stmts: vec![],
                    value: Expr::NilLit { span: s() },
                    span: s(),
                },
                span: s(),
            },
            Stmt::ForEach {
                var: "x".into(),
                iter: Expr::SeqLit {
                    items: vec![],
                    span: s(),
                },
                body: Block {
                    stmts: vec![],
                    value: Expr::NilLit { span: s() },
                    span: s(),
                },
                span: s(),
            },
            Stmt::SeqSet {
                seq: Expr::VarRef {
                    name: "xs".into(),
                    scope: Scope::Local,
                    span: s(),
                },
                index: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                value: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            },
            Stmt::MapSet {
                map: Expr::VarRef {
                    name: "d".into(),
                    scope: Scope::Local,
                    span: s(),
                },
                key: Expr::StrLit {
                    value: "k".into(),
                    span: s(),
                },
                value: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            },
        ];
        for st in &cases {
            assert_eq!(st.span(), &s());
        }
    }

    // `3.14` is an arbitrary float literal test value, not an approximation of PI.
    #[allow(clippy::approx_constant)]
    #[test]
    fn sir16_expr_kind_names() {
        let span = s();
        let cases: Vec<(Expr, &'static str)> = vec![
            (
                Expr::FloatLit {
                    value: 3.14,
                    span: span.clone(),
                },
                "float",
            ),
            (
                Expr::SeqLit {
                    items: vec![],
                    span: span.clone(),
                },
                "seq",
            ),
            (
                Expr::SeqIndex {
                    seq: Box::new(Expr::NilLit { span: span.clone() }),
                    index: Box::new(Expr::IntLit {
                        value: 0,
                        span: span.clone(),
                    }),
                    span: span.clone(),
                },
                "seq-index",
            ),
            (
                Expr::SeqLen {
                    seq: Box::new(Expr::NilLit { span: span.clone() }),
                    span: span.clone(),
                },
                "seq-len",
            ),
            (
                Expr::MapLit {
                    entries: vec![],
                    span: span.clone(),
                },
                "map",
            ),
            (
                Expr::MapGet {
                    map: Box::new(Expr::NilLit { span: span.clone() }),
                    key: Box::new(Expr::NilLit { span: span.clone() }),
                    span: span.clone(),
                },
                "map-get",
            ),
            (
                Expr::LogicalAnd {
                    lhs: Box::new(Expr::BoolLit {
                        value: true,
                        span: span.clone(),
                    }),
                    rhs: Box::new(Expr::BoolLit {
                        value: false,
                        span: span.clone(),
                    }),
                    span: span.clone(),
                },
                "and",
            ),
            (
                Expr::LogicalOr {
                    lhs: Box::new(Expr::BoolLit {
                        value: true,
                        span: span.clone(),
                    }),
                    rhs: Box::new(Expr::BoolLit {
                        value: false,
                        span: span.clone(),
                    }),
                    span: span.clone(),
                },
                "or",
            ),
            (
                Expr::StrConcat {
                    parts: vec![
                        Expr::StrLit {
                            value: "a".into(),
                            span: span.clone(),
                        },
                        Expr::StrLit {
                            value: "b".into(),
                            span: span.clone(),
                        },
                    ],
                    span: span.clone(),
                },
                "str-concat",
            ),
        ];
        for (e, expected) in &cases {
            assert_eq!(e.kind_name(), *expected);
            assert_eq!(e.span(), &span);
        }
    }

    // `3.14` is an arbitrary float literal test value, not an approximation of PI.
    #[allow(clippy::approx_constant)]
    #[test]
    fn float_lit_partial_eq_handles_nan() {
        // f64::NAN is never equal to itself — Expr only impls
        // PartialEq, not Eq, and that's the reason.  This test pins
        // the contract.
        let a = Expr::FloatLit {
            value: f64::NAN,
            span: s(),
        };
        let b = Expr::FloatLit {
            value: f64::NAN,
            span: s(),
        };
        assert_ne!(a, b);
        let c = Expr::FloatLit {
            value: 3.14,
            span: s(),
        };
        let d = Expr::FloatLit {
            value: 3.14,
            span: s(),
        };
        assert_eq!(c, d);
    }

    // ── KW1: keyword parameters & arguments ────────────────────────

    #[test]
    fn keyword_arg_span_and_kind_name() {
        let e = Expr::KeywordArg {
            name: "a".into(),
            value: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(e.span(), &s());
        assert_eq!(e.kind_name(), "keyword-arg");
    }

    /// A `Keyword` param with `default == None` is REQUIRED; with
    /// `default == Some(_)` it is OPTIONAL.  This test pins the truth table
    /// documented on `ParamKind::Keyword`.
    #[test]
    fn keyword_param_required_vs_optional_via_default() {
        let required = Param {
            name: "x".into(),
            sir_type: None,
            kind: ParamKind::Keyword,
            default: None,
            span: s(),
        };
        let optional = Param {
            name: "y".into(),
            sir_type: None,
            kind: ParamKind::Keyword,
            default: Some(Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            })),
            span: s(),
        };
        assert_eq!(required.kind, ParamKind::Keyword);
        assert!(required.default.is_none(), "kw with no default = required");
        assert!(optional.default.is_some(), "kw with default = optional");
    }

    fn kw_param(name: &str, default: Option<i64>) -> Param {
        Param {
            name: name.into(),
            sir_type: None,
            kind: ParamKind::Keyword,
            default: default.map(|v| {
                Box::new(Expr::IntLit {
                    value: v,
                    span: s(),
                })
            }),
            span: s(),
        }
    }

    fn fn_with_params(params: Vec<Param>) -> Function {
        Function {
            name: "f".into(),
            params,
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }
    }

    #[test]
    fn keyword_params_helper_selects_only_keyword_kind() {
        // def f(a, x:, y: 1, **rest) → keyword_params() == [x, y].
        let f = fn_with_params(vec![
            Param {
                name: "a".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: None,
                span: s(),
            },
            kw_param("x", None),
            kw_param("y", Some(1)),
            Param {
                name: "rest".into(),
                sir_type: None,
                kind: ParamKind::KwRest,
                default: None,
                span: s(),
            },
        ]);
        let kws: Vec<&str> = f.keyword_params().iter().map(|p| p.name.as_str()).collect();
        assert_eq!(kws, vec!["x", "y"]);
    }

    #[test]
    fn missing_keywords_returns_unsupplied_keyword_params() {
        // def f(x:, y: 1, z: 2)
        let f = fn_with_params(vec![
            kw_param("x", None),
            kw_param("y", Some(1)),
            kw_param("z", Some(2)),
        ]);
        // Supplying x and y leaves z omitted.
        let missing: Vec<&str> = f
            .missing_keywords(&["x", "y"])
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(missing, vec!["z"]);
        // Supplying only x leaves y and z omitted (both carry defaults).
        let missing2: Vec<&str> = f
            .missing_keywords(&["x"])
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(missing2, vec!["y", "z"]);
        // Supplying all leaves nothing.
        assert!(f.missing_keywords(&["x", "y", "z"]).is_empty());
    }

    // ── SIR22: array/matrix nodes ───────────────────────────────────

    #[test]
    fn index_set_is_a_stmt_not_an_expr() {
        // SIR22 §"Effects": IndexSet is lowered as a Stmt-position
        // mutation (like Assign), not a value-producing Expr — pin that
        // by constructing it as a Stmt and confirming span() dispatches
        // through Stmt::span, exactly like the sir16_stmt_kinds test does
        // for Assign/SeqSet/MapSet.
        let st = Stmt::IndexSet {
            target: Box::new(Expr::VarRef {
                name: "a".into(),
                scope: Scope::Local,
                span: s(),
            }),
            indices: vec![IndexArg::Scalar(Box::new(Expr::IntLit {
                value: 0,
                span: s(),
            }))],
            value: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(st.span(), &s());
        // There is no Expr::IndexSet variant — the type system itself
        // enforces "not an Expr": this file would fail to compile if
        // such a variant existed and this test tried to skip it, so the
        // absence is exercised implicitly by every other test in this
        // module compiling against the real Expr enum.
    }

    #[test]
    fn sir22_stmt_index_set_has_span() {
        let cases: Vec<Stmt> = vec![Stmt::IndexSet {
            target: Box::new(Expr::VarRef {
                name: "m".into(),
                scope: Scope::Local,
                span: s(),
            }),
            indices: vec![
                IndexArg::Whole,
                IndexArg::Scalar(Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                })),
            ],
            value: Box::new(Expr::IntLit {
                value: 9,
                span: s(),
            }),
            span: s(),
        }];
        for st in &cases {
            assert_eq!(st.span(), &s());
        }
    }

    #[test]
    fn sir22_expr_kind_names_and_spans() {
        let span = s();
        let cases: Vec<(Expr, &'static str)> = vec![
            (
                Expr::ArrayLit {
                    rows: vec![
                        vec![
                            Expr::IntLit {
                                value: 1,
                                span: span.clone(),
                            },
                            Expr::IntLit {
                                value: 2,
                                span: span.clone(),
                            },
                        ],
                        vec![
                            Expr::IntLit {
                                value: 3,
                                span: span.clone(),
                            },
                            Expr::IntLit {
                                value: 4,
                                span: span.clone(),
                            },
                        ],
                    ],
                    span: span.clone(),
                },
                "array",
            ),
            (
                Expr::Range {
                    start: Box::new(Expr::IntLit {
                        value: 1,
                        span: span.clone(),
                    }),
                    step: None,
                    stop: Box::new(Expr::IntLit {
                        value: 5,
                        span: span.clone(),
                    }),
                    span: span.clone(),
                },
                "range",
            ),
            (
                Expr::MatMul {
                    lhs: Box::new(Expr::VarRef {
                        name: "a".into(),
                        scope: Scope::Local,
                        span: span.clone(),
                    }),
                    rhs: Box::new(Expr::VarRef {
                        name: "b".into(),
                        scope: Scope::Local,
                        span: span.clone(),
                    }),
                    span: span.clone(),
                },
                "matmul",
            ),
            (
                Expr::ElementwiseOp {
                    op: ElementwiseOpKind::Mul,
                    lhs: Box::new(Expr::VarRef {
                        name: "a".into(),
                        scope: Scope::Local,
                        span: span.clone(),
                    }),
                    rhs: Box::new(Expr::VarRef {
                        name: "b".into(),
                        scope: Scope::Local,
                        span: span.clone(),
                    }),
                    span: span.clone(),
                },
                "elementwise-op",
            ),
            (
                Expr::Transpose {
                    target: Box::new(Expr::VarRef {
                        name: "a".into(),
                        scope: Scope::Local,
                        span: span.clone(),
                    }),
                    conjugate: true,
                    span: span.clone(),
                },
                "transpose",
            ),
            (
                Expr::IndexGet {
                    target: Box::new(Expr::VarRef {
                        name: "a".into(),
                        scope: Scope::Local,
                        span: span.clone(),
                    }),
                    indices: vec![IndexArg::Whole],
                    span: span.clone(),
                },
                "index-get",
            ),
        ];
        for (e, expected) in &cases {
            assert_eq!(e.kind_name(), *expected);
            assert_eq!(e.span(), &span);
        }
    }

    #[test]
    fn range_with_explicit_step() {
        // 0:2:10 — step is `Some`, distinct from the default-step form.
        let r = Expr::Range {
            start: Box::new(Expr::IntLit {
                value: 0,
                span: s(),
            }),
            step: Some(Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            })),
            stop: Box::new(Expr::IntLit {
                value: 10,
                span: s(),
            }),
            span: s(),
        };
        match r {
            Expr::Range {
                step: Some(step), ..
            } => {
                assert_eq!(
                    *step,
                    Expr::IntLit {
                        value: 2,
                        span: s()
                    }
                );
            }
            _ => panic!("expected Range with explicit step"),
        }
    }

    #[test]
    fn transpose_conjugate_flag_distinguishes_tick_from_dot_tick() {
        // MATLAB `A'` (conjugate transpose) vs `A.'` (plain transpose) —
        // the SIR22 spec maps both onto the same node, distinguished only
        // by `conjugate`.
        let tick = Expr::Transpose {
            target: Box::new(Expr::VarRef {
                name: "a".into(),
                scope: Scope::Local,
                span: s(),
            }),
            conjugate: true,
            span: s(),
        };
        let dot_tick = Expr::Transpose {
            target: Box::new(Expr::VarRef {
                name: "a".into(),
                scope: Scope::Local,
                span: s(),
            }),
            conjugate: false,
            span: s(),
        };
        assert_ne!(tick, dot_tick);
    }

    #[test]
    fn elementwise_op_kind_name_round_trips() {
        for op in [
            ElementwiseOpKind::Add,
            ElementwiseOpKind::Sub,
            ElementwiseOpKind::Mul,
            ElementwiseOpKind::Div,
            ElementwiseOpKind::Pow,
        ] {
            assert_eq!(ElementwiseOpKind::from_name(op.name()), Some(op));
        }
        assert_eq!(ElementwiseOpKind::from_name("bogus"), None);
    }

    #[test]
    fn index_arg_variants_construct() {
        // Scalar / Whole / Range — the three IndexArg shapes from the
        // SIR22 spec's worked examples (`A(3)`, `A(:, 2)`, `A(1:5)`).
        let scalar = IndexArg::Scalar(Box::new(Expr::IntLit {
            value: 2,
            span: s(),
        }));
        let whole = IndexArg::Whole;
        let range = IndexArg::Range(Box::new(Expr::Range {
            start: Box::new(Expr::IntLit {
                value: 0,
                span: s(),
            }),
            step: None,
            stop: Box::new(Expr::IntLit {
                value: 5,
                span: s(),
            }),
            span: s(),
        }));
        assert!(matches!(scalar, IndexArg::Scalar(_)));
        assert!(matches!(whole, IndexArg::Whole));
        assert!(matches!(range, IndexArg::Range(_)));
    }

    #[test]
    fn index_get_carries_multiple_axes() {
        // A(i, :, 1:3) — one IndexArg per axis, in source order.
        let e = Expr::IndexGet {
            target: Box::new(Expr::VarRef {
                name: "a".into(),
                scope: Scope::Local,
                span: s(),
            }),
            indices: vec![
                IndexArg::Scalar(Box::new(Expr::IntLit {
                    value: 0,
                    span: s(),
                })),
                IndexArg::Whole,
                IndexArg::Range(Box::new(Expr::Range {
                    start: Box::new(Expr::IntLit {
                        value: 0,
                        span: s(),
                    }),
                    step: None,
                    stop: Box::new(Expr::IntLit {
                        value: 3,
                        span: s(),
                    }),
                    span: s(),
                })),
            ],
            span: s(),
        };
        if let Expr::IndexGet { indices, .. } = &e {
            assert_eq!(indices.len(), 3);
        } else {
            panic!("expected IndexGet");
        }
    }

    // ── SIR23: symbolic expression + pattern/rewrite nodes ───────────

    #[test]
    fn sir23_expr_kind_names_and_spans() {
        let span = s();
        let cases: Vec<(Expr, &'static str)> = vec![
            (
                Expr::SymSymbol {
                    name: "x".into(),
                    span: span.clone(),
                },
                "sym-symbol",
            ),
            (
                Expr::SymRational {
                    numer: 1,
                    denom: 3,
                    span: span.clone(),
                },
                "sym-rational",
            ),
            (
                Expr::SymApply {
                    head: Box::new(Expr::SymSymbol {
                        name: "Plus".into(),
                        span: span.clone(),
                    }),
                    args: vec![
                        Expr::IntLit {
                            value: 1,
                            span: span.clone(),
                        },
                        Expr::IntLit {
                            value: 2,
                            span: span.clone(),
                        },
                    ],
                    span: span.clone(),
                },
                "sym-apply",
            ),
            (
                Expr::SymPatternBlank {
                    head: None,
                    span: span.clone(),
                },
                "sym-pattern-blank",
            ),
            (
                Expr::SymPatternNamed {
                    name: "x".into(),
                    pattern: Box::new(Expr::SymPatternBlank {
                        head: None,
                        span: span.clone(),
                    }),
                    span: span.clone(),
                },
                "sym-pattern-named",
            ),
            (
                Expr::SymRule {
                    lhs: Box::new(Expr::SymSymbol {
                        name: "x".into(),
                        span: span.clone(),
                    }),
                    rhs: Box::new(Expr::IntLit {
                        value: 0,
                        span: span.clone(),
                    }),
                    delayed: false,
                    span: span.clone(),
                },
                "sym-rule",
            ),
            (
                Expr::SymReplaceAll {
                    expr: Box::new(Expr::SymSymbol {
                        name: "x".into(),
                        span: span.clone(),
                    }),
                    rules: vec![],
                    repeated: false,
                    span: span.clone(),
                },
                "sym-replace-all",
            ),
        ];
        for (e, expected) in &cases {
            assert_eq!(e.kind_name(), *expected);
            assert_eq!(e.span(), &span);
        }
    }

    #[test]
    fn sym_apply_head_is_an_expr_not_a_bare_string() {
        // The SIR23 spec's explicit callout: `head` is a full `Expr` (not
        // a bare `String`) because a *computed* head is legal Wolfram
        // (`f[x][y]` applies the result of `f[x]` to `y`).  Pin that a
        // SymApply's own head may itself be a SymApply.
        let inner = Expr::SymApply {
            head: Box::new(Expr::SymSymbol {
                name: "f".into(),
                span: s(),
            }),
            args: vec![Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            }],
            span: s(),
        };
        let outer = Expr::SymApply {
            head: Box::new(inner),
            args: vec![Expr::SymSymbol {
                name: "y".into(),
                span: s(),
            }],
            span: s(),
        };
        match outer {
            Expr::SymApply { head, .. } => {
                assert!(matches!(*head, Expr::SymApply { .. }));
            }
            _ => panic!("expected SymApply"),
        }
    }

    #[test]
    fn sym_pattern_blank_head_constrained_vs_bare() {
        // Wolfram `_` (head: None) vs `_h` (head: Some(SymSymbol("h"))).
        let bare = Expr::SymPatternBlank {
            head: None,
            span: s(),
        };
        let constrained = Expr::SymPatternBlank {
            head: Some(Box::new(Expr::SymSymbol {
                name: "Integer".into(),
                span: s(),
            })),
            span: s(),
        };
        assert_ne!(bare, constrained);
        match constrained {
            Expr::SymPatternBlank { head: Some(h), .. } => {
                assert_eq!(
                    *h,
                    Expr::SymSymbol {
                        name: "Integer".into(),
                        span: s()
                    }
                );
            }
            _ => panic!("expected head-constrained SymPatternBlank"),
        }
    }

    #[test]
    fn sym_pattern_named_desugars_x_underscore() {
        // Wolfram `x_` desugars to SymPatternNamed { name: "x", pattern:
        // SymPatternBlank { head: None } } per the SIR23 spec.
        let e = Expr::SymPatternNamed {
            name: "x".into(),
            pattern: Box::new(Expr::SymPatternBlank {
                head: None,
                span: s(),
            }),
            span: s(),
        };
        match e {
            Expr::SymPatternNamed { name, pattern, .. } => {
                assert_eq!(name, "x");
                assert!(matches!(*pattern, Expr::SymPatternBlank { head: None, .. }));
            }
            _ => panic!("expected SymPatternNamed"),
        }
    }

    #[test]
    fn sym_rule_delayed_flag_distinguishes_rule_from_rule_delayed() {
        // `->` (Rule, delayed: false) vs `:>` (RuleDelayed, delayed: true).
        let rule = Expr::SymRule {
            lhs: Box::new(Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            delayed: false,
            span: s(),
        };
        let rule_delayed = Expr::SymRule {
            lhs: Box::new(Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            delayed: true,
            span: s(),
        };
        assert_ne!(rule, rule_delayed);
    }

    #[test]
    fn sym_replace_all_repeated_flag_distinguishes_replace_all_from_repeated() {
        // `/.` (ReplaceAll, repeated: false) vs `//.` (ReplaceRepeated,
        // repeated: true).
        let once = Expr::SymReplaceAll {
            expr: Box::new(Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            }),
            rules: vec![],
            repeated: false,
            span: s(),
        };
        let fixed_point = Expr::SymReplaceAll {
            expr: Box::new(Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            }),
            rules: vec![],
            repeated: true,
            span: s(),
        };
        assert_ne!(once, fixed_point);
    }

    #[test]
    fn sym_rational_carries_numer_denom_without_reducing() {
        // The IR is a carrier, not a verifier (SIR10): it does not itself
        // reduce 2/4 to 1/2 — that's the frontend's job, mirroring
        // `symbolic_ir::IRNode::rational`'s own contract.
        let unreduced = Expr::SymRational {
            numer: 2,
            denom: 4,
            span: s(),
        };
        match unreduced {
            Expr::SymRational { numer, denom, .. } => {
                assert_eq!((numer, denom), (2, 4));
            }
            _ => panic!("expected SymRational"),
        }
    }

    #[test]
    fn expr_kind_names_exhaustive() {
        let span = s();
        let cases: Vec<Expr> = vec![
            Expr::IntLit {
                value: 1,
                span: span.clone(),
            },
            Expr::BoolLit {
                value: true,
                span: span.clone(),
            },
            Expr::NilLit { span: span.clone() },
            Expr::SymLit {
                name: "x".into(),
                span: span.clone(),
            },
            Expr::StrLit {
                value: "y".into(),
                span: span.clone(),
            },
            Expr::VarRef {
                name: "z".into(),
                scope: Scope::Local,
                span: span.clone(),
            },
            Expr::DirectCall {
                fn_name: "f".into(),
                args: vec![],
                effects: EffectSet::PURE,
                span: span.clone(),
            },
            Expr::BuiltinCall {
                name: "+".into(),
                args: vec![],
                effects: EffectSet::PURE,
                span: span.clone(),
            },
            Expr::MakeClosure {
                fn_name: "__lambda_0".into(),
                captures: vec![],
                span: span.clone(),
            },
        ];
        let names: Vec<&'static str> = cases.iter().map(Expr::kind_name).collect();
        assert_eq!(
            names,
            vec![
                "int",
                "bool",
                "nil",
                "sym",
                "str",
                "var-ref",
                "direct-call",
                "builtin-call",
                "make-closure"
            ]
        );
    }
}
