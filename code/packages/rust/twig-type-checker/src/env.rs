//! Type environment and scope management for TW05-B + TW05-C.
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
//! - `fn_param_refinements` — per-function parameter `RefinedType`s (TW05-C).
//!   Index `i` is `None` when the i-th parameter carries no refinement
//!   predicate.  Populated by `classify_define` in Pass 1.
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

use lang_refined_types::RefinedType;
use twig_parser::TypeExpr;

use crate::kinds::TwigKind;

// ---------------------------------------------------------------------------
// TypeEnv
// ---------------------------------------------------------------------------

/// Global declaration table built during Pass 1 and read during Pass 2.
///
/// All maps use the declared name as the key (exactly as it appears in
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

    /// Per-function refined parameter types (TW05-C).
    ///
    /// `fn_param_refinements["f"]` is a `Vec<Option<RefinedType>>` where
    /// index `i` is:
    /// - `Some(rt)` when parameter `i` of `f` carries a refinement predicate
    ///   (e.g., `(Int 0 128)` or `(Member int 1 2 5)`).
    /// - `None` when parameter `i` has no refinement annotation.
    ///
    /// Functions with no refined parameters are *not* stored here at all —
    /// callers check `fn_param_refinements.get(fn_name)` before iterating.
    ///
    /// Populated by `register_fn_refinements` during Pass 1.
    pub fn_param_refinements: HashMap<String, Vec<Option<RefinedType>>>,
}

impl TypeEnv {
    /// Construct a `TypeEnv` pre-populated with all Twig runtime builtins.
    ///
    /// Every builtin is registered as [`TwigKind::Any`] so that:
    /// - Calls to builtins no longer produce "unresolved variable" warnings or
    ///   errors in `(typed lenient)` / `(typed strict)` modules.
    /// - Variadic builtins (`list`, `string-append`, …) are not false-positived
    ///   by an arity check — `Any` skips arity enforcement.
    ///
    /// Explicit `(define ...)` stubs in test prelude code (e.g. for refinement
    /// testing) **shadow** the pre-registered entries: `collect_forms` (Pass 1)
    /// overwrites `globals` with whatever kind the define produces, just like
    /// for user-declared functions.
    pub fn new() -> Self {
        let mut env = TypeEnv::default();
        env.register_builtins();
        env
    }

    /// Pre-populate `globals` with every Twig runtime builtin.
    ///
    /// ## Included names
    ///
    /// - All 43 names from the `BUILTINS` const in `twig-ir-compiler`
    ///   (arithmetic, comparisons, cons-cells, predicates, list ops, symbol
    ///   utilities, string/char ops, I/O, higher-order ops).
    /// - `and` and `or` — special-cased in the IR compiler (`compile_apply`)
    ///   but parsed as regular `Apply` nodes by `twig-parser`.  Without this
    ///   registration the type-checker emits "unresolved variable `and`".
    ///
    /// ## Not included
    ///
    /// - Record accessors / predicates (`span-start`, `Span?`, …) — these are
    ///   generated by the IR compiler as `call_builtin` and registered per-
    ///   module by `collect_forms` when it walks `(record ...)` declarations.
    /// - Import-supplied names — the module driver is responsible for passing
    ///   exported names from imported modules (multi-module strict mode TW05-P).
    fn register_builtins(&mut self) {
        let names: &[&str] = &[
            // ── Arithmetic / comparison (TW00 core + LANG52) ────────────────
            "+", "-", "*", "/", "=", "<", ">", "<=", ">=",
            "modulo", "remainder", "quotient",
            // ── Cons cells ──────────────────────────────────────────────────
            "cons", "car", "cdr",
            // ── Predicates ──────────────────────────────────────────────────
            "null?", "pair?", "number?", "symbol?", "not", "boolean?",
            "equal?", "list?",
            // ── List stdlib (LANG52) ─────────────────────────────────────────
            "list", "length", "append", "reverse", "list-ref", "assoc",
            // ── Symbol utilities ────────────────────────────────────────────
            "symbol-append",
            // ── Conversions ─────────────────────────────────────────────────
            "number->string", "string->symbol", "symbol->string",
            // ── String and char operations (LANG58) ─────────────────────────
            "string-length", "string-ref", "substring", "string-append",
            "string->number", "string=?", "string<?", "string>?",
            "char->integer", "integer->char",
            "char-alphabetic?", "char-numeric?", "char-whitespace?",
            // ── I/O ─────────────────────────────────────────────────────────
            "print",
            // ── Host I/O (LANG52) ───────────────────────────────────────────
            "host/write_string", "host/read_line", "host/read_file",
            // ── Higher-order list operations (LANG55) ───────────────────────
            "map", "filter", "fold-left", "fold-right",
            // ── Special forms that parse as regular calls (LANG52) ───────────
            // `and` and `or` are handled in the IR compiler via compile_apply,
            // not listed in BUILTINS, but look like function calls to the type
            // checker.
            "and", "or",
        ];
        for &name in names {
            self.globals.insert(name.to_owned(), TwigKind::Any);
        }
    }

    /// Register a type alias declaration `(type Name expr)`.
    ///
    /// Aliases are stored unevaluated — resolution happens on demand in
    /// `kinds::type_expr_to_kind` to avoid expansion-time cycles.
    pub fn register_alias(&mut self, name: String, expr: TypeExpr) {
        self.aliases.insert(name, expr);
    }

    /// Register a record definition and expose its constructor, predicate, and
    /// field accessors in `globals`.
    ///
    /// For `(record Span (start : int) (end : int))`:
    /// - `env.records["Span"] = ["start", "end"]`
    /// - `env.globals["Span"]       = TwigKind::Record("Span")`  ← constructor
    /// - `env.globals["span?"]      = TwigKind::Any`              ← predicate
    /// - `env.globals["span-start"] = TwigKind::Any`              ← accessor
    /// - `env.globals["span-end"]   = TwigKind::Any`              ← accessor
    ///
    /// ## Naming convention (mirrors `twig-ir-compiler`)
    ///
    /// The IR compiler uses `RecordName.to_lowercase()` as the accessor prefix:
    ///   - Predicate:  `{to_lowercase(name)}?`         (e.g. `span?`, `token?`)
    ///   - Accessor i: `{to_lowercase(name)}-{field}`  (e.g. `span-start`)
    ///
    /// All generated symbols carry `TwigKind::Any` — the type checker does not
    /// verify accessor return types; arity enforcement is done by the IR
    /// compiler.  `Any` suppresses "unresolved variable" errors without
    /// introducing false arity mismatches.
    pub fn register_record(&mut self, r: &twig_parser::RecordDef) {
        let field_names: Vec<String> = r.fields.iter().map(|f| f.name.clone()).collect();
        self.records.insert(r.name.clone(), field_names);
        // Constructor: stored as Record kind so type_expr_to_kind can map back.
        self.globals
            .insert(r.name.clone(), TwigKind::Record(r.name.clone()));

        // ── Generated symbols (TW05-P Part 1 / LANG70) ──────────────────────
        //
        // The IR compiler emits these as `call_builtin` instructions, matching
        // the naming conventions in twig-ir-compiler/src/compiler.rs lines
        // 343-348.  Registering them here allows modules that call their own
        // record accessors / predicates to pass in `(typed strict)` mode.
        let prefix = r.name.to_lowercase();
        // Predicate: <lower(RecordName)>?
        self.globals
            .insert(format!("{prefix}?"), TwigKind::Any);
        // Field accessors: <lower(RecordName)>-<field_name>
        for f in &r.fields {
            self.globals
                .insert(format!("{prefix}-{}", f.name), TwigKind::Any);
        }
    }

    /// Register a union definition and expose variant constructors, variant
    /// predicates, and variant field accessors in `globals`.
    ///
    /// For `(union Expr (IntLit (value : Int)) (NameRef (name : Symbol)))`:
    /// - `env.unions["Expr"] = ["IntLit", "NameRef"]`
    /// - `env.globals["Expr"]         = Union("Expr")`
    /// - `env.globals["IntLit"]       = Function { arity: 1 }` ← constructor
    /// - `env.globals["IntLit?"]      = Any`                   ← predicate
    /// - `env.globals["intlit-value"] = Any`                   ← field accessor
    /// - `env.globals["NameRef"]      = Function { arity: 1 }` ← constructor
    /// - `env.globals["NameRef?"]     = Any`                   ← predicate
    /// - `env.globals["nameref-name"] = Any`                   ← field accessor
    ///
    /// ## Naming convention (mirrors `twig-ir-compiler`)
    ///
    /// From twig-ir-compiler/src/compiler.rs lines 355-359:
    ///   - Variant predicate:      `{VariantName}?`              (original case,
    ///                             NOT lowercased — `TkInteger?` not `tkinteger?`)
    ///   - Variant field accessor: `{to_lowercase(VariantName)}-{field_name}`
    ///                             (e.g. `intlit-value`, `ifexpr-cond`)
    ///
    /// Note the asymmetry: record predicates use `to_lowercase(RecordName)` but
    /// union variant predicates keep the **original** case of `VariantName`.
    pub fn register_union(&mut self, u: &twig_parser::UnionDef) {
        let variant_names: Vec<String> = u.variants.iter().map(|v| v.name.clone()).collect();
        self.unions.insert(u.name.clone(), variant_names);
        // Union type name itself.
        self.globals
            .insert(u.name.clone(), TwigKind::Union(u.name.clone()));
        // Expose each variant constructor + generated symbols.
        for v in &u.variants {
            // Constructor: callable with arity = number of fields.
            self.globals.insert(
                v.name.clone(),
                TwigKind::Function {
                    arity: v.fields.len(),
                },
            );
            // ── Generated symbols (TW05-P Part 1 / LANG70) ─────────────────
            //
            // Variant predicate: <VariantName>?  (original case, not lowercased).
            // Mirrors compiler.rs line 355: format!("{}?", variant.name)
            self.globals
                .insert(format!("{}?", v.name), TwigKind::Any);
            // Variant field accessors: <lower(VariantName)>-<field_name>
            // Mirrors compiler.rs lines 357-359:
            //   let vprefix = variant.name.to_lowercase();
            //   format!("{vprefix}-{}", f.name)
            let vprefix = v.name.to_lowercase();
            for f in &v.fields {
                self.globals
                    .insert(format!("{vprefix}-{}", f.name), TwigKind::Any);
            }
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

    /// Register per-parameter refined types for a top-level function (TW05-C).
    ///
    /// Called by `classify_define` in Pass 1 when a `Lambda` has at least one
    /// parameter with a refinement annotation (`RangeInt` or `MembershipInt`).
    ///
    /// The `refinements` slice has one entry per parameter — `None` for
    /// parameters that carry no refinement predicate.
    ///
    /// This method is a no-op (does not store anything) when every entry is
    /// `None`, since there's nothing for the call-site checker to verify.
    pub fn register_fn_refinements(
        &mut self,
        fn_name: String,
        refinements: Vec<Option<RefinedType>>,
    ) {
        // Only store the entry when at least one parameter has a refinement.
        if refinements.iter().any(|r| r.is_some()) {
            self.fn_param_refinements.insert(fn_name, refinements);
        }
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
