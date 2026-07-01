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

#[cfg(test)]
mod tests {
    use super::*;
    use semantic_ir::{
        Block, EffectSet, Expr, FeatureManifest, Function, Metadata, Span,
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
}
