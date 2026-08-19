//! # semantic-ir-to-rust
//!
//! Second backend for the narrow-waist Semantic IR — emits **self-
//! contained** Rust source code from a [`semantic_ir::Module`].
//!
//! Self-contained means every produced `.rs` file embeds an `__sir`
//! runtime module — no external crate dependencies; the output can
//! be compiled with `rustc <file>.rs` and run.
//!
//! ## Public API
//!
//! ```ignore
//! use semantic_ir_to_rust::{compile, RustBackend};
//! use semantic_ir::Backend;
//!
//! let module = /* a semantic_ir::Module from any frontend */;
//!
//! let artifact = compile(&module)?;
//! // or:
//! let backend = RustBackend::new();
//! let artifact = backend.compile(&module)?;
//! ```
//!
//! See [SIR13](../../../specs/SIR13-semantic-ir-to-rust.md) for the
//! per-node lowering rules.

mod emit;
mod runtime;

use semantic_ir::{
    Artifact, ArtifactMetadata, Backend, BackendError, BackendErrorKind, Expr, Feature, IndexArg,
    Module, ParamKind, Scope, Stmt,
};

pub use emit::sanitize_ident;

/// Convenience entry point.
pub fn compile(module: &Module) -> Result<Artifact, BackendError> {
    RustBackend::new().compile(module)
}

/// The v0 Rust backend.
pub struct RustBackend;

impl RustBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustBackend {
    fn default() -> Self {
        Self::new()
    }
}

const ACCEPTED_FEATURES: &[Feature] = &[
    Feature::Closures,
    Feature::Pairs,
    Feature::Symbols,
    Feature::Strings,
    Feature::DynamicTyping,
    Feature::OptionalTypeAnnotations,
    Feature::MutualRecursion,
    Feature::Globals,
    // ── SIR16 (v1) — added incrementally ───────────────────────────
    // `Floats` adds a `Value::Float(f64)` arm to the runtime value
    // model plus numeric promotion across the arithmetic helpers.
    // `ShortCircuit` is pure emit (a guarded block over the existing
    // `truthy` helper) and needs no runtime change.
    Feature::Floats,
    Feature::ShortCircuit,
    // `MutableBindings` lets a `LetBinding` be re-targeted by a later
    // `Assign`.  The emitter runs a per-function pre-pass to find every
    // reassigned name and declares those bindings `let mut` (immutable
    // bindings stay plain `let`); the `Assign` arm then emits a bare
    // `<name> = <value>;`.
    Feature::MutableBindings,
    // `Loops` covers `While`, `ForRange`, and `ForEach`.  All three emit
    // real Rust loops (see `emit.rs`); `ForEach` iterates a cons-list via
    // the runtime `seq_iter` helper (no `Sequences` runtime needed — the
    // existing `Pair` value model carries the list).  Because every loop
    // statement now emits without panic, accepting `Loops` keeps the
    // capability check and emit coverage consistent.
    Feature::Loops,
    // `Sequences` adds a shared, mutable `Value::Seq(Rc<RefCell<Vec<…>>>)`
    // to the runtime value model.  `SeqLit`/`SeqIndex`/`SeqLen` lower to
    // the `seq_lit`/`seq_index`/`seq_len` helpers; the `SeqSet` statement
    // mutates the backing vector through `seq_set`.  Because `Seq` is a
    // real value now, the A2 `ForEach`/`seq_iter` path was reconciled to
    // iterate it (snapshotting the elements) in addition to the legacy
    // cons-list — `for x in [1, 2, 3]` works end to end.
    Feature::Sequences,
    // `Maps` adds a shared, mutable, insertion-ordered
    // `Value::Map(Rc<RefCell<Vec<(Value, Value)>>>)`.  `MapLit`/`MapGet`
    // lower to `map_lit`/`map_get`; the `MapSet` statement mutates via
    // `map_set`.  Keys compare with the runtime's `value_eq` (linear
    // lookup), so any value type is a usable key and a missing-key
    // `MapGet` returns `Nil`.
    Feature::Maps,
    // With Sequences + Maps declared, all SIX SIR16 (v1) features —
    // Floats, ShortCircuit, MutableBindings, Loops, Sequences, Maps —
    // are accepted, and every SIR16 IR node now has a real emit arm.
    // The only remaining `panic!`s in `emit.rs` cover SIR17/18 nodes
    // (classes, modules, try/catch, str-concat, instance/class/const
    // vars, intrinsics) whose features stay unaccepted, so those arms
    // remain unreachable for any validated module.
    //
    // `DefaultParams` (P2e) lets a `Param` carry a `default` expression
    // that runs when the caller omits that trailing argument.  Rust fns
    // are fixed-arity over `__sir::Value` with no native defaults, so the
    // backend uses a RUNTIME-MIMIC strategy: a `Value::Missing` sentinel
    // marks an omitted argument, every defaulted param gets a body-top
    // prologue (`let p = if is_missing(&p) { <default> } else { p };`)
    // that evaluates the default *in the body* — where earlier params are
    // already bound, giving call-time, param-scope semantics — and each
    // `DirectCall` pads its omitted trailing positions with
    // `__sir::missing()` so the emitted call is full-arity.  This needs no
    // validator change (the core validator already permits omitting
    // trailing defaulted args in a `DirectCall`).
    Feature::DefaultParams,
    // `KeywordParams` (KW5) adds name-matched keyword parameters
    // (`def f(a:)` / `def f(a: 1)`) and keyword arguments (`f(a: 1)`).
    // Rust has NO native keyword-argument syntax, so — per spec §4 — the
    // backend performs STATIC keyword→positional resolution at emit time
    // (no runtime library):
    //
    //   • Def side: a `Keyword` param emits as an ORDINARY positional
    //     parameter in its declared order (the name simply becomes the
    //     Rust parameter name; the by-name affordance is dropped).  An
    //     OPTIONAL keyword (one carrying a `default`) reuses the very same
    //     `DefaultParams` body-top prologue — it is a defaulted parameter
    //     like any other — so no new def-side machinery is required.
    //
    //   • Call side: for a `DirectCall` whose callee signature is known
    //     (looked up in the module's functions, exactly as default-param
    //     padding consults the arity table), the backend builds the FULL
    //     positional argument list in the callee's DECLARED order:
    //       1. positional args fill positional params in order;
    //       2. each `KeywordArg { name, value }` fills the callee param
    //          whose name matches `name` (a name→position reorder);
    //       3. any omitted OPTIONAL keyword param is filled by emitting
    //          ITS default (via `Function::missing_keywords`), exactly as
    //          trailing positional defaults are filled today.
    //     The result is a plain positional Rust call `f(a, b_val, c_def)`.
    //
    // This is the same shape as the existing default-param call emission
    // (fill omitted params with their defaults) plus a name→position
    // reorder — so accepting the feature keeps the capability check and
    // emit coverage consistent.
    Feature::KeywordParams,
    // ── SIR17 (E4) — exceptions ────────────────────────────────────
    // `Exceptions` lowers `Stmt::TryCatch` to a `std::panic::catch_unwind`
    // region and `raise` to a `panic_any(SirError{…})`, with a runtime
    // `rescue_matches` over a built-in + user ancestry table (see
    // `runtime.rs`).  All emit is localized to the `raise`/`TryCatch`
    // arms; every other node is unchanged.
    Feature::Exceptions,
    // ── SIR17 (O5) — user-defined-class OOP ─────────────────────────
    // `Classes` is now accepted for REAL user-defined classes, not just
    // the narrow exception-subclass idiom.  The Ruby→SIR frontend (O2)
    // lowers OOP to a small family of builtins the backend routes to the
    // inlined `__sir` OOP runtime:
    //   • `Foo.new(args)`        → `__new__`     → `call_new` (runs an
    //                                              inherited `initialize`)
    //   • `def m` in `class C`   → `__def_method__` → `def_method`
    //   • `def self.m`           → `__def_class_method__` → `def_class_method`
    //   • `super(args)`          → `__super__`   → `call_super`
    //   • `self`                 → `__self__`    → `current_self`
    //   • `recv.m(args)`         → `__method__`  → `call_method` (a
    //       `Value::Instance` receiver resolves the USER method table
    //       walking ancestry; every other receiver keeps the unchanged
    //       collection/built-in path)
    // Method `def`s still HOIST to top-level `Function`s (referenced by the
    // `__def_*` builtins' `MakeClosure`), so a `ClassDef` body remains
    // EMPTY; a class carrying an executable statement in its body is still
    // rejected by `reject_stateful_class` (the backend emits methods from
    // the hoisted functions + registrations, never from the class body).
    // The `subclass → superclass` edge is registered into the SHARED
    // ancestry table at init (see `emit_ancestry_registration`), which the
    // OOP method resolver and the exception matcher both walk.  Dispatch is
    // an EXPLICIT `HashMap` lookup, never reflection (C3 RCE lesson).
    Feature::Classes,
    // `Modules` (Ruby `module Foo … end`) is accepted on the SAME footing
    // as `Classes`: the validator marks it for a namespace/mixin-opening
    // construct, and — like a class — the frontend hoists its method
    // `def`s to top-level functions with `__def_*` registrations, leaving
    // the `ModuleDef`/nested `ClassDef` body empty.  A `ModuleDef` with an
    // executable body reaches the `Stmt::ModuleDef` emit panic, so it is
    // rejected up front by `reject_stateful_class` (extended to cover
    // module bodies) — keeping this acceptance sound.
    Feature::Modules,
    // `InstanceVars` (`@x`) and `ClassVars` (`@@x`) are accepted: a read
    // lowers to `__sir::ivar_get`/`cvar_get` and a write to
    // `ivar_set`/`cvar_set`, both acting on the current-self instance (a
    // no-op returning the value / `Nil` outside any method).  These are the
    // storage half of real OOP — with them, `Dog.new("Rex").speak` reading
    // `@name` through method dispatch executes end to end.
    Feature::InstanceVars,
    Feature::ClassVars,
    // `Constants` is accepted because a `Scope::Const` VarRef names a class
    // in exactly the positions this backend LIFTS to a string literal — a
    // `raise MyErr` exception class, and the class-name slot of an OOP
    // builtin (`Dog.new` → `__new__(Dog, …)`, `super` → `__super__(m, C,
    // …)`).  In none of those does the emitter read a runtime constant.
    // Any OTHER `Const` reference (a genuine constant read/assign this
    // backend cannot lower) is rejected cleanly by `reject_const_ref`
    // below, keeping this acceptance sound.
    Feature::Constants,
    // `Feature::ConsoleIO` (SIR28) — `__sys_write__`, the reserved
    // console-output primitive `print`/`puts` will migrate to. Additive:
    // nothing emits it yet, so this declares acceptance ahead of any
    // frontend using it.
    Feature::ConsoleIO,
    // ── SIR22 array/matrix domain (second-wave backend rollout) ──────
    // `ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet`
    // (+ `Stmt::IndexSet`) — the "base cut" (Phase A Slice 2) — and the
    // 9-node "APL addendum" (`Reduce`/`Scan`/`OuterProduct`/`Shape`/
    // `Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`, Phase A
    // Slice 3) route into `__sir::array_*` helpers, an inlined port of
    // the already-proven `semantic-ir-to-javascript` `ArrayRt` sub-
    // runtime (see `runtime.rs`'s "SIR22 array/matrix domain" and "SIR22
    // addendum: APL primitive operators" sections) — this backend's
    // existing "self-contained, no external crate" convention, same as
    // `seq_*`/`map_*`. All 16 SIR22 node kinds now share the same three
    // features below with no need for a dedicated pre-emit scan to tell
    // any subset apart — Slice 2's `reject_sir22_addendum` (which used
    // to reject the addendum nine) was removed once Slice 3 gave them
    // real codegen.
    Feature::NDArrays,
    Feature::MatrixOps,
    Feature::ArrayColumnMajor,
    // ── SIR23 symbolic-expression/pattern-matcher domain, Tier A (Phase A
    // Slice 4) ─────────────────────────────────────────────────────────
    // `SymSymbol`/`SymRational`/`SymApply`/`SymPatternBlank`/
    // `SymPatternNamed`/`SymRule`/`SymReplaceAll` route into
    // `__sir::sir_sym_*` helpers, an inlined port of the already-proven
    // `semantic-ir-to-javascript` `Symbolic` sub-runtime's Tier A (matcher)
    // slice — see `runtime.rs`'s "SIR23 symbolic expressions" section for
    // the full value-model / DoS-guard rationale, and `emit.rs`'s SIR23
    // arms for the lowering. Tier A ONLY: no `evalTerm`-equivalent
    // arithmetic/calculus/user-function-dispatch evaluator exists — a
    // `SymApply` builds an inert term tree, nothing more (Tier B is
    // explicitly out of scope for this slice, matching the SIR23 spec's
    // own split).
    //
    // `Rationals` (a `SirType::Rational`) is NOT a new flag introduced by
    // this slice — it already exists in `semantic_ir::Feature`, shared with
    // the SIR22 array/matrix domain rather than owned by SIR23, matching
    // the JS reference's own comment on the same point. `SymRational`
    // observes it (see `semantic-ir`'s `validator.rs`), so it must be
    // declared here too for a module using `SymRational` to validate.
    Feature::SymbolicExpr,
    Feature::PatternMatching,
    Feature::Rationals,
];

impl Backend for RustBackend {
    fn target_tag(&self) -> &'static str {
        "rust"
    }

    fn accepts_features(&self) -> &'static [Feature] {
        ACCEPTED_FEATURES
    }

    fn accepts_intrinsics(&self) -> &'static [&'static str] {
        &[]
    }

    fn compile(&self, module: &Module) -> Result<Artifact, BackendError> {
        // 1. Validate.  Collapse the "not ok" branch to a direct
        //    `if let Some(e) = ...` so a non-ok result with no
        //    error-severity issue (in theory: warnings only, though
        //    today `is_ok()` already maps that case to true) cannot
        //    silently bypass the guard.
        let r = semantic_ir::validate(module);
        if let Some(e) = r.errors().next().cloned() {
            return Err(BackendError {
                kind: BackendErrorKind::InvalidModule,
                message: format!("module failed validation: {}", e.message),
                span: e.span,
            });
        }

        // 2. Capability checks.
        let cap_errors = self.check_module(module);
        if let Some(e) = cap_errors.into_iter().next() {
            return Err(e);
        }

        // 2b. Structural capability check the manifest cannot express:
        //     reject any function that mixes a keyword parameter with a
        //     `*rest`/`**kwrest` variadic slot.  Static keyword→positional
        //     resolution (this backend's whole keyword strategy) requires
        //     FIXED arity — see `reject_keyword_with_variadic`.
        if let Some(e) = reject_keyword_with_variadic(module) {
            return Err(e);
        }

        // 2c. Exception-feature soundness gates (E4).  `Feature::Classes`
        //     and `Feature::Constants` are accepted ONLY for the narrow
        //     exception use case (an exception-subclass declaration and a
        //     `raise MyErr` class name), so reject anything broader that
        //     the emitter genuinely cannot lower — BEFORE emit, so those
        //     emit paths are true internal-bug guards, not DoS surfaces.
        if let Some(e) = reject_stateful_class(module) {
            return Err(e);
        }
        if let Some(e) = reject_const_ref(module) {
            return Err(e);
        }

        // 3. TailCalls cannot be satisfied.
        if module.manifest.contains(Feature::TailCalls) {
            return Err(BackendError {
                kind: BackendErrorKind::UnsupportedFeature,
                message: "rust backend cannot satisfy `tail_calls` feature".into(),
                span: module.span.clone(),
            });
        }

        // 4. Emit.
        let source = emit::emit_module_with_arity_table(module);
        let metadata = ArtifactMetadata {
            bytes: source.len(),
            line_count: source.lines().count(),
            ..Default::default()
        };

        Ok(Artifact {
            filename: format!("{}.rs", module.name.replace('/', "_")),
            source,
            metadata,
        })
    }
}

/// Reject a module whose any function mixes a keyword parameter with a
/// `*rest`/`**kwrest` variadic parameter.
///
/// ## Why keyword + `*rest`/`**kwrest` cannot be statically resolved
///
/// This backend implements keyword parameters with **static**
/// keyword→positional resolution at emit time (see the `KeywordParams`
/// note in `ACCEPTED_FEATURES` and `emit.rs`): for each `DirectCall`, the
/// emitter walks the callee's parameter list and routes every keyword
/// argument to the Rust argument slot of the same-named parameter,
/// producing a plain positional Rust call `f(a, b, c)`.  That
/// name→position map is only well-defined because the arity is FIXED —
/// parameter *i* always lands at Rust argument slot *i*.
///
/// The core validator's M3 ordering rule, however, PERMITS a signature
/// that mixes a keyword parameter with a variadic slot — e.g. Ruby
/// `def f(a, *rest, x: 1)` (the legal order is
/// `Required* Rest? Keyword* KwRest?`).  A `Rest` (`*rest`) or `KwRest`
/// (`**opts`) parameter absorbs a *variable* number of arguments, so the
/// slot index of every parameter after it depends on how many arguments
/// the caller actually passed.  The name→position map is then no longer a
/// function of the signature alone — there is no fixed Rust argument slot
/// to route a keyword into, so static resolution genuinely cannot be
/// performed.
///
/// A backend carrying a runtime keyword-dispatch library could handle
/// this; this backend deliberately has none.  It therefore rejects such a
/// module cleanly HERE, rather than letting the emitter reach the
/// `ParamKind::Rest | ParamKind::KwRest` arm in the keyword-resolution
/// path and panic.  That panic was reachable on validator-accepted input
/// (a DoS) before this check existed; with the check in place the emit
/// arm is a true internal-bug guard.
///
/// Returns `Some(err)` for the FIRST offending function (fail-fast, like
/// the other capability checks), or `None` if the module is clean.
fn reject_keyword_with_variadic(module: &Module) -> Option<BackendError> {
    for func in &module.functions {
        let has_keyword = func.params.iter().any(|p| p.kind == ParamKind::Keyword);
        let has_variadic = func
            .params
            .iter()
            .any(|p| matches!(p.kind, ParamKind::Rest | ParamKind::KwRest));
        if has_keyword && has_variadic {
            return Some(BackendError {
                kind: BackendErrorKind::UnsupportedFeature,
                message: "rust backend cannot emit a function mixing keyword \
                          parameters with *rest/**kwrest (static keyword \
                          resolution requires fixed arity)"
                    .into(),
                span: func.span.clone(),
            });
        }
    }
    None
}

/// Reject a `Stmt::ClassDef` whose body is **non-empty** (E4 soundness).
///
/// `Feature::Classes` is accepted only for the exception-subclass idiom
/// `class MyErr < StandardError; end`, whose body is empty because the Ruby
/// frontend hoists method `def`s to top-level `Function`s.  A NON-empty body
/// carries executable class state (constant / class-variable assigns, or any
/// other statement) that this backend has no object model to emit — so we
/// reject it cleanly HERE rather than letting emit produce nonsense (or reach
/// a panic for a nested unsupported node).  This keeps the ClassDef emit arm a
/// pure ancestry-metadata path.
///
/// Returns `Some(err)` for the FIRST offending class (fail-fast), else `None`.
fn reject_stateful_class(module: &Module) -> Option<BackendError> {
    for func in &module.functions {
        if let Some(e) = stateful_class_in_stmts(&func.body.stmts) {
            return Some(e);
        }
    }
    None
}

fn stateful_class_in_stmts(stmts: &[Stmt]) -> Option<BackendError> {
    for s in stmts {
        match s {
            Stmt::ClassDef {
                name, body, span, ..
            } => {
                if !body.is_empty() {
                    return Some(BackendError {
                        kind: BackendErrorKind::UnsupportedFeature,
                        message: format!(
                            "rust backend accepts only empty-body (exception-subclass) \
                             class declarations; class `{name}` has a non-empty body \
                             (class state / methods are out of scope for this backend)"
                        ),
                        span: span.clone(),
                    });
                }
            }
            // A `module Foo … end` is accepted on the SAME footing as a
            // class: its method `def`s hoist to top-level functions with
            // `__def_*` registrations, so an accepted `ModuleDef` body is
            // EMPTY.  A non-empty body carries executable state the backend
            // has no object model for (and would reach the `Stmt::ModuleDef`
            // emit path), so reject it cleanly HERE — mirroring the ClassDef
            // rule — rather than letting emit produce nonsense.
            Stmt::ModuleDef {
                name, body, span, ..
            } => {
                if !body.is_empty() {
                    return Some(BackendError {
                        kind: BackendErrorKind::UnsupportedFeature,
                        message: format!(
                            "rust backend accepts only empty-body module \
                             declarations (methods hoist to top-level functions); \
                             module `{name}` has a non-empty body"
                        ),
                        span: span.clone(),
                    });
                }
            }
            // A `class << self`/singleton-class body likewise recurses so a
            // stateful class nested inside is still caught.
            Stmt::SingletonClassDef { body, .. } => {
                if let Some(e) = stateful_class_in_stmts(body) {
                    return Some(e);
                }
            }
            Stmt::TryCatch {
                body,
                rescues,
                ensure_body,
                ..
            } => {
                if let Some(e) = stateful_class_in_stmts(body) {
                    return Some(e);
                }
                for r in rescues {
                    if let Some(e) = stateful_class_in_stmts(&r.body) {
                        return Some(e);
                    }
                }
                if let Some(ens) = ensure_body {
                    if let Some(e) = stateful_class_in_stmts(ens) {
                        return Some(e);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Reject any `Scope::Const` reference the backend cannot lower (E4
/// soundness).
///
/// `Feature::Constants` is accepted only because `raise MyErr` names its
/// exception class through a `Scope::Const` VarRef, which the `raise` emit
/// arm LIFTS to a string literal (never a runtime constant read).  Every
/// OTHER `Const` reference — a `Const` VarRef that is not a raise class name,
/// or an `Assign`/`ExprStmt` to a `Const` — has no backend lowering and would
/// otherwise reach the `emit_var_ref` `Scope::Const` panic on validated input
/// (a DoS).  We reject such modules cleanly here.
///
/// The ALLOWED shape is exactly `BuiltinCall("raise", [VarRef{Const}, …])`
/// with the Const as the first argument; we walk every expression and flag a
/// `Const` VarRef found in any other position.
///
/// Returns `Some(err)` for the FIRST offending reference, else `None`.
fn reject_const_ref(module: &Module) -> Option<BackendError> {
    for func in &module.functions {
        if let Some(e) = const_ref_in_stmts(&func.body.stmts) {
            return Some(e);
        }
        if let Some(e) = const_ref_in_expr(&func.body.value) {
            return Some(e);
        }
    }
    None
}

fn const_ref_in_stmts(stmts: &[Stmt]) -> Option<BackendError> {
    for s in stmts {
        if let Some(e) = const_ref_in_stmt(s) {
            return Some(e);
        }
    }
    None
}

fn const_ref_in_stmt(s: &Stmt) -> Option<BackendError> {
    match s {
        Stmt::LetBinding { value, .. }
        | Stmt::LetStarBinding { value, .. }
        | Stmt::ExprStmt { expr: value, .. } => const_ref_in_expr(value),
        Stmt::Assign {
            scope: Scope::Const,
            span,
            ..
        } => Some(unsupported_const(span.clone())),
        Stmt::Assign { value, .. } => const_ref_in_expr(value),
        Stmt::While { cond, body, .. } => {
            const_ref_in_expr(cond).or_else(|| const_ref_in_block(body))
        }
        Stmt::ForRange {
            start,
            stop,
            step,
            body,
            ..
        } => const_ref_in_expr(start)
            .or_else(|| const_ref_in_expr(stop))
            .or_else(|| const_ref_in_expr(step))
            .or_else(|| const_ref_in_block(body)),
        Stmt::ForEach { iter, body, .. } => {
            const_ref_in_expr(iter).or_else(|| const_ref_in_block(body))
        }
        Stmt::SeqSet {
            seq, index, value, ..
        } => const_ref_in_expr(seq)
            .or_else(|| const_ref_in_expr(index))
            .or_else(|| const_ref_in_expr(value)),
        Stmt::MapSet {
            map, key, value, ..
        } => const_ref_in_expr(map)
            .or_else(|| const_ref_in_expr(key))
            .or_else(|| const_ref_in_expr(value)),
        Stmt::ClassDef { body, .. }
        | Stmt::ModuleDef { body, .. }
        | Stmt::SingletonClassDef { body, .. } => const_ref_in_stmts(body),
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
            if let Some(e) = const_ref_in_stmts(body) {
                return Some(e);
            }
            for r in rescues {
                if let Some(e) = const_ref_in_stmts(&r.body) {
                    return Some(e);
                }
            }
            if let Some(ens) = ensure_body {
                if let Some(e) = const_ref_in_stmts(ens) {
                    return Some(e);
                }
            }
            None
        }
        // SIR22 array/matrix indexed assignment: `Feature::NDArrays` /
        // `Feature::MatrixOps` are not in this backend's accepted-features
        // list, so `check_module` rejects any module using `IndexSet`
        // before this analysis ever runs. Still scan `target`, each index
        // sub-expression, and `value` faithfully — same recursive style as
        // the `SeqSet`/`MapSet` arms above (this function never takes a
        // "rejected elsewhere" shortcut).
        Stmt::IndexSet {
            target,
            indices,
            value,
            ..
        } => {
            if let Some(e) = const_ref_in_expr(target) {
                return Some(e);
            }
            for idx in indices {
                let inner = match idx {
                    IndexArg::Scalar(inner) | IndexArg::Range(inner) => Some(inner.as_ref()),
                    IndexArg::Whole => None,
                };
                if let Some(inner) = inner {
                    if let Some(e) = const_ref_in_expr(inner) {
                        return Some(e);
                    }
                }
            }
            const_ref_in_expr(value)
        }
    }
}

fn const_ref_in_block(b: &semantic_ir::Block) -> Option<BackendError> {
    const_ref_in_stmts(&b.stmts).or_else(|| const_ref_in_expr(&b.value))
}

fn const_ref_in_expr(e: &Expr) -> Option<BackendError> {
    match e {
        // A `Const` VarRef standing alone (not a raise class name) cannot be
        // lowered — flag it.
        Expr::VarRef {
            scope: Scope::Const,
            span,
            ..
        } => Some(unsupported_const(span.clone())),
        // `raise` is one allowed home for a `Const`: its first argument may
        // be a `Const` class name (lifted to a string).  Skip that slot;
        // still scan the remaining arguments (the message expression, etc.).
        Expr::BuiltinCall { name, args, .. } if name == "raise" => {
            let skip_first = matches!(
                args.first(),
                Some(Expr::VarRef {
                    scope: Scope::Const,
                    ..
                })
            );
            let start = if skip_first { 1 } else { 0 };
            for a in &args[start.min(args.len())..] {
                if let Some(err) = const_ref_in_expr(a) {
                    return Some(err);
                }
            }
            None
        }
        // ── OOP builtins: class/method NAME slots may be `Const` ────────
        // `__new__("Klass", …)` / `__super__("m", "Klass", …)` carry a class
        // name that the Ruby frontend may lower as a `Const` VarRef
        // (`Dog.new`).  The emitter LIFTS that `Const` to a `&str` literal
        // (see `emit_oop_name_arg`) — never a runtime constant read — so it
        // is sound to skip the NAME slots here while still scanning the
        // ordinary call ARGS.  `__new__`'s name is arg[0]; `__super__`'s are
        // arg[0] (method) and arg[1] (class).  (`__def_method__` /
        // `__def_class_method__` name slots are `StrLit`, never `Const`, so
        // they need no skip — but scanning their closure arg is still
        // correct: a `Const` hidden in a method-body capture is flagged.)
        Expr::BuiltinCall { name, args, .. } if name == "__new__" => {
            // Skip the class-name slot (arg[0]); scan the rest.
            for a in args.iter().skip(1) {
                if let Some(err) = const_ref_in_expr(a) {
                    return Some(err);
                }
            }
            None
        }
        Expr::BuiltinCall { name, args, .. } if name == "__super__" => {
            // Skip the method-name (arg[0]) and class-name (arg[1]) slots;
            // scan the call args from arg[2].
            for a in args.iter().skip(2) {
                if let Some(err) = const_ref_in_expr(a) {
                    return Some(err);
                }
            }
            None
        }
        // MX6 mixins — `Owner.method(args…)` lowers to
        // `__class_method__(Owner, "method", args…)`, and the OWNER may be a
        // bare-constant class written as a `Const` VarRef (`Registry.total`).
        // The emitter LIFTS that `Const` to a `&str` literal (via
        // `emit_oop_name_arg`) — never a runtime constant read — so it is
        // sound to skip the owner (arg[0]) and method-name (arg[1]) slots
        // while still scanning the ordinary call ARGS from arg[2].
        // (`__include__`/`__extend__` name slots are `StrLit`, never `Const`,
        // so they need no skip — but scanning their args as ordinary
        // `BuiltinCall` args below is harmless: they carry only name slots.)
        Expr::BuiltinCall { name, args, .. } if name == "__class_method__" => {
            for a in args.iter().skip(2) {
                if let Some(err) = const_ref_in_expr(a) {
                    return Some(err);
                }
            }
            None
        }
        Expr::BuiltinCall { args, .. } | Expr::DirectCall { args, .. } => {
            for a in args {
                if let Some(err) = const_ref_in_expr(a) {
                    return Some(err);
                }
            }
            None
        }
        Expr::IndirectCall { target, args, .. } => {
            if let Some(err) = const_ref_in_expr(target) {
                return Some(err);
            }
            for a in args {
                if let Some(err) = const_ref_in_expr(a) {
                    return Some(err);
                }
            }
            None
        }
        Expr::Intrinsic { args, .. } => {
            for a in args {
                if let Some(err) = const_ref_in_expr(a) {
                    return Some(err);
                }
            }
            None
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => const_ref_in_expr(cond)
            .or_else(|| const_ref_in_block(then_branch))
            .or_else(|| const_ref_in_block(else_branch)),
        Expr::Block(b) => const_ref_in_block(b),
        Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
            const_ref_in_expr(lhs).or_else(|| const_ref_in_expr(rhs))
        }
        Expr::KeywordArg { value, .. } => const_ref_in_expr(value),
        Expr::SeqLit { items, .. } | Expr::StrConcat { parts: items, .. } => {
            for it in items {
                if let Some(err) = const_ref_in_expr(it) {
                    return Some(err);
                }
            }
            None
        }
        Expr::SeqIndex { seq, index, .. } => {
            const_ref_in_expr(seq).or_else(|| const_ref_in_expr(index))
        }
        Expr::SeqLen { seq, .. } => const_ref_in_expr(seq),
        Expr::MapLit { entries, .. } => {
            for entry in entries {
                if let Some(err) = const_ref_in_expr(&entry.key) {
                    return Some(err);
                }
                if let Some(err) = const_ref_in_expr(&entry.value) {
                    return Some(err);
                }
            }
            None
        }
        Expr::MapGet { map, key, .. } => const_ref_in_expr(map).or_else(|| const_ref_in_expr(key)),
        // A closure's captured values are ordinary expressions emitted at the
        // capture site — a `Const` VarRef hiding in a capture would otherwise
        // slip past this gate and reach the `Scope::Const` emit panic (a
        // reachable backend DoS on a validated module).  Walk each capture.
        Expr::MakeClosure { captures, .. } => {
            for c in captures {
                if let Some(err) = const_ref_in_expr(&c.value) {
                    return Some(err);
                }
            }
            None
        }
        // Leaf / already-supported expressions carry no nested Const.
        _ => None,
    }
}

fn unsupported_const(span: semantic_ir::Span) -> BackendError {
    BackendError {
        kind: BackendErrorKind::UnsupportedFeature,
        message: "rust backend cannot lower a constant reference; the only \
                  accepted `Const` is an exception class name in `raise Foo` \
                  (lifted to a string literal)"
            .into(),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semantic_ir::{
        Block, EffectSet, Expr, FeatureManifest, Function, Metadata, Param, RescueClause, Scope,
        Span, Stmt,
    };

    fn s() -> Span {
        Span::synthetic()
    }

    fn minimal_module() -> Module {
        Module {
            name: "demo".into(),
            manifest: FeatureManifest::new(),
            imports: vec![],
            exports: vec![],
            functions: vec![Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body: Block {
                    stmts: vec![],
                    value: Expr::IntLit {
                        value: 42,
                        span: s(),
                    },
                    span: s(),
                },
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            }],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("twig")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        }
    }

    #[test]
    fn compiles_minimal_module() {
        let m = minimal_module();
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("fn __sir_user_main()"));
        assert!(a.source.contains("__sir::Value::Int(42i64)"));
        assert!(a.filename.ends_with(".rs"));
        assert!(a.metadata.bytes > 0);
    }

    #[test]
    fn target_tag_is_rust() {
        let b = RustBackend::new();
        assert_eq!(b.target_tag(), "rust");
    }

    #[test]
    fn rejects_tail_calls() {
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[Feature::TailCalls]);
        let err = compile(&m).expect_err("tail calls rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
    }

    #[test]
    fn end_to_end_twig_to_rust_id_function() {
        let module = twig_to_semantic_ir::compile_source("(define (id x) x)\n(id 42)", "demo")
            .expect("lower");
        let a = compile(&module).expect("compile");
        assert!(a.source.contains("fn id(x: __sir::Value) -> __sir::Value"));
        assert!(a.source.contains("x.clone()"));
        assert!(a.source.contains("id(__sir::Value::Int(42i64))"));
    }

    #[test]
    fn end_to_end_twig_to_rust_arith() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (add a b) (+ a b))\n(print (add 1 2))",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        assert!(a.source.contains("fn add("));
        assert!(a.source.contains("__sir::plus(vec!["));
        // SIR28 §2: `print` lowers to `__sys_write__`, which this backend
        // maps to `__sir::write(...)`.
        assert!(a.source.contains("__sir::write(\"stdout\", \"none\", false, vec![add("));
    }

    #[test]
    fn end_to_end_twig_to_rust_closure_program() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (adder n) (lambda (x) (+ x n)))\n(define add5 (adder 5))\n(print (add5 3))",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        assert!(a.source.contains("fn __lambda_0("));
        assert!(a
            .source
            .contains("__sir::Value::Closure(::std::rc::Rc::new"));
        assert!(a.source.contains("__sir::apply_closure"));
        assert!(a.source.contains("Globals (initialised in _init): add5"));
    }

    #[test]
    fn output_is_deterministic() {
        let module = twig_to_semantic_ir::compile_source("(define (id x) x)\n(id 7)", "demo")
            .expect("lower");
        let a = compile(&module).expect("compile");
        let b = compile(&module).expect("compile again");
        assert_eq!(a.source, b.source);
    }

    #[test]
    fn module_filename_sanitised() {
        let mut m = minimal_module();
        m.name = "compiler/lexer".into();
        let a = compile(&m).expect("compile");
        assert_eq!(a.filename, "compiler_lexer.rs");
    }

    // ── Keyword + *rest/**kwrest rejection (hardening) ─────────────────
    //
    // The core validator's M3 ordering rule ACCEPTS a signature that mixes
    // a keyword param with a variadic slot (`Required* Rest? Keyword*
    // KwRest?`), e.g. Ruby `def f(a, *rest, x: 1)`.  This backend accepts
    // `Feature::KeywordParams`, so such a module would formerly reach the
    // emitter's keyword-resolution path and hit the
    // `ParamKind::Rest | ParamKind::KwRest` panic — a reachable panic on
    // validator-accepted input (a DoS).  `reject_keyword_with_variadic`
    // now rejects these modules at capability-check time; these tests pin
    // that (a) a keyword+rest callee with a keyword call is rejected via
    // `compile()` WITHOUT panicking, and (b) a keyword+kwrest callee is
    // likewise rejected.

    /// Build `def f(a, *rest, name: <default>)`: a required positional,
    /// then a `*rest`, then an OPTIONAL keyword — the exact shape the core
    /// validator accepts but this backend cannot statically resolve.
    fn kw_rest_fn() -> Function {
        Function {
            name: "f".into(),
            params: vec![
                Param {
                    name: "a".into(),
                    kind: ParamKind::Required,
                    sir_type: None,
                    default: None,
                    span: s(),
                },
                Param {
                    name: "rest".into(),
                    kind: ParamKind::Rest,
                    sir_type: None,
                    default: None,
                    span: s(),
                },
                Param {
                    name: "name".into(),
                    kind: ParamKind::Keyword,
                    sir_type: None,
                    default: Some(Box::new(Expr::StrLit {
                        value: "world".into(),
                        span: s(),
                    })),
                    span: s(),
                },
            ],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "name".into(),
                    scope: Scope::Param,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }
    }

    /// A `main` that calls `f("hi", name: "ada")` — a keyword call whose
    /// static resolution would drive the emitter into the variadic panic
    /// arm if the module were not rejected first.
    fn main_calling_f_by_keyword() -> Function {
        Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::DirectCall {
                    fn_name: "f".into(),
                    args: vec![
                        Expr::StrLit {
                            value: "hi".into(),
                            span: s(),
                        },
                        Expr::KeywordArg {
                            name: "name".into(),
                            value: Box::new(Expr::StrLit {
                                value: "ada".into(),
                                span: s(),
                            }),
                            span: s(),
                        },
                    ],
                    effects: EffectSet::PURE,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }
    }

    fn kw_module(functions: Vec<Function>) -> Module {
        Module {
            name: "demo".into(),
            // Observed features for these functions: DynamicTyping (untyped
            // params), KeywordParams (the keyword param), and Strings (the
            // string-literal defaults / args).  A `*rest` param observes no
            // feature of its own, so declaring exactly these keeps the
            // validator's manifest comparison happy — the module is
            // VALIDATOR-ACCEPTED, which is the whole point: the panic was
            // reachable on validated input.
            manifest: FeatureManifest::from_features(&[
                Feature::DynamicTyping,
                Feature::KeywordParams,
                Feature::Strings,
            ]),
            imports: vec![],
            exports: vec![],
            functions,
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("ruby")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        }
    }

    #[test]
    fn rejects_keyword_mixed_with_rest_without_panicking() {
        let m = kw_module(vec![kw_rest_fn(), main_calling_f_by_keyword()]);
        // `compile()` runs the capability check BEFORE emit, so this must
        // return Err — never panic in the emitter's variadic arm.
        let err = compile(&m).expect_err("keyword + *rest must be rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
        assert!(
            err.message.contains("keyword")
                && err.message.contains("*rest/**kwrest")
                && err.message.contains("fixed arity"),
            "unexpected rejection message: {}",
            err.message
        );
    }

    #[test]
    fn rejects_keyword_mixed_with_kwrest_without_panicking() {
        // Same shape but a `**kwrest` (KwRest) variadic instead of `*rest`.
        let mut f = kw_rest_fn();
        // Reorder to the M3-legal `Required* Keyword* KwRest?`: keyword
        // BEFORE the KwRest slot (a `**opts` must come last).
        f.params = vec![
            Param {
                name: "a".into(),
                kind: ParamKind::Required,
                sir_type: None,
                default: None,
                span: s(),
            },
            Param {
                name: "name".into(),
                kind: ParamKind::Keyword,
                sir_type: None,
                default: Some(Box::new(Expr::StrLit {
                    value: "world".into(),
                    span: s(),
                })),
                span: s(),
            },
            Param {
                name: "opts".into(),
                kind: ParamKind::KwRest,
                sir_type: None,
                default: None,
                span: s(),
            },
        ];
        let m = kw_module(vec![f, main_calling_f_by_keyword()]);
        let err = compile(&m).expect_err("keyword + **kwrest must be rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
        assert!(err.message.contains("*rest/**kwrest"));
    }

    #[test]
    fn keyword_without_variadic_still_compiles() {
        // Regression guard: the happy path (keyword params WITHOUT any
        // rest/kwrest) must still emit — the rejection is narrow.
        let f = Function {
            name: "greet".into(),
            params: vec![
                Param {
                    name: "greeting".into(),
                    kind: ParamKind::Required,
                    sir_type: None,
                    default: None,
                    span: s(),
                },
                Param {
                    name: "name".into(),
                    kind: ParamKind::Keyword,
                    sir_type: None,
                    default: Some(Box::new(Expr::StrLit {
                        value: "world".into(),
                        span: s(),
                    })),
                    span: s(),
                },
            ],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "name".into(),
                    scope: Scope::Param,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let main = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::DirectCall {
                    fn_name: "greet".into(),
                    args: vec![Expr::StrLit {
                        value: "hi".into(),
                        span: s(),
                    }],
                    effects: EffectSet::PURE,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let m = kw_module(vec![f, main]);
        let a = compile(&m).expect("keyword-only module should still compile");
        assert!(a.source.contains("fn greet("));
    }

    // ── E4: exception feature acceptance + soundness gates ─────────

    fn exc_module(stmts: Vec<Stmt>, features: &[Feature]) -> Module {
        let mut m = minimal_module();
        m.functions[0].body = Block {
            stmts,
            value: Expr::NilLit { span: s() },
            span: s(),
        };
        let mut feats = vec![Feature::Exceptions, Feature::Strings];
        feats.extend_from_slice(features);
        m.manifest = FeatureManifest::from_features(&feats);
        m
    }

    fn raise_stmt(cls: &str) -> Stmt {
        Stmt::ExprStmt {
            expr: Expr::BuiltinCall {
                name: "raise".into(),
                args: vec![
                    Expr::VarRef {
                        name: cls.into(),
                        scope: Scope::Const,
                        span: s(),
                    },
                    Expr::StrLit {
                        value: "m".into(),
                        span: s(),
                    },
                ],
                effects: EffectSet::PURE,
                span: s(),
            },
            span: s(),
        }
    }

    #[test]
    fn accepts_exceptions_and_emits_try_catch() {
        let tc = Stmt::TryCatch {
            body: vec![raise_stmt("ArgumentError")],
            rescues: vec![RescueClause {
                exception_types: vec!["StandardError".into()],
                binding: Some("e".into()),
                body: vec![],
                span: s(),
            }],
            ensure_body: None,
            span: s(),
        };
        let a = compile(&exc_module(vec![tc], &[Feature::Constants])).expect("exceptions accepted");
        assert!(
            a.source.contains("std::panic::catch_unwind"),
            "got:\n{}",
            a.source
        );
        assert!(
            a.source.contains(r#"__sir::raise("ArgumentError", "#),
            "got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("__sir::install_panic_hook();"),
            "got:\n{}",
            a.source
        );
    }

    #[test]
    fn accepts_exception_subclass_class_def_and_registers_ancestry() {
        let cd = Stmt::ClassDef {
            name: "MyErr".into(),
            superclass: Some("StandardError".into()),
            body: vec![],
            span: s(),
        };
        let a = compile(&exc_module(vec![cd], &[Feature::Classes]))
            .expect("empty-body exception subclass accepted");
        assert!(
            a.source
                .contains(r#"__sir::register_ancestry(&[("MyErr", "StandardError")]);"#),
            "got:\n{}",
            a.source
        );
    }

    #[test]
    fn rejects_stateful_class_body() {
        // A class whose body carries an executable statement is out of scope.
        let cd = Stmt::ClassDef {
            name: "Widget".into(),
            superclass: None,
            body: vec![Stmt::ExprStmt {
                expr: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        };
        let err = compile(&exc_module(vec![cd], &[Feature::Classes]))
            .expect_err("stateful class rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
        assert!(
            err.message.contains("non-empty body"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn rejects_non_raise_const_reference() {
        // A `Const` VarRef that is NOT a raise class name has no lowering.
        let stmt = Stmt::LetBinding {
            name: "x".into(),
            sir_type: None,
            value: Expr::VarRef {
                name: "PI".into(),
                scope: Scope::Const,
                span: s(),
            },
            span: s(),
        };
        let err = compile(&exc_module(vec![stmt], &[Feature::Constants]))
            .expect_err("bare const ref rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
        assert!(
            err.message.contains("constant reference"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn rejects_const_reference_hidden_in_closure_capture() {
        // Regression: a `Const` VarRef buried in a `MakeClosure` capture must be
        // rejected by the capability gate, NOT reach the `Scope::Const` emit
        // panic (a reachable backend DoS on a validated, feature-consistent
        // module — `Closures` + `Constants` are both accepted).
        let stmt = Stmt::LetBinding {
            name: "f".into(),
            sir_type: None,
            value: Expr::MakeClosure {
                fn_name: "lam".into(),
                captures: vec![semantic_ir::CaptureValue {
                    name: "c".into(),
                    value: Expr::VarRef {
                        name: "PI".into(),
                        scope: Scope::Const,
                        span: s(),
                    },
                }],
                span: s(),
            },
            span: s(),
        };
        // A real `lam` target so the module VALIDATES (else it fails earlier as
        // InvalidModule and never reaches the backend const-ref gate).
        let mut m = exc_module(vec![stmt], &[Feature::Constants, Feature::Closures]);
        m.functions.push(Function {
            name: "lam".into(),
            params: vec![],
            return_type: None,
            captures: vec![semantic_ir::Capture {
                name: "c".into(),
                sir_type: None,
            }],
            body: Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let err =
            compile(&m).expect_err("const ref in closure capture must be rejected, not panic");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
        assert!(
            err.message.contains("constant reference"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn allows_const_only_as_raise_class_name() {
        // `raise Foo, "m"` — the Const is the class name → allowed.
        let a = compile(&exc_module(vec![raise_stmt("Foo")], &[Feature::Constants]))
            .expect("raise-class-name const is allowed");
        assert!(
            a.source.contains(r#"__sir::raise("Foo", "#),
            "got:\n{}",
            a.source
        );
    }

    // ── O5: user-defined-class OOP acceptance + soundness ──────────────

    fn feat_module(stmts: Vec<Stmt>, features: &[Feature]) -> Module {
        let mut m = minimal_module();
        m.functions[0].body = Block {
            stmts,
            value: Expr::NilLit { span: s() },
            span: s(),
        };
        m.manifest = FeatureManifest::from_features(features);
        m
    }

    #[test]
    fn accepts_real_oop_module_and_emits_runtime_calls() {
        // A real OOP module: `class Dog`, a `__def_method__`, a `Dog.new`,
        // and an `@ivar` write — all now ACCEPTED and routed to the runtime.
        let stmts = vec![
            Stmt::ClassDef {
                name: "Dog".into(),
                superclass: None,
                body: vec![],
                span: s(),
            },
            Stmt::ExprStmt {
                expr: Expr::BuiltinCall {
                    name: "__def_method__".into(),
                    args: vec![
                        Expr::StrLit {
                            value: "Dog".into(),
                            span: s(),
                        },
                        Expr::StrLit {
                            value: "speak".into(),
                            span: s(),
                        },
                        Expr::MakeClosure {
                            fn_name: "Dog_speak".into(),
                            captures: vec![],
                            span: s(),
                        },
                    ],
                    effects: EffectSet::PURE,
                    span: s(),
                },
                span: s(),
            },
            Stmt::Assign {
                name: "@name".into(),
                scope: Scope::Instance,
                value: Expr::StrLit {
                    value: "Rex".into(),
                    span: s(),
                },
                span: s(),
            },
            Stmt::ExprStmt {
                expr: Expr::BuiltinCall {
                    name: "__new__".into(),
                    args: vec![Expr::StrLit {
                        value: "Dog".into(),
                        span: s(),
                    }],
                    effects: EffectSet::PURE,
                    span: s(),
                },
                span: s(),
            },
        ];
        // A real `Dog_speak` so the module validates.
        let mut m = feat_module(
            stmts,
            &[
                Feature::Classes,
                Feature::InstanceVars,
                Feature::Closures,
                Feature::Strings,
                Feature::MutableBindings,
            ],
        );
        m.functions.push(Function {
            name: "Dog_speak".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "@name".into(),
                    scope: Scope::Instance,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let a = compile(&m).expect("real OOP module should be accepted");
        assert!(
            a.source.contains(r#"__sir::def_method("Dog", "speak", "#),
            "got:\n{}",
            a.source
        );
        assert!(
            a.source.contains(r#"__sir::call_new("Dog", vec![])"#),
            "got:\n{}",
            a.source
        );
        assert!(
            a.source.contains(r#"__sir::ivar_set("@name", "#),
            "got:\n{}",
            a.source
        );
    }

    #[test]
    fn accepts_empty_module_def() {
        // `module Foo … end` with method defs hoisted (empty body) → accepted,
        // emits only a comment marker.
        let md = Stmt::ModuleDef {
            name: "Foo".into(),
            body: vec![],
            span: s(),
        };
        let a = compile(&feat_module(vec![md], &[Feature::Modules]))
            .expect("empty module def accepted");
        assert!(a.source.contains("// module Foo"), "got:\n{}", a.source);
    }

    #[test]
    fn rejects_stateful_module_def() {
        // A module whose body carries an executable statement is out of scope.
        let md = Stmt::ModuleDef {
            name: "Foo".into(),
            body: vec![Stmt::ExprStmt {
                expr: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        };
        let err = compile(&feat_module(vec![md], &[Feature::Modules]))
            .expect_err("stateful module rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
        assert!(
            err.message.contains("non-empty body"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn allows_const_as_new_class_name() {
        // `Dog.new` lowers the class as a `Const` VarRef in __new__'s name
        // slot → lifted to a string, so `reject_const_ref` must NOT flag it.
        let st = Stmt::ExprStmt {
            expr: Expr::BuiltinCall {
                name: "__new__".into(),
                args: vec![Expr::VarRef {
                    name: "Dog".into(),
                    scope: Scope::Const,
                    span: s(),
                }],
                effects: EffectSet::PURE,
                span: s(),
            },
            span: s(),
        };
        let a = compile(&feat_module(
            vec![st],
            &[Feature::Classes, Feature::Constants],
        ))
        .expect("const class name in __new__ is allowed");
        assert!(
            a.source.contains(r#"__sir::call_new("Dog", vec![])"#),
            "got:\n{}",
            a.source
        );
    }
}
