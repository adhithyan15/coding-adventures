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
    ParamKind,
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
}
