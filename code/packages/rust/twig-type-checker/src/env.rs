//! Type environment and scope management for TW05-B.
//!
//! ## Two structures, two responsibilities
//!
//! ### `TypeEnv` — the global declaration table
//!
//! `TypeEnv` is built during **Pass 1** (declaration collection) and then
//! treated as read-only during **Pass 2** (expression walking).  It maps
//! names declared at the *module level* to their types.
//!
//! Think of it like a global symbol table in a C compiler: it knows about
//! every `struct`, `typedef`, and top-level `int foo(...)` before any
//! function body is compiled.
//!
//! Contents:
//! - `globals` — every `(define name …)` binding and record/union constructors.
//! - `aliases` — every `(type Name expr)` alias (stored as `TypeExpr` for
//!   lazy resolution — see `kinds::type_expr_to_kind`).
//! - `records` — every `(record Name …)` definition's field names.
//! - `unions` — every `(union Name …)` definition's variant names.
//!
//! ### `ScopeStack` — the local binding stack
//!
//! `ScopeStack` tracks bindings introduced by `lambda` parameters, `let`
//! bindings, and `match` arm pattern bindings.  It's a stack of frames —
//! push a frame on entry to a new scope, pop it on exit.
//!
//! Name resolution during Pass 2: first search the `ScopeStack` (top frame
//! first), then fall back to `TypeEnv::globals`.  This mirrors lexical
//! scoping in Scheme: inner bindings shadow outer ones.
//!
//! ## Example of scoping
//!
//! ```scheme
//! (define x 1)           ;; globals: x → Any
//! (define (f x) x)       ;; lambda param x shadows global x
//! ```
//!
//! When type-checking the body of `f`:
//! - ScopeStack: [{x → Any}]   (lambda frame)
//! - globals: {x → Any, f → Function{1}}
//!
//! `VarRef("x")` hits the scope stack first → `Any`.
//! `VarRef("f")` misses the scope stack, falls to globals → `Function{1}`.

use std::collections::HashMap;

use twig_parser::TypeExpr;

use crate::kinds::TwigKind;

// ---------------------------------------------------------------------------
// TypeEnv
// ---------------------------------------------------------------------------

/// Global declaration table built during Pass 1 and read during Pass 2.
///
/// All five maps use the declared name as the key (exactly as it appears in
/// source: `"Span"`, `"Expr"`, `"x"`, etc.).
#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    /// Top-level name → inferred `TwigKind`.
    ///
    /// Populated from:
    /// - `Form::Define` with a `Lambda` expr → `Function { arity }`.
    /// - `Form::Define` with a `type_annotation` → from annotation.
    /// - `Form::Define` value (no annotation) → `Any`.
    /// - `Form::RecordDef` → `Record(name)` for the constructor.
    /// - `Form::UnionDef` → `Function { arity }` for each variant constructor.
    pub globals: HashMap<String, TwigKind>,

    /// Type alias name → the `TypeExpr` it expands to.
    ///
    /// Lazily resolved via `kinds::type_expr_to_kind` rather than eagerly
    /// expanded — this avoids infinite loops for recursive aliases (the depth
    /// guard in `type_expr_to_kind_depth` handles those).
    pub aliases: HashMap<String, TypeExpr>,

    /// Record name → ordered field names.
    ///
    /// Used by:
    /// - Constructor arity checking (arity = fields.len()).
    /// - `MatchPat::Variant` binding assignment (fields[i] gets bound to
    ///   the i-th binding name).
    pub records: HashMap<String, Vec<String>>,

    /// Union name → ordered variant names.
    ///
    /// Used by exhaustiveness checking: the variant list is the "complete set"
    /// against which covered patterns are compared.
    pub unions: HashMap<String, Vec<String>>,
}

impl TypeEnv {
    /// Construct a fresh, empty `TypeEnv`.
    pub fn new() -> Self {
        TypeEnv::default()
    }

    /// Register a type alias declaration `(type Name expr)`.
    ///
    /// Aliases are stored unevaluated — resolution happens on demand in
    /// `kinds::type_expr_to_kind` to avoid expansion-time cycles.
    pub fn register_alias(&mut self, name: String, expr: TypeExpr) {
        self.aliases.insert(name, expr);
    }

    /// Register a record definition and expose its constructor in `globals`.
    ///
    /// For `(record Span (start : Nat) (end : Nat))`:
    /// - `env.records["Span"] = ["start", "end"]`
    /// - `env.globals["Span"] = TwigKind::Record("Span")`  ← the type itself
    ///
    /// The IR compiler also generates `span-start`, `span-end`, `span?`
    /// builtins, but those carry `Any` in the type env for now (no declared
    /// return types).
    pub fn register_record(&mut self, r: &twig_parser::RecordDef) {
        let field_names: Vec<String> = r.fields.iter().map(|f| f.name.clone()).collect();
        self.records.insert(r.name.clone(), field_names);
        // The record name itself behaves as a type tag, not a callable — store
        // as Record so type_expr_to_kind can map the name back.
        self.globals
            .insert(r.name.clone(), TwigKind::Record(r.name.clone()));
    }

    /// Register a union definition and expose variant constructors in `globals`.
    ///
    /// For `(union Expr (IntLit (value : Int)) (NameRef (name : Symbol)))`:
    /// - `env.unions["Expr"] = ["IntLit", "NameRef"]`
    /// - `env.globals["IntLit"] = Function { arity: 1 }`  ← constructor
    /// - `env.globals["NameRef"] = Function { arity: 1 }` ← constructor
    ///
    /// The union type name itself is stored as `Union("Expr")` so that a
    /// `VarRef` named `Expr` resolves correctly.
    pub fn register_union(&mut self, u: &twig_parser::UnionDef) {
        let variant_names: Vec<String> = u.variants.iter().map(|v| v.name.clone()).collect();
        self.unions.insert(u.name.clone(), variant_names);
        // Store the union type name itself.
        self.globals
            .insert(u.name.clone(), TwigKind::Union(u.name.clone()));
        // Expose each variant constructor as a callable function.
        for v in &u.variants {
            self.globals.insert(
                v.name.clone(),
                TwigKind::Function {
                    arity: v.fields.len(),
                },
            );
        }
    }

    /// Bind a top-level name to a `TwigKind` in `globals`.
    pub fn bind_global(&mut self, name: String, kind: TwigKind) {
        self.globals.insert(name, kind);
    }

    /// Look up a name in `globals`.  Returns `None` if not found.
    pub fn lookup_global(&self, name: &str) -> Option<&TwigKind> {
        self.globals.get(name)
    }
}

// ---------------------------------------------------------------------------
// ScopeStack
// ---------------------------------------------------------------------------

/// A stack of lexical frames for local bindings.
///
/// Each frame is a `HashMap<String, TwigKind>`.  Frames are pushed on entry
/// to `lambda`, `let`, and `match-arm` bodies, and popped on exit.
///
/// ## Lookup algorithm
///
/// Start from the top (most-recently-pushed) frame and walk downward.
/// Return the first hit.  If no frame contains the name, return `None`
/// and the caller checks `TypeEnv::globals`.
///
/// ## Example
///
/// ```scheme
/// (lambda (x)                ;; push frame: {x → Any}
///   (let ((y 1))             ;; push frame: {y → Int}
///     (+ x y)))              ;; lookup x → outer frame; lookup y → top frame
/// ;; both frames popped after let / lambda body
/// ```
#[derive(Debug, Default)]
pub struct ScopeStack {
    /// Stack of frames; the last element is the innermost (most recent) scope.
    frames: Vec<HashMap<String, TwigKind>>,
}

impl ScopeStack {
    /// Create a fresh, empty `ScopeStack`.
    pub fn new() -> Self {
        ScopeStack::default()
    }

    /// Push a new empty frame onto the stack.
    ///
    /// Call this at the start of a `lambda` body, a `let` body, or a
    /// `match` arm body.
    pub fn push_frame(&mut self) {
        self.frames.push(HashMap::new());
    }

    /// Pop the innermost frame off the stack.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty (a push/pop mismatch in `check.rs`).
    pub fn pop_frame(&mut self) {
        self.frames
            .pop()
            .expect("ScopeStack::pop_frame called on empty stack");
    }

    /// Bind `name` to `kind` in the innermost frame.
    ///
    /// # Panics
    ///
    /// Panics if no frame has been pushed yet (a push/pop ordering bug in
    /// `check.rs`).
    pub fn bind(&mut self, name: &str, kind: TwigKind) {
        self.frames
            .last_mut()
            .expect("ScopeStack::bind called with no active frame")
            .insert(name.to_owned(), kind);
    }

    /// Search for `name` from innermost frame outward.
    ///
    /// Returns a reference to the first matching `TwigKind`, or `None` if
    /// the name isn't in any frame.
    pub fn lookup(&self, name: &str) -> Option<&TwigKind> {
        // Walk from the top of the stack downward (last frame first).
        for frame in self.frames.iter().rev() {
            if let Some(kind) = frame.get(name) {
                return Some(kind);
            }
        }
        None
    }
}
