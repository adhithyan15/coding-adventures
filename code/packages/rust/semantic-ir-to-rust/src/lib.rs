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
    Artifact, ArtifactMetadata, Backend, BackendError, BackendErrorKind, Feature, Module,
    ParamKind,
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

#[cfg(test)]
mod tests {
    use super::*;
    use semantic_ir::{
        Block, EffectSet, Expr, FeatureManifest, Function, Metadata, Param, Scope, Span,
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
                    value: Expr::IntLit { value: 42, span: s() },
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
        let module = twig_to_semantic_ir::compile_source(
            "(define (id x) x)\n(id 42)",
            "demo",
        )
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
        assert!(a.source.contains("__sir::print(add("));
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
        assert!(a.source.contains("__sir::Value::Closure(::std::rc::Rc::new"));
        assert!(a.source.contains("__sir::apply_closure"));
        assert!(a.source.contains("Globals (initialised in _init): add5"));
    }

    #[test]
    fn output_is_deterministic() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (id x) x)\n(id 7)",
            "demo",
        )
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
                        Expr::StrLit { value: "hi".into(), span: s() },
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
            Param { name: "a".into(), kind: ParamKind::Required, sir_type: None, default: None, span: s() },
            Param {
                name: "name".into(),
                kind: ParamKind::Keyword,
                sir_type: None,
                default: Some(Box::new(Expr::StrLit { value: "world".into(), span: s() })),
                span: s(),
            },
            Param { name: "opts".into(), kind: ParamKind::KwRest, sir_type: None, default: None, span: s() },
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
                Param { name: "greeting".into(), kind: ParamKind::Required, sir_type: None, default: None, span: s() },
                Param {
                    name: "name".into(),
                    kind: ParamKind::Keyword,
                    sir_type: None,
                    default: Some(Box::new(Expr::StrLit { value: "world".into(), span: s() })),
                    span: s(),
                },
            ],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef { name: "name".into(), scope: Scope::Param, span: s() },
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
                    args: vec![Expr::StrLit { value: "hi".into(), span: s() }],
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
}
