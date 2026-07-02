//! # semantic-ir-to-go
//!
//! Fourth backend for the narrow-waist Semantic IR — emits
//! **self-contained** Go source code from a [`semantic_ir::Module`].
//!
//! Output is a single `.go` file with `package main` plus inlined
//! runtime helpers; no `go.mod` dependencies required.  The file
//! compiles with `go build <file>.go` and runs.

mod emit;
mod runtime;

use semantic_ir::{
    Artifact, ArtifactMetadata, Backend, BackendError, BackendErrorKind, Feature, Module,
    ParamKind, Scope,
};

pub use emit::sanitize_ident;

pub fn compile(module: &Module) -> Result<Artifact, BackendError> {
    GoBackend::new().compile(module)
}

pub struct GoBackend;

impl GoBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoBackend {
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
    // `Floats` adds a `float64` arm to the Go runtime's `Value` plus
    // numeric promotion across the arithmetic/comparison helpers.
    // `ShortCircuit` is pure emit (a truthy-guarded IIFE over the
    // existing `_sir_truthy` helper) and needs no runtime change.
    Feature::Floats,
    Feature::ShortCircuit,
    // `MutableBindings` is pure emit — Go has native reassignment, so a
    // re-bound name just emits `<name> = <value>` (the matching
    // `LetBinding` already declared it with `:=`).  `Loops` maps `While`
    // / `ForRange` / `ForEach` onto Go's native `for`; `ForEach` adds a
    // `_sir_seq_iter` cons-list flattener to the runtime.
    Feature::MutableBindings,
    Feature::Loops,
    // `Sequences` and `Maps` are the final two SIR16 (v1) features — with
    // them the Go backend reaches **full SIR-v1 parity** (the fifth and
    // last backend to do so).  Both add a shared-mutable value to the Go
    // runtime: a `Seq` (pointer-backed `*[]Value`, so `SeqSet` mutates the
    // very sequence the caller holds and aliasing bindings observe the
    // write) and a `Map` (insertion-ordered assoc list keyed by the
    // runtime's structural value-equality; missing key ⇒ `nil`).  With all
    // six SIR16 features now declared, every SIR16 IR node has a real
    // (non-panicking) emit path — the only remaining `panic!`s cover
    // SIR17/18 nodes (classes/exceptions/str-concat) whose features stay
    // unaccepted, so they remain strictly unreachable.
    Feature::Sequences,
    Feature::Maps,
    // ── SIR19 (P2f) — default parameters ───────────────────────────
    // `DefaultParams` is a hybrid emit + runtime feature.  Go has no
    // native optional/default parameters, so we use a RUNTIME-MIMIC
    // strategy: a package-level MISSING sentinel (`_sir_missing`, a
    // distinct `*_missingMarker`) flows through the ordinary `Value`
    // channel.  A `DirectCall` that omits trailing defaulted arguments
    // pads up to the callee's full (fixed) arity with the sentinel; the
    // callee's body prologue replaces each sentinel with that param's
    // default expression, evaluated where earlier params are already
    // bound (call-time + param-scope).  `_sir_is_missing` tests the
    // sentinel by pointer identity; `_sir_format`/`_sir_value_eq` handle
    // it defensively so it never reaches user-visible output.
    Feature::DefaultParams,
    // ── KW6 — keyword parameters & arguments ───────────────────────
    // `KeywordParams` is DIRECT-lowered (no runtime library), mirroring
    // `DefaultParams` above.  Go has no native keyword arguments, so:
    //   * a `Keyword` DEF param emits as an ordinary positional `name Value`
    //     (it already does — `emit_function` walks every param uniformly),
    //     and an *optional* keyword's default is filled by the same
    //     `_sir_missing`/prologue path as a positional default; and
    //   * a `KeywordArg` at a `DirectCall` is resolved to the callee's
    //     declared position at emit time (the signature is statically known —
    //     see `emit::emit_direct_call`), producing a plain positional call.
    // Indirect/closure keyword calls are deferred (spec §Out of scope); the
    // frontends do not emit them, so accepting this feature is safe.
    Feature::KeywordParams,
    // ── C5 — collection-method dispatch (runtime catalog) ──────────
    //
    // `recv.meth(args…)` reaches the backend as
    // `BuiltinCall("__method__", [recv, StrLit("meth"), …])` and is
    // emitted as `_sir_call_method(recv, "meth", []Value{…})`, an EXPLICIT
    // type-switch + name-switch catalog inlined in the runtime preamble
    // (see `runtime::RUNTIME`), ported from the Python/TS `sir-runtime-oop`
    // reference for behavioural parity.
    //
    // Crucially, the deferred `Feature::MethodDispatch` variant (spec §C1)
    // is NOT needed here: the validator observes NO feature for a
    // `BuiltinCall("__method__", …)` (it falls through the builtin-name
    // match), so a *pure* collection-method module observes only the
    // features of its receiver/argument nodes — `Sequences`, `Strings`,
    // `Closures`, `Symbols`, `Maps`, `DynamicTyping` — every one already in
    // this accepted set.  So method-dispatch-WITHOUT-classes is accepted
    // with no feature-gate change, while class-bearing modules stay
    // rejected (they observe `Feature::Classes`, which we do NOT accept).
    // The runtime catalog IS the gate: an unknown method name fails at
    // runtime with a controlled "undefined method" panic, never via
    // reflection.  (See the `pure_method_dispatch_module_is_accepted` test.)
    //
    // ── E3 — exception handling (panic / recover) ──────────────────
    //
    // `Exceptions` maps `Stmt::TryCatch` onto an immediately-invoked func
    // with a deferred `recover` (Go has no native try/catch) and the `raise`
    // builtin onto `panic(_sir_new_error(...))`.  See `emit::emit_try_catch`.
    Feature::Exceptions,
    // ── O4 — user-defined-class OOP ────────────────────────────────
    //
    // `Classes` + `Constants` were first accepted (post-E3) only to admit
    // exception-*subclass* declarations; O4 widens acceptance to REAL
    // user-defined classes by ALSO accepting `InstanceVars`/`ClassVars`.  A
    // real OO module now routes through the inlined OOP runtime (see
    // `runtime::RUNTIME` — `_sir_call_new`/`_sir_call_super`/the
    // `*SirInstance` path in `_sir_call_method`, `_sir_ivar_get`/`set`,
    // `_sir_cvar_get`/`set`) rather than being rejected:
    //   * `class C < B` still contributes ONE ANCESTRY edge (shared with the
    //     exception hierarchy) registered at init;
    //   * instance methods hoist to top-level Functions and are wired to their
    //     class at runtime by emitted `__def_method__` registrations;
    //   * `C.new`/`super`/`self` lower to the `__new__`/`__super__`/`__self__`
    //     builtins, whose class/method NAMES ride in as `StrLit`s (never a
    //     `Const` ref) and are emitted through `quote_go_string` — so dispatch
    //     stays a pure `(class, method)` map lookup, NEVER reflection;
    //   * `@ivar`/`@@cvar` lower to `VarRef`/`Assign{Instance|ClassVar}`,
    //     emitted as `_sir_ivar_get/set` / `_sir_cvar_get/set`.
    //
    // Accepting these features stays SOUND: the widened surface does NOT emit a
    // general `Const` reference (a class name reaches the backend only as a
    // `StrLit` inside a builtin, or a `raise Foo` first-arg `Const` — both
    // handled explicitly), so `check_exception_soundness` below STILL cleanly
    // rejects genuinely-unsupported constructs — a `Const` used as a value
    // (`x = MyClass`) and a `Const` assignment (`FOO = 4`).  (`Feature::Modules`
    // is now ACCEPTED for MX5 mixins; see the note on it below.)
    Feature::Classes,
    Feature::Constants,
    Feature::InstanceVars,
    Feature::ClassVars,
    // ── MX5 — Ruby mixins (`module` + `include` / `extend`) ────────
    //
    // `Modules` admits `module M; …; end` (`Stmt::ModuleDef`) — a method
    // *owner* alongside classes.  A module body's `def`s hoist and register
    // via the SAME `__def_method__("M", …)` builtin classes use (keyed by the
    // module name); `include M` / `extend M` lower to the `__include__` /
    // `__extend__` builtins, whose owner/module NAMES ride in as `StrLit`s
    // (never a `Const`), so the runtime side stays a pure NAME-keyed map
    // operation — NEVER reflection.  Method resolution follows Ruby's MRO
    // (class → its included modules in reverse → superclass → …), cycle-guarded
    // (see `runtime::RUNTIME` — `_sir_included_modules`,
    // `_sir_resolve_instance_method`, `_sir_extend`, `_sir_call_class_method`).
    // A `ModuleDef` body reaching emit is now hosted (not rejected); the
    // soundness gate below recurses into it for the residual `Const` checks.
    Feature::Modules,
];

impl Backend for GoBackend {
    fn target_tag(&self) -> &'static str {
        "go"
    }

    fn accepts_features(&self) -> &'static [Feature] {
        ACCEPTED_FEATURES
    }

    fn accepts_intrinsics(&self) -> &'static [&'static str] {
        &[]
    }

    fn compile(&self, module: &Module) -> Result<Artifact, BackendError> {
        let r = semantic_ir::validate(module);
        if let Some(e) = r.errors().next().cloned() {
            return Err(BackendError {
                kind: BackendErrorKind::InvalidModule,
                message: format!("module failed validation: {}", e.message),
                span: e.span,
            });
        }

        // Capability gate.  `check_module` is the trait default (manifest
        // features must all be accepted; intrinsics whitelisted +
        // target-tagged).  We then layer on ONE Go-specific STRUCTURAL
        // rejection the feature manifest cannot express — a function mixing
        // keyword parameters with `*rest`/`**kwrest` (see
        // `check_no_keyword_rest_mix` for the full "why").  Doing it here,
        // right beside the manifest gate and before any emission, keeps the
        // backend's promise: it never silently emits wrong code.
        let mut cap_errors = self.check_module(module);
        check_no_keyword_rest_mix(module, &mut cap_errors);
        // E3: accepting `Classes`/`Constants` only for exception subclasses
        // means we must reject any OTHER class/const usage cleanly (never
        // mis-emit).  This layers a structural gate on top of the manifest
        // gate, right beside `check_no_keyword_rest_mix`.
        check_exception_soundness(module, &mut cap_errors);
        if let Some(e) = cap_errors.into_iter().next() {
            return Err(e);
        }

        if module.manifest.contains(Feature::TailCalls) {
            return Err(BackendError {
                kind: BackendErrorKind::UnsupportedFeature,
                message: "go backend cannot satisfy `tail_calls` feature".into(),
                span: module.span.clone(),
            });
        }

        let source = emit::emit_module(module);
        let metadata = ArtifactMetadata {
            bytes: source.len(),
            line_count: source.lines().count(),
            ..Default::default()
        };

        Ok(Artifact {
            filename: format!("{}.go", module.name.replace('/', "_")),
            source,
            metadata,
        })
    }
}

/// Reject any function that mixes a keyword parameter with a `*rest`/`**kwrest`
/// variadic — appends a [`BackendError`] to `errs` for each offender.
///
/// # Why keyword params mixed with `*rest`/`**kwrest` cannot be emitted
///
/// KW6 lowers keyword arguments by **static** keyword→positional resolution:
/// because a `DirectCall`'s callee signature is known at emit time,
/// `emit::emit_direct_call` walks the callee's *fixed* parameter slots
/// (`FN_PARAMS`/`ParamShape`) and drops each `KeywordArg` into the slot whose
/// name matches, producing a plain positional Go call.  That rewrite is only
/// sound when every slot maps to exactly one argument — i.e. when the callee
/// has **fixed arity**.
///
/// A `Rest` (`*rest`) or `KwRest` (`**opts`) param breaks that invariant: a
/// `*rest` slot collects a *variable* number of positional arguments, so there
/// is no single fixed position for a later keyword to resolve against.  The
/// core validator *accepts* the mixed shape — its ordering rule is
/// `Required* Rest? Keyword* KwRest?`, so Ruby's `def f(a, *rest, x: 1)` is
/// well-formed — and this backend now accepts `Feature::KeywordParams`, so such
/// a module reaches `emit_direct_call`.  There, the slot loop treats the
/// `*rest` param as an ordinary fixed slot:
///
/// ```text
///   def f(a, *rest, x: 1)         // slots: [a, rest, x]
///   f(10, x: 5)
///     slot 0 (a)    ← positional 10
///     slot 1 (rest) ← NOT a positional, NOT the keyword `x`, no default
///                     ⇒ debug build: `debug_assert!` PANICS
///                     ⇒ release build: pads `_sir_missing` — a SILENT
///                       mis-emit (the variadic slot gets one sentinel Value
///                       instead of a collected sequence)
///     slot 2 (x)    ← keyword 5
/// ```
///
/// Either outcome is unacceptable, so we reject the whole module cleanly rather
/// than let a bad slot map through.  This becomes frontend-reachable once the
/// Ruby frontend (KW7) emits keyword+splat methods; until a keyword-aware
/// runtime dispatch exists, the honest answer for the Go backend is
/// "unsupported construct".
///
/// The happy path — keyword params WITHOUT any `*rest`/`**kwrest` — is
/// fixed-arity and stays fully supported (see the KW6 emit + tests).
fn check_no_keyword_rest_mix(module: &Module, errs: &mut Vec<BackendError>) {
    for f in &module.functions {
        let has_keyword = f.params.iter().any(|p| p.kind == ParamKind::Keyword);
        let has_rest = f
            .params
            .iter()
            .any(|p| matches!(p.kind, ParamKind::Rest | ParamKind::KwRest));
        if has_keyword && has_rest {
            errs.push(BackendError {
                kind: BackendErrorKind::UnsupportedFeature,
                message: format!(
                    "go backend cannot emit a function mixing keyword parameters \
                     with *rest/**kwrest (static keyword resolution requires fixed arity); \
                     offending function `{}`",
                    f.name
                ),
                span: f.span.clone(),
            });
        }
    }
}

/// E3 structural soundness gate.
///
/// The Go backend accepts `Feature::Classes`/`Constants` ONLY for exception
/// subclasses and the `raise Foo`/`rescue Foo` class-name refs.  Any OTHER
/// class/const usage would reach an emit path that cannot represent it, so
/// we reject those modules CLEANLY here (an honest "unsupported construct"
/// error) rather than let a `panic!` or a mis-emit through.  Rejected:
///
///   * (a `ModuleDef` is NO LONGER rejected — MX5 hosts it as a mixin method
///     owner; we recurse into its body for the residual const checks);
///   * a `Const` reference (`VarRef{Const}`) ANYWHERE except as the first
///     argument of a `raise` builtin — a general constant-as-value (`x =
///     MyClass`) has no runtime representation here;
///   * a `Const` assignment (`Assign{Const}`, i.e. `FOO = 4`, including
///     inside a class body) — the backend has no constant store.
///
/// Every offending node appends one `BackendError`.  Instance/class-variable
/// usage is already rejected upstream by the manifest gate (those observe
/// `InstanceVars`/`ClassVars`, which this backend does not accept), so this
/// gate need only cover the const/module surface that `Classes`/`Constants`
/// acceptance newly admits.
fn check_exception_soundness(module: &Module, errs: &mut Vec<BackendError>) {
    for f in &module.functions {
        for s in &f.body.stmts {
            check_soundness_stmt(s, errs);
        }
        check_soundness_expr(&f.body.value, errs);
    }
}

fn unsupported(msg: &str, span: semantic_ir::Span) -> BackendError {
    BackendError {
        kind: BackendErrorKind::UnsupportedFeature,
        message: msg.into(),
        span,
    }
}

fn check_soundness_stmt(s: &semantic_ir::Stmt, errs: &mut Vec<BackendError>) {
    use semantic_ir::Stmt;
    match s {
        Stmt::Assign { scope: Scope::Const, name, span, .. } => {
            errs.push(unsupported(
                &format!(
                    "go backend cannot emit a constant assignment `{}` — \
                     `Feature::Constants` is accepted only for `raise ClassName` \
                     exception class references, not general constants",
                    name
                ),
                span.clone(),
            ));
        }
        // MX5 — a `ModuleDef` is now a HOSTED method owner (mixins).  Its body
        // still may not carry an unsupported `Const` use, so recurse into it
        // for the same residual const/module checks a class body gets, rather
        // than rejecting the module wholesale.
        Stmt::ModuleDef { body, .. } => {
            for st in body {
                check_soundness_stmt(st, errs);
            }
        }
        Stmt::LetBinding { value, .. }
        | Stmt::LetStarBinding { value, .. }
        | Stmt::Assign { value, .. }
        | Stmt::ExprStmt { expr: value, .. } => check_soundness_expr(value, errs),
        Stmt::While { cond, body, .. } => {
            check_soundness_expr(cond, errs);
            for st in &body.stmts {
                check_soundness_stmt(st, errs);
            }
            check_soundness_expr(&body.value, errs);
        }
        Stmt::ForRange { start, stop, step, body, .. } => {
            check_soundness_expr(start, errs);
            check_soundness_expr(stop, errs);
            check_soundness_expr(step, errs);
            for st in &body.stmts {
                check_soundness_stmt(st, errs);
            }
            check_soundness_expr(&body.value, errs);
        }
        Stmt::ForEach { iter, body, .. } => {
            check_soundness_expr(iter, errs);
            for st in &body.stmts {
                check_soundness_stmt(st, errs);
            }
            check_soundness_expr(&body.value, errs);
        }
        Stmt::SeqSet { seq, index, value, .. } => {
            check_soundness_expr(seq, errs);
            check_soundness_expr(index, errs);
            check_soundness_expr(value, errs);
        }
        Stmt::MapSet { map, key, value, .. } => {
            check_soundness_expr(map, errs);
            check_soundness_expr(key, errs);
            check_soundness_expr(value, errs);
        }
        Stmt::ClassDef { body, .. } | Stmt::SingletonClassDef { body, .. } => {
            for st in body {
                check_soundness_stmt(st, errs);
            }
        }
        Stmt::TryCatch { body, rescues, ensure_body, .. } => {
            for st in body {
                check_soundness_stmt(st, errs);
            }
            for r in rescues {
                for st in &r.body {
                    check_soundness_stmt(st, errs);
                }
            }
            if let Some(ens) = ensure_body {
                for st in ens {
                    check_soundness_stmt(st, errs);
                }
            }
        }
    }
}

fn check_soundness_expr(e: &semantic_ir::Expr, errs: &mut Vec<BackendError>) {
    use semantic_ir::Expr;
    match e {
        Expr::VarRef { scope: Scope::Const, name, span, .. } => {
            errs.push(unsupported(
                &format!(
                    "go backend cannot emit a constant reference `{}` outside a \
                     `raise ClassName` — `Feature::Constants` is accepted only for \
                     exception class references",
                    name
                ),
                span.clone(),
            ));
        }
        Expr::BuiltinCall { name, args, .. } if name == "raise" => {
            // The FIRST arg of `raise` may be a `Const` class name — that is
            // the whitelisted position, so skip it and check only the rest.
            for (i, a) in args.iter().enumerate() {
                if i == 0 && matches!(a, Expr::VarRef { scope: Scope::Const, .. }) {
                    continue;
                }
                check_soundness_expr(a, errs);
            }
        }
        Expr::BuiltinCall { args, .. } | Expr::DirectCall { args, .. } => {
            for a in args {
                check_soundness_expr(a, errs);
            }
        }
        Expr::IndirectCall { target, args, .. } => {
            check_soundness_expr(target, errs);
            for a in args {
                check_soundness_expr(a, errs);
            }
        }
        Expr::If { cond, then_branch, else_branch, .. } => {
            check_soundness_expr(cond, errs);
            check_soundness_block(then_branch, errs);
            check_soundness_block(else_branch, errs);
        }
        Expr::Block(b) => check_soundness_block(b, errs),
        Expr::MakeClosure { captures, .. } => {
            for c in captures {
                check_soundness_expr(&c.value, errs);
            }
        }
        Expr::SeqLit { items, .. } => {
            for it in items {
                check_soundness_expr(it, errs);
            }
        }
        Expr::SeqIndex { seq, index, .. } => {
            check_soundness_expr(seq, errs);
            check_soundness_expr(index, errs);
        }
        Expr::SeqLen { seq, .. } => check_soundness_expr(seq, errs),
        Expr::MapLit { entries, .. } => {
            for en in entries {
                check_soundness_expr(&en.key, errs);
                check_soundness_expr(&en.value, errs);
            }
        }
        Expr::MapGet { map, key, .. } => {
            check_soundness_expr(map, errs);
            check_soundness_expr(key, errs);
        }
        Expr::KeywordArg { value, .. } => check_soundness_expr(value, errs),
        // Leaves / nodes with no sub-exprs of interest.
        _ => {}
    }
}

fn check_soundness_block(b: &semantic_ir::Block, errs: &mut Vec<BackendError>) {
    for s in &b.stmts {
        check_soundness_stmt(s, errs);
    }
    check_soundness_expr(&b.value, errs);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_tag_is_go() {
        assert_eq!(GoBackend::new().target_tag(), "go");
    }

    #[test]
    fn end_to_end_twig_to_go_identity() {
        let m = twig_to_semantic_ir::compile_source("(define (id x) x)\n(id 42)", "demo")
            .expect("lower");
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("package main"));
        assert!(a.source.contains("func id(x Value) Value"));
        assert!(a.source.contains("id(Value(int64(42)))"));
    }

    #[test]
    fn end_to_end_twig_to_go_arithmetic() {
        let m = twig_to_semantic_ir::compile_source(
            "(define (add a b) (+ a b))\n(print (add 1 2))",
            "demo",
        )
        .expect("lower");
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("func add(a Value, b Value) Value"));
        assert!(a.source.contains("_sir_plus([]Value{a, b})"));
        assert!(a.source.contains("_sir_print([]Value{add("));
    }

    #[test]
    fn end_to_end_twig_to_go_closure() {
        let m = twig_to_semantic_ir::compile_source(
            "(define (adder n) (lambda (x) (+ x n)))\n(define add5 (adder 5))\n(print (add5 3))",
            "demo",
        )
        .expect("lower");
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("func _0x5flambda_5f0_(n Value, x Value) Value")
            || a.source.contains("func __lambda_0(n Value, x Value) Value"));
        assert!(a.source.contains("_sir_make_closure"));
    }

    #[test]
    fn rejects_tail_calls() {
        let mut m = twig_to_semantic_ir::compile_source("(+ 1 2)", "demo").expect("lower");
        m.manifest = semantic_ir::FeatureManifest::from_features(&[Feature::TailCalls]);
        let err = compile(&m).expect_err("tail calls rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
    }

    // ── KW6 hardening: keyword params mixed with *rest/**kwrest ───────────
    //
    // The core validator ACCEPTS `def f(a, *rest, x: 1)` (ordering rule
    // `Required* Rest? Keyword* KwRest?`) and this backend accepts
    // `KeywordParams`, so without an explicit gate such a module would reach
    // `emit_direct_call`, whose static keyword→position resolver PANICS in a
    // debug build (`debug_assert!`) or SILENTLY mis-emits in release (a single
    // `_sir_missing` sentinel lands in the variadic `*rest` slot).  These
    // tests pin the clean rejection AND prove the fixed-arity happy path is
    // untouched.
    use semantic_ir::{
        Block, EffectSet, Expr, FeatureManifest, Function, Metadata, Param, ParamKind, Scope, Span,
        Stmt,
    };

    fn sp() -> Span {
        Span::synthetic()
    }

    fn kw_rest_mix_module(rest_kind: ParamKind) -> Module {
        // Callee param order obeys the core validator's rule
        //   Required*  Rest?  Keyword*  KwRest?
        // so the offending shapes it ACCEPTS are:
        //   def f(a, *rest, x: 1)      // Rest   sits before the keyword
        //   def f(a, x: 1, **opts)     // KwRest sits after  the keyword
        // We build the corresponding param list per `rest_kind` — in both
        // cases the function carries BOTH a `Keyword` param and a variadic.
        let params = match rest_kind {
            ParamKind::Rest => vec![
                Param {
                    name: "a".into(),
                    kind: ParamKind::Required,
                    sir_type: None,
                    default: None,
                    span: sp(),
                },
                Param {
                    name: "rest".into(),
                    kind: ParamKind::Rest,
                    sir_type: None,
                    default: None,
                    span: sp(),
                },
                Param {
                    name: "x".into(),
                    kind: ParamKind::Keyword,
                    sir_type: None,
                    default: Some(Box::new(Expr::IntLit { value: 1, span: sp() })),
                    span: sp(),
                },
            ],
            ParamKind::KwRest => vec![
                Param {
                    name: "a".into(),
                    kind: ParamKind::Required,
                    sir_type: None,
                    default: None,
                    span: sp(),
                },
                Param {
                    name: "x".into(),
                    kind: ParamKind::Keyword,
                    sir_type: None,
                    default: Some(Box::new(Expr::IntLit { value: 1, span: sp() })),
                    span: sp(),
                },
                Param {
                    name: "opts".into(),
                    kind: ParamKind::KwRest,
                    sir_type: None,
                    default: None,
                    span: sp(),
                },
            ],
            other => panic!("kw_rest_mix_module: unexpected kind {other:?}"),
        };
        let f = Function {
            name: "f".into(),
            params,
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef { name: "a".into(), scope: Scope::Param, span: sp() },
                span: sp(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        };
        // Caller: main() { f(10, x: 5) }
        let main = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::ExprStmt {
                    expr: Expr::DirectCall {
                        fn_name: "f".into(),
                        args: vec![
                            Expr::IntLit { value: 10, span: sp() },
                            Expr::KeywordArg {
                                name: "x".into(),
                                value: Box::new(Expr::IntLit { value: 5, span: sp() }),
                                span: sp(),
                            },
                        ],
                        effects: EffectSet::PURE,
                        span: sp(),
                    },
                    span: sp(),
                }],
                value: Expr::NilLit { span: sp() },
                span: sp(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        };
        Module {
            name: "kw_rest_mix".into(),
            // The `*rest`/`**opts` param itself observes no distinct feature; the
            // keyword `x` observes `KeywordParams`, untyped params observe
            // `DynamicTyping`.  (This mirrors what the validator will accept.)
            manifest: FeatureManifest::from_features(&[
                Feature::KeywordParams,
                Feature::DynamicTyping,
            ]),
            imports: vec![],
            exports: vec![],
            functions: vec![f, main],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: sp(),
        }
    }

    #[test]
    fn rejects_keyword_params_mixed_with_rest() {
        // No panic, no mis-emit: a clean `Err` with the unsupported-construct
        // kind and a message that names the offending mix.
        let m = kw_rest_mix_module(ParamKind::Rest);
        let err = compile(&m).expect_err("keyword+*rest mix must be rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
        assert!(
            err.message.contains("keyword parameters")
                && err.message.contains("*rest/**kwrest"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn rejects_keyword_params_mixed_with_kwrest() {
        let m = kw_rest_mix_module(ParamKind::KwRest);
        let err = compile(&m).expect_err("keyword+**kwrest mix must be rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
    }

    #[test]
    fn keyword_params_without_rest_still_compile() {
        // Happy path: the SAME callee minus the `*rest` param is fixed-arity
        // and must still emit (guards against an over-broad rejection).
        let mut m = kw_rest_mix_module(ParamKind::Rest);
        m.functions[0].params.remove(1); // drop the `*rest` param
        let a = compile(&m).expect("keyword params without rest must compile");
        assert!(a.source.contains("func f("));
        // The keyword call resolves `x` to its slot; the omitted default is
        // padded with the missing sentinel — proving the static resolver ran.
        assert!(a.source.contains("f(Value(int64(10)), Value(int64(5)))"));
    }

    #[test]
    fn output_is_deterministic() {
        let m = twig_to_semantic_ir::compile_source("(define (id x) x)\n(id 7)", "demo")
            .expect("lower");
        let a = compile(&m).expect("compile");
        let b = compile(&m).expect("compile again");
        assert_eq!(a.source, b.source);
    }

    #[test]
    fn module_filename_sanitised() {
        let m = twig_to_semantic_ir::compile_source("(+ 1 2)", "compiler/lexer").expect("lower");
        let a = compile(&m).expect("compile");
        assert_eq!(a.filename, "compiler_lexer.go");
    }

    // ── C5: collection-method dispatch acceptance ─────────────────────────
    //
    // A pure collection-method module (a `__method__` dispatch with NO class
    // features) must be ACCEPTED — method dispatch is decoupled from classes.
    // `__method__` observes no feature in the validator, so the module carries
    // only its receiver/argument features (here `Sequences`), all accepted.
    #[test]
    fn pure_method_dispatch_module_is_accepted() {
        use semantic_ir::{
            Block, EffectSet, Expr, FeatureManifest, Function, Metadata, Span, Stmt,
        };
        let sp = Span::synthetic;
        // main() { [1,2,3].length }
        let dispatch = Expr::BuiltinCall {
            name: "__method__".into(),
            args: vec![
                Expr::SeqLit {
                    items: vec![
                        Expr::IntLit { value: 1, span: sp() },
                        Expr::IntLit { value: 2, span: sp() },
                        Expr::IntLit { value: 3, span: sp() },
                    ],
                    span: sp(),
                },
                Expr::StrLit { value: "length".into(), span: sp() },
            ],
            effects: EffectSet::PURE,
            span: sp(),
        };
        let main = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::ExprStmt { expr: dispatch, span: sp() }],
                value: Expr::NilLit { span: sp() },
                span: sp(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        };
        let m = Module {
            name: "coll".into(),
            // `Sequences` (the receiver) + `Strings` (the method-name StrLit).
            // Notably NO `Classes` / `InstanceVars` — this is method dispatch
            // decoupled from OOP, and it is accepted with no gate change.
            manifest: FeatureManifest::from_features(&[Feature::Sequences, Feature::Strings]),
            imports: vec![],
            exports: vec![],
            functions: vec![main],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: sp(),
        };
        let a = compile(&m).expect("pure method-dispatch module must be accepted");
        assert!(
            a.source.contains(r#"_sir_call_method(_sir_seq_lit"#),
            "expected dispatch emission; got:\n{}",
            a.source
        );
        assert!(a.source.contains(r#", "length", []Value{}"#));
    }

    // ── O4: a class carrying instance vars is now ACCEPTED ────────────────
    //
    // Pre-O4 this exact module (a `ClassDef` whose body assigns `@count`,
    // observing `Feature::InstanceVars`) was REJECTED at the manifest gate.
    // O4 accepts `InstanceVars`/`ClassVars`, so a real OO module now compiles:
    // the ivar assign lowers to `_sir_ivar_set("@count", …)` and the class
    // itself emits no top-level code (its ancestry edge, if any, registers at
    // init).  (`Block`, `Stmt`, `Scope`, … are already imported by the KW6
    // test block above; `sp()` is the shared span helper.)
    #[test]
    fn class_with_instance_vars_now_accepted() {
        // class Counter; @count = 0; end   (an ivar assign in the body)
        let class = Stmt::ClassDef {
            name: "Counter".into(),
            superclass: None,
            body: vec![Stmt::Assign {
                name: "@count".into(),
                scope: Scope::Instance,
                value: Expr::IntLit { value: 0, span: sp() },
                span: sp(),
            }],
            span: sp(),
        };
        let main = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![class],
                value: Expr::NilLit { span: sp() },
                span: sp(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        };
        let m = Module {
            name: "oop".into(),
            manifest: FeatureManifest::from_features(&[
                Feature::Classes,
                Feature::InstanceVars,
                Feature::MutableBindings,
            ]),
            imports: vec![],
            exports: vec![],
            functions: vec![main],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: sp(),
        };
        let a = compile(&m).expect("O4: instance-var class must now compile");
        assert!(
            a.source.contains(r#"_sir_ivar_set("@count", "#),
            "expected ivar-set emission; got:\n{}",
            a.source
        );
    }

    // A general constant assignment (`FOO = 4`) is rejected CLEANLY — the E3
    // soundness gate (or the upstream validator) turns it into an honest
    // `Err`, never a panic or a silent mis-emit.  `Constants` is accepted only
    // for `raise ClassName`.
    #[test]
    fn general_constant_assignment_rejected() {
        let main = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::Assign {
                    name: "FOO".into(),
                    scope: Scope::Const,
                    value: Expr::IntLit { value: 4, span: sp() },
                    span: sp(),
                }],
                value: Expr::NilLit { span: sp() },
                span: sp(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        };
        let m = Module {
            name: "const".into(),
            manifest: FeatureManifest::from_features(&[Feature::Constants]),
            imports: vec![],
            exports: vec![],
            functions: vec![main],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: sp(),
        };
        let err = compile(&m).expect_err("general const assignment must be rejected");
        assert!(matches!(
            err.kind,
            BackendErrorKind::UnsupportedFeature | BackendErrorKind::InvalidModule
        ));
    }

    // Direct proof that the E3 SOUNDNESS GATE fires: a `VarRef{Const}` used as
    // a value OUTSIDE a `raise` (here as a `print` argument) is a shape the
    // validator accepts (`Constants` observed, manifest declares it) yet the
    // Go backend cannot emit — so `check_exception_soundness` rejects it with
    // an `UnsupportedFeature` naming the constant reference.
    #[test]
    fn general_constant_reference_rejected_by_soundness_gate() {
        let main = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::ExprStmt {
                    expr: Expr::BuiltinCall {
                        name: "print".into(),
                        args: vec![Expr::VarRef {
                            name: "FOO".into(),
                            scope: Scope::Const,
                            span: sp(),
                        }],
                        effects: EffectSet::PURE,
                        span: sp(),
                    },
                    span: sp(),
                }],
                value: Expr::NilLit { span: sp() },
                span: sp(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        };
        let m = Module {
            name: "constref".into(),
            manifest: FeatureManifest::from_features(&[Feature::Constants]),
            imports: vec![],
            exports: vec![],
            functions: vec![main],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: sp(),
        };
        let err = compile(&m).expect_err("const-as-value must be rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
        assert!(
            err.message.contains("constant reference"),
            "expected the soundness gate's message; got: {}",
            err.message
        );
    }
}
