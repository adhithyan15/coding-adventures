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
    // SIR16 mutation & loops — emitted natively (assignment → `=`,
    // indexed set → `s[i] = v` / `m[k] = v`, `while` →
    // `while _sir_truthy(c):`, `for`-range → `for v in range(a, b, step):`,
    // `for`-each → `for v in it:`), per code/specs/sir-runtime.md.
    Feature::MutableBindings,
    Feature::Loops,
    // SIR17 OOP & scopes — class/module declarations register in the OOP
    // runtime; instance/class vars route through its stores; consts are
    // module-level bindings; `is_a?`-style dispatch goes through
    // `_sir_oop_call_method`.  Per code/specs/sir-runtime.md, with the
    // documented v0 limit (frontend hoists methods without receivers).
    Feature::Classes,
    Feature::Modules,
    Feature::InstanceVars,
    Feature::ClassVars,
    Feature::Constants,
    // SIR17 exceptions — `try`/`except`/`finally` is native; the SIR exception
    // object, `raise`, and ordered rescue-clause class matching come from
    // `coding-adventures-sir-runtime-exceptions`.  Per code/specs/sir-runtime.md.
    Feature::Exceptions,
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
        Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Span,
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

    // ── Execution-proof: block-capture round-trip actually *runs* ──────
    //
    // Every other Ruby→Python test above asserts the *shape* of the emitted
    // source.  This one additionally executes it through a real interpreter,
    // because RB1/RB2 introduced the first SIR shape that emits a **non-empty**
    // `MakeClosure` capture — a hoisted block that closes over the enclosing
    // method's block parameter — and we want proof the backend binds that
    // capture correctly at runtime, not just on paper.

    /// Probe whether `exe` is a usable Python interpreter by running `-c pass`.
    /// This distinguishes a genuinely-absent interpreter (and the Windows Store
    /// `python3` stub, which refuses to run and exits non-zero) from a real one,
    /// so the caller can *skip* cleanly rather than mistaking "no Python" for a
    /// test failure.
    fn python_is_runnable(exe: &str) -> bool {
        std::process::Command::new(exe)
            .args(["-c", "pass"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Run emitted Python `source` with the local SIR runtime packages on
    /// `PYTHONPATH`, returning captured stdout with newlines normalised.
    /// Returns `None` (⇒ skip the execution assertion) when no usable
    /// interpreter exists, so CI hosts without Python never hard-fail.  A
    /// present-but-erroring interpreter panics with the captured stderr — that
    /// is a real regression, not a skip.
    fn run_emitted_python(source: &str) -> Option<String> {
        let exe = ["python3", "python"].into_iter().find(|e| python_is_runnable(e))?;

        // Runtime packages live at <crate>/../../python/<pkg>/src (src layout).
        let py_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python");
        let pythonpath = std::env::join_paths([
            py_root.join("sir-runtime-core/src"),
            py_root.join("sir-runtime-pairs/src"),
        ])
        .expect("join PYTHONPATH");

        let file = std::env::temp_dir().join(format!("sir_rb3_{}.py", std::process::id()));
        std::fs::write(&file, source).expect("write temp python");
        let out = std::process::Command::new(exe)
            .arg(&file)
            .env("PYTHONPATH", &pythonpath)
            .output()
            .expect("spawn python");
        let _ = std::fs::remove_file(&file);

        assert!(
            out.status.success(),
            "emitted Python failed under {exe}:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        Some(String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"))
    }

    #[test]
    fn end_to_end_ruby_block_capture_executes_py() {
        // `outer`'s block `{ |x| yield x }` is hoisted to `__block_0` and its
        // `yield x` re-targets the *enclosing* method's block — so `__block_0`
        // closes over `outer`'s `__sir_block__`.  That is the first non-empty
        // `MakeClosure` capture in the pipeline.  `print` is one of the few
        // builtins `sir-runtime-core` implements natively, so the whole chain
        // (`outer` → `twice` → captured block → enclosing block) is runnable.
        let src = "def twice\n  yield 1\n  yield 2\nend\n\
                   def outer\n  twice { |x| yield x }\nend\n\
                   outer { |n| print n }\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");

        // Shape: the capture is threaded into the hoisted block, and the block
        // receives it as a prepended parameter (make_closure prepends captures).
        assert!(
            a.source.contains("def __block_0(__sir_block__, x):"),
            "hoisted block must take the captured block first; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("twice(_sir_make_closure(__block_0, [__sir_block__]))"),
            "enclosing block must be threaded into the non-empty capture; got:\n{}",
            a.source
        );

        // Execution-proof: running it prints the two yielded values, proving the
        // captured block reaches `outer`'s caller block at runtime.
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "1\n2\n", "emitted python printed unexpected output");
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

    // ── SIR16 mutation & loops: Ruby / direct SIR → native Python ───

    #[test]
    fn end_to_end_ruby_while_loop() {
        // Reassignment is a bare `=`; the condition routes through SIR
        // truthiness via `_sir_truthy`.
        let module = ruby_to_semantic_ir::compile_source(
            "i = 0\nwhile i < 3\n  i = i + 1\nend\nputs(i)\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(a.source.contains("    i = 0\n"), "got:\n{}", a.source);
        assert!(
            a.source.contains("while _sir_truthy(_sir_lt(i, 3)):"),
            "got:\n{}",
            a.source
        );
        assert!(a.source.contains("        i = _sir_plus(i, 1)"), "got:\n{}", a.source);
    }

    fn module_with_main_body(
        stmts: Vec<semantic_ir::Stmt>,
        value: Expr,
        feats: &[Feature],
    ) -> Module {
        Module {
            name: "demo".into(),
            manifest: FeatureManifest::from_features(feats),
            imports: vec![],
            exports: vec![],
            functions: vec![Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body: Block { stmts, value, span: s() },
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            }],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        }
    }

    #[test]
    fn emit_for_range_uses_native_range() {
        use semantic_ir::{Scope, Stmt};
        // for i in range(0, 3, 1): arr[i] = i  (SeqSet inside body)
        let body = Block {
            stmts: vec![Stmt::SeqSet {
                seq: Expr::VarRef { name: "arr".into(), scope: Scope::Local, span: s() },
                index: Expr::VarRef { name: "i".into(), scope: Scope::Local, span: s() },
                value: Expr::VarRef { name: "i".into(), scope: Scope::Local, span: s() },
                span: s(),
            }],
            value: Expr::NilLit { span: s() },
            span: s(),
        };
        let m = module_with_main_body(
            vec![
                Stmt::LetBinding {
                    name: "arr".into(),
                    sir_type: None,
                    value: Expr::SeqLit {
                        items: vec![Expr::IntLit { value: 0, span: s() }],
                        span: s(),
                    },
                    span: s(),
                },
                Stmt::ForRange {
                    var: "i".into(),
                    start: Expr::IntLit { value: 0, span: s() },
                    stop: Expr::IntLit { value: 3, span: s() },
                    step: Expr::IntLit { value: 1, span: s() },
                    body,
                    span: s(),
                },
            ],
            Expr::VarRef { name: "arr".into(), scope: Scope::Local, span: s() },
            &[Feature::Loops, Feature::Sequences, Feature::MutableBindings],
        );
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("for i in range(0, 3, 1):"), "got:\n{}", a.source);
        assert!(a.source.contains("arr[i] = i"), "got:\n{}", a.source);
    }

    #[test]
    fn emit_for_each_and_map_set() {
        use semantic_ir::{Scope, Stmt};
        // m = {}; for x in keys: m[x] = x
        let body = Block {
            stmts: vec![Stmt::MapSet {
                map: Expr::VarRef { name: "m".into(), scope: Scope::Local, span: s() },
                key: Expr::VarRef { name: "x".into(), scope: Scope::Local, span: s() },
                value: Expr::VarRef { name: "x".into(), scope: Scope::Local, span: s() },
                span: s(),
            }],
            value: Expr::NilLit { span: s() },
            span: s(),
        };
        let m = module_with_main_body(
            vec![
                Stmt::LetBinding {
                    name: "m".into(),
                    sir_type: None,
                    value: Expr::MapLit { entries: vec![], span: s() },
                    span: s(),
                },
                Stmt::LetBinding {
                    name: "keys".into(),
                    sir_type: None,
                    value: Expr::SeqLit {
                        items: vec![Expr::IntLit { value: 1, span: s() }],
                        span: s(),
                    },
                    span: s(),
                },
                Stmt::ForEach {
                    var: "x".into(),
                    iter: Expr::VarRef { name: "keys".into(), scope: Scope::Local, span: s() },
                    body,
                    span: s(),
                },
            ],
            Expr::VarRef { name: "m".into(), scope: Scope::Local, span: s() },
            &[Feature::Loops, Feature::Sequences, Feature::Maps, Feature::MutableBindings],
        );
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("for x in keys:"), "got:\n{}", a.source);
        assert!(a.source.contains("m[x] = x"), "got:\n{}", a.source);
    }

    #[test]
    fn empty_loop_body_emits_pass() {
        use semantic_ir::{Scope, Stmt};
        let m = module_with_main_body(
            vec![
                Stmt::LetBinding {
                    name: "i".into(),
                    sir_type: None,
                    value: Expr::IntLit { value: 0, span: s() },
                    span: s(),
                },
                Stmt::While {
                    cond: Expr::BoolLit { value: false, span: s() },
                    body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
                    span: s(),
                },
            ],
            Expr::VarRef { name: "i".into(), scope: Scope::Local, span: s() },
            &[Feature::Loops, Feature::MutableBindings],
        );
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("while _sir_truthy(False):"), "got:\n{}", a.source);
        assert!(a.source.contains("        pass"), "got:\n{}", a.source);
    }

    #[test]
    fn loop_in_expression_position_lifts_to_nested_def_with_nonlocal() {
        use semantic_ir::{Scope, Stmt};
        // main value = (if true then { while k<2 { total+=1; k+=1 }; total } else 9).
        // The loop block can't be walrus'd, so it lifts to a nested def
        // that declares `nonlocal total` / `nonlocal k`.
        let loop_block = Block {
            stmts: vec![Stmt::While {
                cond: Expr::BuiltinCall {
                    name: "<".into(),
                    args: vec![
                        Expr::VarRef { name: "k".into(), scope: Scope::Local, span: s() },
                        Expr::IntLit { value: 2, span: s() },
                    ],
                    effects: EffectSet::PURE,
                    span: s(),
                },
                body: Block {
                    stmts: vec![
                        Stmt::Assign {
                            name: "total".into(),
                            scope: Scope::Local,
                            value: Expr::BuiltinCall {
                                name: "+".into(),
                                args: vec![
                                    Expr::VarRef { name: "total".into(), scope: Scope::Local, span: s() },
                                    Expr::IntLit { value: 1, span: s() },
                                ],
                                effects: EffectSet::PURE,
                                span: s(),
                            },
                            span: s(),
                        },
                        Stmt::Assign {
                            name: "k".into(),
                            scope: Scope::Local,
                            value: Expr::BuiltinCall {
                                name: "+".into(),
                                args: vec![
                                    Expr::VarRef { name: "k".into(), scope: Scope::Local, span: s() },
                                    Expr::IntLit { value: 1, span: s() },
                                ],
                                effects: EffectSet::PURE,
                                span: s(),
                            },
                            span: s(),
                        },
                    ],
                    value: Expr::NilLit { span: s() },
                    span: s(),
                },
                span: s(),
            }],
            value: Expr::VarRef { name: "total".into(), scope: Scope::Local, span: s() },
            span: s(),
        };
        let m = module_with_main_body(
            vec![
                Stmt::LetBinding {
                    name: "total".into(),
                    sir_type: None,
                    value: Expr::IntLit { value: 0, span: s() },
                    span: s(),
                },
                Stmt::LetBinding {
                    name: "k".into(),
                    sir_type: None,
                    value: Expr::IntLit { value: 0, span: s() },
                    span: s(),
                },
            ],
            Expr::If {
                cond: Box::new(Expr::BoolLit { value: true, span: s() }),
                then_branch: Box::new(loop_block),
                else_branch: Box::new(Block {
                    stmts: vec![],
                    value: Expr::IntLit { value: 9, span: s() },
                    span: s(),
                }),
                span: s(),
            },
            &[Feature::Loops, Feature::MutableBindings],
        );
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("def __block_0():"), "expected lifted def; got:\n{}", a.source);
        assert!(a.source.contains("nonlocal k"), "got:\n{}", a.source);
        assert!(a.source.contains("nonlocal total"), "got:\n{}", a.source);
        // The def is called from the ternary, and the loop lives inside it.
        assert!(a.source.contains("__block_0() if _sir_truthy(True)"), "got:\n{}", a.source);
        assert!(a.source.contains("while _sir_truthy(_sir_lt(k, 2)):"), "got:\n{}", a.source);
    }

    #[test]
    fn global_assign_writes_globals_dict() {
        use semantic_ir::{Global, Scope, Stmt};
        // A `Global`-scoped reassignment writes the module-level
        // `_globals` dict (the same shape `_init`/`global_set` use).
        // The global must be declared in `module.globals` to satisfy
        // the validator's scope check.
        let m = Module {
            name: "demo".into(),
            manifest: FeatureManifest::from_features(&[Feature::MutableBindings, Feature::Globals]),
            imports: vec![],
            exports: vec![],
            functions: vec![Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body: Block {
                    stmts: vec![Stmt::Assign {
                        name: "counter".into(),
                        scope: Scope::Global,
                        value: Expr::IntLit { value: 5, span: s() },
                        span: s(),
                    }],
                    value: Expr::NilLit { span: s() },
                    span: s(),
                },
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            }],
            globals: vec![Global {
                name: "counter".into(),
                sir_type: None,
                init_function: "_init".into(),
                span: s(),
            }],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("_globals[\"counter\"] = 5"), "got:\n{}", a.source);
    }

    // ── SIR17 OOP & scopes: Ruby / direct SIR → native Python + oop ──

    #[test]
    fn end_to_end_ruby_class_inheritance_and_is_a_py() {
        let module = ruby_to_semantic_ir::compile_source(
            "class Dog < Animal\n  def speak\n    42\n  end\nend\nd = 5\nputs(d.is_a?(Integer))\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("from coding_adventures_sir_runtime_oop import"),
            "expected the OOP import; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("_sir_oop_define_class(\"Dog\", \"Animal\")"),
            "got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("_sir_oop_call_method(d, \"is_a?\", \"Integer\")"),
            "got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_ruby_const_in_class_body_py() {
        let module =
            ruby_to_semantic_ir::compile_source("class Foo\n  LEGS = 4\nend\n", "demo")
                .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(a.source.contains("_sir_oop_define_class(\"Foo\", None)"), "got:\n{}", a.source);
        assert!(a.source.contains("    LEGS = 4"), "got:\n{}", a.source);
    }

    #[test]
    fn end_to_end_ruby_class_var_py() {
        let module = ruby_to_semantic_ir::compile_source(
            "class Foo\n  @@count = 0\n  def inc\n    @@count = @@count + 1\n  end\nend\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(a.source.contains("_sir_oop_cvar_set(\"@@count\", 0)"), "got:\n{}", a.source);
        assert!(
            a.source.contains("_sir_oop_cvar_set(\"@@count\", _sir_plus(_sir_oop_cvar_get(\"@@count\"), 1))"),
            "got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_ruby_module_py() {
        let module =
            ruby_to_semantic_ir::compile_source("module Greet\n  def hi\n    1\n  end\nend\n", "demo")
                .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(a.source.contains("_sir_oop_define_class(\"Greet\", None)"), "got:\n{}", a.source);
    }

    #[test]
    fn instance_var_via_direct_sir_py() {
        use semantic_ir::{Scope, Stmt};
        // The frontend mis-parses multi-statement method bodies, so drive
        // the Instance scope directly: a method that writes then reads `@x`.
        let body = Block {
            stmts: vec![Stmt::Assign {
                name: "@x".into(),
                scope: Scope::Instance,
                value: Expr::IntLit { value: 1, span: s() },
                span: s(),
            }],
            value: Expr::VarRef { name: "@x".into(), scope: Scope::Instance, span: s() },
            span: s(),
        };
        let m = Module {
            name: "demo".into(),
            manifest: FeatureManifest::from_features(&[
                Feature::InstanceVars,
                Feature::MutableBindings,
            ]),
            imports: vec![],
            exports: vec![],
            functions: vec![Function {
                name: "bar".into(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body,
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            }],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };
        let a = compile(&m).expect("compile");
        assert!(a.source.contains("_sir_oop_ivar_set(\"@x\", 1)"), "got:\n{}", a.source);
        assert!(a.source.contains("_sir_oop_ivar_get(\"@x\")"), "got:\n{}", a.source);
    }

    #[test]
    fn non_oop_module_omits_oop_import_py() {
        let module = twig_to_semantic_ir::compile_source("(print (+ 1 2))", "demo").expect("lower");
        let a = compile(&module).expect("compile");
        assert!(!a.source.contains("sir_runtime_oop"), "got:\n{}", a.source);
    }

    // ─── SIR17 exceptions (Q7b) ─────────────────────────────────────────────

    #[test]
    fn end_to_end_ruby_begin_rescue_ensure_py() {
        // begin … raise … rescue Type => e … ensure … end → native
        // try/except/finally, dispatching on the rescue class through the
        // exception runtime and binding the caught value.
        let module = ruby_to_semantic_ir::compile_source(
            "begin\n  raise ArgumentError, \"bad\"\nrescue ArgumentError => e\n  puts(e)\nensure\n  puts(1)\nend\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        let src = &a.source;
        assert!(src.contains("from coding_adventures_sir_runtime_exceptions import"), "got:\n{}", src);
        assert!(src.contains("try:"), "got:\n{}", src);
        assert!(
            src.contains("_sir_exc_raise_error(\"ArgumentError\", \"bad\")"),
            "got:\n{}",
            src
        );
        assert!(src.contains("except Exception as __exc:"), "got:\n{}", src);
        assert!(
            src.contains("if _sir_exc_rescue_matches(__exc, [\"ArgumentError\"]):"),
            "got:\n{}",
            src
        );
        assert!(src.contains("e = __exc"), "got:\n{}", src);
        assert!(src.contains("raise\n"), "got:\n{}", src);
        assert!(src.contains("finally:"), "got:\n{}", src);
    }

    #[test]
    fn end_to_end_ruby_raise_message_only_py() {
        // `raise "boom"` (no class) → implicit RuntimeError carrying the message.
        let module =
            ruby_to_semantic_ir::compile_source("raise \"boom\"\n", "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("_sir_exc_raise_error(\"RuntimeError\", \"boom\")"),
            "got:\n{}",
            a.source
        );
    }

    #[test]
    fn non_throwing_module_omits_exc_import_py() {
        let module = twig_to_semantic_ir::compile_source("(print (+ 1 2))", "demo").expect("lower");
        let a = compile(&module).expect("compile");
        assert!(!a.source.contains("sir_runtime_exceptions"), "got:\n{}", a.source);
    }

    #[test]
    fn try_catch_bare_rescue_and_reraise_py() {
        use semantic_ir::{RescueClause, Scope, Stmt};
        // A bare `rescue` (no exception types, no binding) is a catch-all:
        // `rescue_matches(__exc, [])` is always true.  No `ensure` → no
        // `finally`.  Built directly because the frontend mis-parses some
        // bare-rescue surface forms.
        let try_stmt = Stmt::TryCatch {
            body: vec![Stmt::ExprStmt {
                expr: Expr::BuiltinCall {
                    name: "raise".into(),
                    args: vec![Expr::VarRef {
                        name: "RuntimeError".into(),
                        scope: Scope::Const,
                        span: s(),
                    }],
                    effects: EffectSet::PURE,
                    span: s(),
                },
                span: s(),
            }],
            rescues: vec![RescueClause {
                exception_types: vec![],
                binding: None,
                body: vec![Stmt::ExprStmt {
                    expr: Expr::IntLit { value: 7, span: s() },
                    span: s(),
                }],
                span: s(),
            }],
            ensure_body: None,
            span: s(),
        };
        let m = module_with_main_body(
            vec![try_stmt],
            Expr::NilLit { span: s() },
            &[Feature::Exceptions, Feature::Constants],
        );
        let a = compile(&m).expect("compile");
        let src = &a.source;
        assert!(src.contains("_sir_exc_raise_error(\"RuntimeError\")"), "got:\n{}", src);
        assert!(src.contains("if _sir_exc_rescue_matches(__exc, []):"), "got:\n{}", src);
        assert!(src.contains("else:"), "got:\n{}", src);
        assert!(src.contains("raise\n"), "got:\n{}", src);
        assert!(!src.contains("finally:"), "got:\n{}", src);
    }

    // ─── SIR cons pairs now ship in the dedicated package (Q8) ──────────────

    #[test]
    fn pairs_emit_from_dedicated_package_py() {
        // `car(cons(1, 2))` as a direct-SIR value, manifest declaring Pairs.
        let car_of_cons = Expr::BuiltinCall {
            name: "car".into(),
            args: vec![Expr::BuiltinCall {
                name: "cons".into(),
                args: vec![
                    Expr::IntLit { value: 1, span: s() },
                    Expr::IntLit { value: 2, span: s() },
                ],
                effects: EffectSet::PURE,
                span: s(),
            }],
            effects: EffectSet::PURE,
            span: s(),
        };
        let m = module_with_main_body(vec![], car_of_cons, &[Feature::Pairs]);
        let a = compile(&m).expect("compile");
        let src = &a.source;
        assert!(
            src.contains("from coding_adventures_sir_runtime_pairs import"),
            "got:\n{}",
            src
        );
        assert!(src.contains("cons as _sir_cons"), "got:\n{}", src);
        assert!(src.contains("_sir_car(_sir_cons(1, 2))"), "got:\n{}", src);
    }

    #[test]
    fn non_pairs_module_omits_pairs_import_py() {
        let module = twig_to_semantic_ir::compile_source("(print (+ 1 2))", "demo").expect("lower");
        let a = compile(&module).expect("compile");
        assert!(!a.source.contains("sir_runtime_pairs"), "got:\n{}", a.source);
    }

    // ─── SIR regex builtin → sir-runtime-regex (Q8b) ────────────────────────

    #[test]
    fn regex_builtin_emits_regex_runtime_py() {
        // `/ab+c/i` lowers to BuiltinCall("regex", [pattern, flags]).
        let rx = Expr::BuiltinCall {
            name: "regex".into(),
            args: vec![
                Expr::StrLit { value: "ab+c".into(), span: s() },
                Expr::StrLit { value: "i".into(), span: s() },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        let m = module_with_main_body(vec![], rx, &[Feature::Strings]);
        let a = compile(&m).expect("compile");
        let src = &a.source;
        assert!(
            src.contains("from coding_adventures_sir_runtime_regex import"),
            "got:\n{}",
            src
        );
        assert!(src.contains("compile as _sir_regex_compile"), "got:\n{}", src);
        assert!(src.contains("_sir_regex_compile(\"ab+c\", \"i\")"), "got:\n{}", src);
    }

    #[test]
    fn non_regex_module_omits_regex_import_py() {
        let module = twig_to_semantic_ir::compile_source("(print (+ 1 2))", "demo").expect("lower");
        let a = compile(&module).expect("compile");
        assert!(!a.source.contains("sir_runtime_regex"), "got:\n{}", a.source);
    }

    // ─── SIR backtick builtin → sir-runtime-shell (Q8c) ─────────────────────

    #[test]
    fn backtick_builtin_emits_shell_runtime_py() {
        // `` `echo hi` `` lowers to BuiltinCall("backtick", [cmd]).
        let bt = Expr::BuiltinCall {
            name: "backtick".into(),
            args: vec![Expr::StrLit { value: "echo hi".into(), span: s() }],
            effects: EffectSet::PURE,
            span: s(),
        };
        let m = module_with_main_body(vec![], bt, &[Feature::Strings]);
        let a = compile(&m).expect("compile");
        let src = &a.source;
        assert!(
            src.contains("from coding_adventures_sir_runtime_shell import backtick as _sir_shell_backtick"),
            "got:\n{}",
            src
        );
        assert!(src.contains("_sir_shell_backtick(\"echo hi\")"), "got:\n{}", src);
    }

    #[test]
    fn non_shell_module_omits_shell_import_py() {
        let module = twig_to_semantic_ir::compile_source("(print (+ 1 2))", "demo").expect("lower");
        let a = compile(&module).expect("compile");
        assert!(!a.source.contains("sir_runtime_shell"), "got:\n{}", a.source);
    }

    // ─── boolean / unary operator builtins (Q8d audit) ──────────────────────

    fn bc(name: &str, args: Vec<Expr>) -> Expr {
        Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
    }

    #[test]
    fn and_or_not_neg_builtins_lower_natively_py() {
        // Ruby `&&`/`and`/`||`/`or`/`!`/unary-minus reach the backend as these
        // builtins; they must lower natively (and/or short-circuit via SIR
        // truthiness), never route to the dispatch table.
        let and = bc("and", vec![Expr::BoolLit { value: true, span: s() }, Expr::IntLit { value: 1, span: s() }]);
        let a = compile(&module_with_main_body(vec![], and, &[])).expect("compile");
        assert!(a.source.contains("if _sir_truthy(__l) else __l"), "and: got:\n{}", a.source);

        let or = bc("or", vec![Expr::BoolLit { value: false, span: s() }, Expr::IntLit { value: 2, span: s() }]);
        let a = compile(&module_with_main_body(vec![], or, &[])).expect("compile");
        assert!(a.source.contains("__l if _sir_truthy(__l) else"), "or: got:\n{}", a.source);

        let not = bc("not", vec![Expr::BoolLit { value: true, span: s() }]);
        let a = compile(&module_with_main_body(vec![], not, &[])).expect("compile");
        assert!(a.source.contains("(not _sir_truthy(True))"), "not: got:\n{}", a.source);

        let neg = bc("neg", vec![Expr::IntLit { value: 5, span: s() }]);
        let a = compile(&module_with_main_body(vec![], neg, &[])).expect("compile");
        assert!(a.source.contains("(-(5))"), "neg: got:\n{}", a.source);
        // None of these route through the eager dispatch table.
        assert!(!a.source.contains("_sir_call_builtin(\"neg\""), "got:\n{}", a.source);
    }

    #[test]
    fn lambda_builtin_lowers_to_inner_closure_py() {
        // Ruby `lambda { … }` / `->{…}` reach the backend as
        // `BuiltinCall("lambda", [MakeClosure])`.  The lambda *is* its closure,
        // so it must emit the inner `MakeClosure` (→ `_sir_make_closure`)
        // directly, never route through the eager dispatch table.  Q10g: it is
        // wrapped in `_sir_as_lambda(...)` to mark it strict-arity.
        let mc = Expr::MakeClosure { fn_name: "main".into(), captures: vec![], span: s() };
        let lam = bc("lambda", vec![mc]);
        let a = compile(&module_with_main_body(vec![], lam, &[Feature::Closures]))
            .expect("compile");
        assert!(a.source.contains("_sir_make_closure("), "got:\n{}", a.source);
        assert!(a.source.contains("_sir_as_lambda(_sir_make_closure("), "got:\n{}", a.source);
        assert!(!a.source.contains("_sir_call_builtin(\"lambda\""), "got:\n{}", a.source);
    }

    #[test]
    fn range_builtin_lowers_to_runtime_and_imports_py() {
        // Ruby `a..b` / `a...b` reach the backend as
        // `BuiltinCall("range", [start, stop, exclusive])`.  It must lower to the
        // `_sir_range(...)` constructor, gate in the range-runtime import, and
        // never route through the eager dispatch table.
        let rng = bc(
            "range",
            vec![
                Expr::IntLit { value: 1, span: s() },
                Expr::IntLit { value: 5, span: s() },
                Expr::BoolLit { value: false, span: s() },
            ],
        );
        let a = compile(&module_with_main_body(vec![], rng, &[])).expect("compile");
        assert!(a.source.contains("_sir_range(1, 5, False)"), "got:\n{}", a.source);
        assert!(
            a.source.contains("from coding_adventures_sir_runtime_range import"),
            "missing range import; got:\n{}",
            a.source
        );
        assert!(!a.source.contains("_sir_call_builtin(\"range\""), "got:\n{}", a.source);
    }

    #[test]
    fn splat_in_seq_literal_emits_native_spread_py() {
        use semantic_ir::{Scope, Stmt};
        // Ruby `mid = [9]; [1, *mid, 3]` reaches the backend as a `SeqLit` whose
        // middle element is `BuiltinCall("splat", [mid])`.  Python splices it
        // natively as `*mid` inside the list literal — never the dispatch path.
        let bind = Stmt::LetBinding {
            name: "mid".into(),
            sir_type: None,
            value: Expr::SeqLit { items: vec![Expr::IntLit { value: 9, span: s() }], span: s() },
            span: s(),
        };
        let mid = Expr::VarRef { name: "mid".into(), scope: Scope::Local, span: s() };
        let seq = Expr::SeqLit {
            items: vec![
                Expr::IntLit { value: 1, span: s() },
                bc("splat", vec![mid]),
                Expr::IntLit { value: 3, span: s() },
            ],
            span: s(),
        };
        let a = compile(&module_with_main_body(vec![bind], seq, &[Feature::Sequences]))
            .expect("compile");
        assert!(a.source.contains("[1, *mid, 3]"), "got:\n{}", a.source);
        assert!(!a.source.contains("_sir_call_builtin(\"splat\""), "got:\n{}", a.source);
    }

    #[test]
    fn splat_and_double_splat_call_args_emit_native_py() {
        use semantic_ir::{Scope, Stmt};
        // Ruby `a = [1]; h = {}; main(*a, **h)` — the call args are
        // `BuiltinCall("splat", [a])` and `BuiltinCall("double_splat", [h])`.
        // Python emits both natively: `*a` (positional spread) and `**h`
        // (keyword spread).  (Targets `main`, which exists, so the module
        // validates; the spread shape is independent of the callee name.)
        let binds = vec![
            Stmt::LetBinding {
                name: "a".into(),
                sir_type: None,
                value: Expr::SeqLit { items: vec![Expr::IntLit { value: 1, span: s() }], span: s() },
                span: s(),
            },
            Stmt::LetBinding {
                name: "h".into(),
                sir_type: None,
                value: Expr::MapLit { entries: vec![], span: s() },
                span: s(),
            },
        ];
        let a_arg = bc("splat", vec![Expr::VarRef { name: "a".into(), scope: Scope::Local, span: s() }]);
        let h_arg = bc("double_splat", vec![Expr::VarRef { name: "h".into(), scope: Scope::Local, span: s() }]);
        let call = Expr::DirectCall {
            fn_name: "main".into(),
            args: vec![a_arg, h_arg],
            effects: EffectSet::PURE,
            span: s(),
        };
        let a = compile(&module_with_main_body(binds, call, &[Feature::Sequences, Feature::Maps]))
            .expect("compile");
        assert!(a.source.contains("(*a, **h)"), "got:\n{}", a.source);
        assert!(!a.source.contains("_sir_call_builtin(\"splat\""), "got:\n{}", a.source);
        assert!(!a.source.contains("_sir_call_builtin(\"double_splat\""), "got:\n{}", a.source);
    }

    #[test]
    fn defined_local_var_emits_static_description_py() {
        use semantic_ir::{Scope, Stmt};
        // `x = 1; defined?(x)` → the description "local-variable", emitted as a
        // constant string — never the dispatch fallthrough.
        let bind = Stmt::LetBinding {
            name: "x".into(),
            sir_type: None,
            value: Expr::IntLit { value: 1, span: s() },
            span: s(),
        };
        let d = bc("defined?", vec![Expr::VarRef { name: "x".into(), scope: Scope::Local, span: s() }]);
        let a = compile(&module_with_main_body(vec![bind], d, &[])).expect("compile");
        assert!(a.source.contains("\"local-variable\""), "got:\n{}", a.source);
        assert!(!a.source.contains("_sir_call_builtin(\"defined?\""), "got:\n{}", a.source);
    }

    #[test]
    fn defined_does_not_evaluate_operand_py() {
        // The core Ruby contract: `defined?` must NOT evaluate its operand.  For
        // `defined?(99)` we emit the constant "expression" and the operand
        // literal `99` must NOT appear anywhere in the output (proof it was not
        // rendered/evaluated).
        let d = bc("defined?", vec![Expr::IntLit { value: 99, span: s() }]);
        let a = compile(&module_with_main_body(vec![], d, &[])).expect("compile");
        assert!(a.source.contains("\"expression\""), "got:\n{}", a.source);
        assert!(!a.source.contains("99"), "operand was evaluated; got:\n{}", a.source);
        assert!(!a.source.contains("_sir_call_builtin(\"defined?\""), "got:\n{}", a.source);
    }

    #[test]
    fn defined_method_call_operand_emits_method_py() {
        use semantic_ir::Scope;
        // Q10h: `defined?(recv.meth)` — the operand is the `__method__` dispatch
        // envelope — reports the constant "method" (Ruby's category when the
        // method resolves), not the generic "expression".  The receiver `r` and
        // method name must NOT be rendered (non-evaluation contract).
        let recv = Expr::IntLit { value: 5, span: s() };
        let meth = bc("__method__", vec![recv, Expr::StrLit { value: "foo".into(), span: s() }]);
        let d = bc("defined?", vec![meth]);
        let a = compile(&module_with_main_body(vec![], d, &[Feature::Strings])).expect("compile");
        assert!(a.source.contains("\"method\""), "got:\n{}", a.source);
        assert!(!a.source.contains("__method__"), "operand was rendered; got:\n{}", a.source);
        assert!(!a.source.contains("\"foo\""), "operand was rendered; got:\n{}", a.source);
    }

    #[test]
    fn no_range_import_when_unused_py() {
        // A module that never builds a range must not gain the range dependency.
        let a = compile(&module_with_main_body(
            vec![],
            Expr::IntLit { value: 7, span: s() },
            &[],
        ))
        .expect("compile");
        assert!(
            !a.source.contains("coding_adventures_sir_runtime_range"),
            "unexpected range import; got:\n{}",
            a.source
        );
    }
}
