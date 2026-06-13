//! # semantic-ir-to-python
//!
//! Third backend for the narrow-waist Semantic IR — emits **self-
//! contained** Python 3 source code from a [`semantic_ir::Module`].
//!
//! Output is a single `.py` file that runs on stock CPython 3.10+
//! with no `pip` dependencies — the runtime helpers (`Symbol`,
//! `Pair`, `Closure` classes and the builtin implementations) are
//! pasted into every artifact.
//!
//! See [SIR14](../../../specs/SIR14-semantic-ir-to-python.md) for
//! the per-node lowering rules.

mod emit;
mod runtime;

use semantic_ir::{
    Artifact, ArtifactMetadata, Backend, BackendError, BackendErrorKind, Feature, Module,
};

pub use emit::sanitize_ident;

/// Convenience entry point.
pub fn compile(module: &Module) -> Result<Artifact, BackendError> {
    PythonBackend::new().compile(module)
}

/// The v0 Python backend.
pub struct PythonBackend;

impl PythonBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonBackend {
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
    // SIR16 expression features — emitted natively (sequences → list,
    // maps → dict, short-circuit → truthy-guarded lambda, interpolation →
    // display-joined), per code/specs/sir-runtime.md.
    Feature::Floats,
    Feature::Sequences,
    Feature::Maps,
    Feature::ShortCircuit,
    Feature::StringInterpolation,
];

impl Backend for PythonBackend {
    fn target_tag(&self) -> &'static str {
        "python"
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

        let cap_errors = self.check_module(module);
        if let Some(e) = cap_errors.into_iter().next() {
            return Err(e);
        }

        if module.manifest.contains(Feature::TailCalls) {
            return Err(BackendError {
                kind: BackendErrorKind::UnsupportedFeature,
                message: "python backend cannot satisfy `tail_calls` feature".into(),
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
            filename: format!("{}.py", module.name.replace('/', "_")),
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
        assert!(a.source.contains("def _sir_user_main():"));
        assert!(a.source.contains("return 42"));
        assert!(a.filename.ends_with(".py"));
    }

    #[test]
    fn target_tag_is_python() {
        let b = PythonBackend::new();
        assert_eq!(b.target_tag(), "python");
    }

    #[test]
    fn rejects_tail_calls() {
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[Feature::TailCalls]);
        let err = compile(&m).expect_err("tail calls rejected");
        assert_eq!(err.kind, BackendErrorKind::UnsupportedFeature);
    }

    #[test]
    fn end_to_end_twig_to_python_identity() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (id x) x)\n(id 42)",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        assert!(a.source.contains("def id(x):"));
        assert!(a.source.contains("return x"));
        assert!(a.source.contains("id(42)"));
    }

    #[test]
    fn end_to_end_twig_to_python_arithmetic() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (add a b) (+ a b))\n(print (add 1 2))",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        assert!(a.source.contains("def add(a, b):"));
        assert!(a.source.contains("_sir_plus(a, b)"));
        assert!(a.source.contains("_sir_print(add(1, 2))"));
    }

    #[test]
    fn end_to_end_twig_to_python_closure() {
        let module = twig_to_semantic_ir::compile_source(
            "(define (adder n) (lambda (x) (+ x n)))\n(define add5 (adder 5))\n(print (add5 3))",
            "demo",
        )
        .expect("lower");
        let a = compile(&module).expect("compile");
        assert!(a.source.contains("def __lambda_0("));
        assert!(a.source.contains("_sir_make_closure"));
        assert!(a.source.contains("_globals[\"add5\"]"));
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
        assert_eq!(a.filename, "compiler_lexer.py");
    }

    // ── End-to-end: Ruby → Semantic IR → Python ─────────────────────────
    //
    // These mirror the Twig→Python e2e tests above, but drive the *Ruby*
    // frontend (`ruby_to_semantic_ir::compile_source`) through the exact
    // same Python backend.  They prove the narrow-waist SIR genuinely
    // decouples frontends from backends: Ruby source in, runnable Python
    // out, with no Ruby-specific code in this crate.
    //
    // Snippets are restricted to the backend's `ACCEPTED_FEATURES`
    // (puts/arithmetic/defs/locals).  Ruby constructs that lower to
    // `Sequences`/`Maps`/`ShortCircuit` (arrays, hashes, case/in patterns)
    // are intentionally excluded — the backend rejects those features by
    // design, so they lower+validate but don't emit Python yet.

    #[test]
    fn end_to_end_ruby_to_python_puts() {
        // `puts("hello")` → a top-level `main` that calls the `puts`
        // builtin through the runtime dispatcher.
        let module = ruby_to_semantic_ir::compile_source("puts(\"hello\")\n", "demo")
            .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("def _sir_user_main():"),
            "expected a main function; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("_sir_call_builtin(\"puts\", [\"hello\"])"),
            "expected the puts call with the string literal; got:\n{}",
            a.source
        );
        assert!(a.source.contains("_sir_user_main()"), "expected main invocation");
        assert!(a.filename.ends_with(".py"));
    }

    #[test]
    fn end_to_end_ruby_to_python_def_and_call() {
        // A Ruby method definition + call round-trips to a Python `def`
        // whose body uses the runtime `_sir_plus`, invoked from `main`.
        let module = ruby_to_semantic_ir::compile_source(
            "def add(a, b)\n  a + b\nend\nputs(add(1, 2))\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("def add(a, b):"),
            "expected the add function def; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("return _sir_plus(a, b)"),
            "expected `a + b` to lower to _sir_plus; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("_sir_call_builtin(\"puts\", [add(1, 2)])"),
            "expected puts(add(1, 2)); got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_ruby_to_python_locals() {
        // Local assignments thread through to the Python body: the final
        // `puts(x + y)` references both locals via `_sir_plus(x, y)`.
        let module = ruby_to_semantic_ir::compile_source(
            "x = 1\ny = 2\nputs(x + y)\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("_sir_call_builtin(\"puts\", [_sir_plus(x, y)])"),
            "expected puts(x + y) referencing both locals; got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_ruby_to_python_is_deterministic() {
        // Same Ruby input → byte-identical Python (no nondeterministic
        // ordering leaking through the Ruby frontend).
        let module = ruby_to_semantic_ir::compile_source(
            "def add(a, b)\n  a + b\nend\nputs(add(1, 2))\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile");
        let b = compile(&module).expect("compile again");
        assert_eq!(a.source, b.source);
    }

    // ── SIR16 expression features: Ruby → native Python ─────────────

    #[test]
    fn end_to_end_ruby_array_literal() {
        // `[10, 20, 30]` → native Python list literal.  (Native `SeqIndex`
        // emission `x[i]` is exercised by the case/in pattern test below.)
        let module =
            ruby_to_semantic_ir::compile_source("x = [10, 20, 30]\nputs(x)\n", "demo")
                .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(a.source.contains("[10, 20, 30]"), "got:\n{}", a.source);
    }

    #[test]
    fn end_to_end_ruby_hash_literal() {
        // `{a: 1}` → native dict keyed by an interned symbol.
        let module =
            ruby_to_semantic_ir::compile_source("puts({a: 1})\n", "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("{_sir_intern(\"a\"): 1}"),
            "expected a dict keyed by an interned symbol; got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_ruby_short_circuit_is_lazy_and_truthy() {
        // A `case/in` array pattern desugars to `LogicalAnd`-chained
        // checks (`len(x) == 2 && x[0] == 7`), which emit as a
        // truthy-guarded lambda (lazy rhs, SIR truthiness) — never a
        // bare Python `and`.
        let module = ruby_to_semantic_ir::compile_source(
            "x = [7, 8]\ncase x\nin [7, b]\n  puts(b)\nend\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("lambda __l:") && a.source.contains("_sir_truthy(__l)"),
            "expected truthy-guarded lambda for &&; got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_ruby_string_interpolation() {
        // `"v=#{x}"` → display-joined parts.
        let module = ruby_to_semantic_ir::compile_source("x = 5\nputs(\"v=#{x}\")\n", "demo")
            .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("_sir_to_display("),
            "expected interpolation via _sir_to_display; got:\n{}",
            a.source
        );
    }
}
