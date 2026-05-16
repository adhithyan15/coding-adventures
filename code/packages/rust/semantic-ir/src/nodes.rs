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
use crate::types::SirType;

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// A function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub sir_type: Option<SirType>,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub value: Expr,
    pub span: Span,
}

/// Three kinds of statement.
///
/// The IR distinguishes parallel `let` ([`Stmt::LetBinding`]) from
/// sequential `let*` ([`Stmt::LetStarBinding`]) explicitly — the
/// frontend commits at lowering time and the backend never has to
/// ask which one is meant.
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

impl Stmt {
    pub fn span(&self) -> &Span {
        match self {
            Stmt::LetBinding { span, .. } => span,
            Stmt::LetStarBinding { span, .. } => span,
            Stmt::ExprStmt { span, .. } => span,
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
            _ => return None,
        })
    }
}

/// The expression grammar.  Every variant is a distinct semantic
/// concept.  Backends `match` on the variant and emit code; the IR
/// guarantees this match is exhaustive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    // ── atomic literals ────────────────────────────────────────────
    IntLit { value: i64, span: Span },
    BoolLit { value: bool, span: Span },
    NilLit { span: Span },
    SymLit { name: String, span: Span },
    StrLit { value: String, span: Span },

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
}

/// A single capture provided to `MakeClosure`.  The `name` matches a
/// `Capture` in the referenced Function; `value` is evaluated at the
/// call site and stored in the closure handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureValue {
    pub name: String,
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
        let e = Expr::IntLit { value: 42, span: s() };
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
    fn expr_kind_names_exhaustive() {
        let span = s();
        let cases: Vec<Expr> = vec![
            Expr::IntLit { value: 1, span: span.clone() },
            Expr::BoolLit { value: true, span: span.clone() },
            Expr::NilLit { span: span.clone() },
            Expr::SymLit { name: "x".into(), span: span.clone() },
            Expr::StrLit { value: "y".into(), span: span.clone() },
            Expr::VarRef { name: "z".into(), scope: Scope::Local, span: span.clone() },
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
                "int", "bool", "nil", "sym", "str", "var-ref",
                "direct-call", "builtin-call", "make-closure"
            ]
        );
    }
}
