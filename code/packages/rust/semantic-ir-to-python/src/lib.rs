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
    // P2c default parameters — emitted via the sentinel + body-prologue
    // strategy (NOT Python's native def-time defaults, which cannot reference
    // earlier params).  A defaulted param emits `name=_SIR_MISSING`; the
    // function body opens with a resolve-prologue that, in param order,
    // rewrites each still-sentinel param to its (call-time, param-scoped)
    // default expression.  See `emit_function`.
    Feature::DefaultParams,
    // KW2 keyword parameters & arguments — Python-native.  A `Keyword` param
    // becomes a keyword-only parameter (after a bare `*`, or after an existing
    // `*args`); a `KeywordArg` call element becomes `name=value`.  Optional
    // keyword params reuse the same sentinel + body-prologue default machinery
    // as positional optionals.  See `emit_function` / the `KeywordArg` emit arm.
    Feature::KeywordParams,
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
        Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, ParamKind, Scope,
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
            py_root.join("sir-runtime-oop/src"),
            py_root.join("sir-runtime-range/src"),
            py_root.join("sir-runtime-regex/src"),
            // E2 execution-proof runs emitted `try/rescue` + `register_ancestry`
            // through a real interpreter, so the exceptions runtime must be
            // importable too.
            py_root.join("sir-runtime-exceptions/src"),
        ])
        .expect("join PYTHONPATH");

        // Unique per call: the process id alone collides when several
        // execution-proof tests run concurrently in the same test binary (they
        // would clobber each other's temp file). A per-call atomic counter
        // disambiguates.
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let file = std::env::temp_dir()
            .join(format!("sir_rb3_{}_{}.py", std::process::id(), nonce));
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
    fn end_to_end_variadic_rest_collects_args_py() {
        // M3 execution-proof: `def f(*a); a.length; end; puts f(1, 2, 3)` →
        // the splat param collects the three positional arguments, so the
        // dispatched `length` reports `3`. Proves the emitted `def f(*a):`
        // binds variadics at runtime, not just on paper.
        let src = "def f(*a)\n  a.length\nend\nprint f(1, 2, 3)\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(a.source.contains("def f(*a):"), "got:\n{}", a.source);
        assert!(a.source.contains("a = list(a)"), "rest param must normalize to list; got:\n{}", a.source);
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "3\n", "emitted python printed unexpected output");
        }
    }

    #[test]
    fn end_to_end_ruby_default_param_resolves_at_call_time_executes_py() {
        // Ruby-1.0 (P7) full-pipeline execution-proof: Ruby SOURCE with a
        // call-time, param-referencing default → ruby-to-semantic-ir →
        // semantic-ir-to-python → CPython.  `def f(a, b = a + 1)` is legal
        // Ruby (the default sees the EARLIER param `a`).  Two calls:
        //   • `f(5)`     omits `b` → default resolves `b = a + 1 = 6`
        //   • `f(5, 10)` passes `b` → default suppressed, `b = 10`
        // so stdout must be `6` then `10`.  This is the discriminating proof
        // that the Ruby frontend now PRODUCES `Param.default` (it previously
        // dropped the `= <expr>` subtree) AND that the default is genuinely
        // call-time and param-scoped end to end — not Python def-time
        // semantics (which could not reference `a`).
        // NB: the def body is `b + 0` rather than a bare `b` — the Ruby
        // parser currently mis-parses a method body that is a single bare
        // identifier as a no-paren call (a pre-existing quirk unrelated to
        // defaults); `b + 0` is an honest expression that evaluates to `b`.
        let src = "def f(a, b = a + 1)\n  b + 0\nend\nprint f(5)\nprint f(5, 10)\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");

        // Frontend must have produced the default and declared the feature.
        assert!(
            module.manifest.contains(Feature::DefaultParams),
            "ruby frontend must declare DefaultParams; manifest = {:?}",
            module.manifest
        );
        let f = module
            .functions
            .iter()
            .find(|f| f.name == "f")
            .expect("fn f");
        assert!(
            f.params[1].default.is_some(),
            "b must carry a lowered default"
        );

        let a = compile(&module).expect("compile to python");
        // Sentinel-default + body-prologue shape (call-time, param-scoped).
        assert!(
            a.source.contains("def f(a, b=_SIR_MISSING):"),
            "got:\n{}",
            a.source
        );
        assert!(a.source.contains("    if b is _SIR_MISSING:"), "got:\n{}", a.source);
        // The omitting call passes only the present arg (no frontend padding).
        assert!(a.source.contains("f(5)"), "got:\n{}", a.source);

        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(
                stdout, "6\n10\n",
                "Ruby call-time param-scoped default produced wrong output"
            );
        }
    }

    #[test]
    fn end_to_end_ruby_keyword_params_and_args_execute_py() {
        // KW7 (Ruby-1.0 unblock) full-pipeline execution-proof: Ruby SOURCE
        // with a keyword parameter list AND keyword call arguments →
        // ruby-to-semantic-ir → semantic-ir-to-python → CPython.
        //
        //   def greet(greeting:, name: "world")
        //     "#{greeting}, #{name}"
        //   end
        //
        // `greeting:` is a REQUIRED keyword (no default); `name: "world"` is
        // an OPTIONAL keyword.  Two calls exercise both paths:
        //   • greet(greeting: "hi")              → omits `name` → "hi, world"
        //   • greet(greeting: "hi", name: "ada") → supplies `name` → "hi, ada"
        // so stdout must be `hi, world` then `hi, ada`.  This is the
        // discriminating proof that the Ruby frontend now PRODUCES keyword
        // params (`ParamKind::Keyword`) and keyword args (`Expr::KeywordArg`)
        // and that they bind BY NAME end to end — the single most-requested
        // modern-Ruby gap.
        //
        // NB: the output builtin is `print` rather than `puts` — `print` is
        // one of the few builtins `sir-runtime-core` implements natively
        // (same reason the P7 default-param execution-proof above uses it).
        let src = "def greet(greeting:, name: \"world\")\n  \"#{greeting}, #{name}\"\nend\n\
                   print greet(greeting: \"hi\")\n\
                   print greet(greeting: \"hi\", name: \"ada\")\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");

        // Frontend must have produced keyword params and declared the feature.
        assert!(
            module.manifest.contains(Feature::KeywordParams),
            "ruby frontend must declare KeywordParams; manifest = {:?}",
            module.manifest
        );
        let greet = module
            .functions
            .iter()
            .find(|f| f.name == "greet")
            .expect("fn greet");
        assert_eq!(greet.params[0].kind, ParamKind::Keyword);
        assert!(
            greet.params[0].default.is_none(),
            "`greeting:` is a required keyword (no default)"
        );
        assert_eq!(greet.params[1].kind, ParamKind::Keyword);
        assert!(
            greet.params[1].default.is_some(),
            "`name: \"world\"` is an optional keyword (has a default)"
        );

        let a = compile(&module).expect("compile to python");
        // Python-native keyword-only shape: a bare `*` opens the keyword-only
        // region; the optional keyword reuses the sentinel-default machinery.
        assert!(
            a.source.contains("def greet(*, greeting"),
            "keyword params must be keyword-only after a bare `*`; got:\n{}",
            a.source
        );

        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(
                stdout, "hi, world\nhi, ada\n",
                "Ruby keyword params/args produced wrong output"
            );
        }
    }

    #[test]
    fn end_to_end_default_param_resolves_at_call_time_executes_py() {
        // P2c discriminating execution-proof.  `def f(a, b)` where `b`'s default
        // is `a + 1` — a *param-referencing* default that Python's native
        // def-time defaults cannot express.  Two calls:
        //   • `f(5)`     omits `b` → prologue resolves `b = a + 1 = 6`
        //   • `f(5, 10)` passes `b` → prologue is skipped, `b = 10`
        // so stdout must be `6` then `10`.  This proves the sentinel binds on
        // omission, the prologue runs in body scope (sees `a`), and a supplied
        // argument suppresses the default.
        use semantic_ir::{Param, ParamKind, Scope, Stmt};

        let default_b = Expr::BuiltinCall {
            name: "+".into(),
            args: vec![
                Expr::VarRef { name: "a".into(), scope: Scope::Param, span: s() },
                Expr::IntLit { value: 1, span: s() },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        let f = Function {
            name: "f".into(),
            params: vec![
                Param { name: "a".into(), sir_type: None, kind: ParamKind::Required, default: None, span: s() },
                Param {
                    name: "b".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: Some(Box::new(default_b)),
                    span: s(),
                },
            ],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                // return b — so the printed value *is* the resolved default:
                // f(5) → b defaults to a+1 = 6; f(5, 10) → b = 10.
                value: Expr::VarRef { name: "b".into(), scope: Scope::Param, span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        // main: print(f(5)); print(f(5, 10))
        let call_f = |args: Vec<Expr>| Expr::DirectCall {
            fn_name: "f".into(),
            args,
            effects: EffectSet::PURE,
            span: s(),
        };
        let print_call = |inner: Expr| Stmt::ExprStmt {
            expr: Expr::BuiltinCall {
                name: "print".into(),
                args: vec![inner],
                effects: EffectSet::PURE,
                span: s(),
            },
            span: s(),
        };
        let main = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![
                    print_call(call_f(vec![Expr::IntLit { value: 5, span: s() }])),
                    print_call(call_f(vec![
                        Expr::IntLit { value: 5, span: s() },
                        Expr::IntLit { value: 10, span: s() },
                    ])),
                ],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let m = Module {
            name: "demo".into(),
            manifest: FeatureManifest::from_features(&[
                Feature::DefaultParams,
                Feature::DynamicTyping,
            ]),
            imports: vec![],
            exports: vec![],
            functions: vec![f, main],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };
        let a = compile(&m).expect("compile to python");
        // Sentinel default + body prologue (the call-time, param-scoped shape).
        assert!(a.source.contains("def f(a, b=_SIR_MISSING):"), "got:\n{}", a.source);
        assert!(a.source.contains("    if b is _SIR_MISSING:"), "got:\n{}", a.source);
        assert!(a.source.contains("        b = _sir_plus(a, 1)"), "got:\n{}", a.source);
        // The omitting call passes only the present arg (no padding).
        assert!(a.source.contains("f(5)"), "got:\n{}", a.source);
        // Execution-proof via the PYTHONPATH-aware harness: 6 then 10.
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "6\n10\n", "call-time default produced wrong output");
        }
    }

    #[test]
    fn end_to_end_block_captures_outer_local_executes_py() {
        // M4 execution-proof: a block reads an enclosing local (`base`). The
        // hoisted block must capture it (prepended as a leading parameter) and
        // the `MakeClosure` must thread the enclosing value, so that when
        // `apply` yields into the block, `n + base` evaluates with base = 100.
        let src = "def apply\n  yield 5\nend\n\
                   def run\n  base = 100\n  apply { |n| print n + base }\nend\n\
                   run()\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        // `base` is captured → prepended before the block param `n`.
        assert!(
            a.source.contains("def __block_0(base, n):"),
            "block must capture `base` as a leading param; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("__block_0, [base]"),
            "MakeClosure must thread the enclosing `base`; got:\n{}",
            a.source
        );
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "105\n", "emitted python printed unexpected output");
        }
    }

    #[test]
    fn end_to_end_case_when_case_equality_executes_py() {
        // M5 execution-proof: a `case/when` over mixed pattern kinds dispatches
        // by Ruby case-equality (`===`), not `==`:
        //   • `when 10..20` → range membership
        //   • `when /hi/`   → regex match
        //   • `when Integer`→ class match (is_a?)
        //   • else          → fallthrough
        // Clauses are tested in order; a `when 10..20` tested against a String
        // must not raise (Ruby returns false).
        let src = "def label(x)\n  case x\n  when 10..20\n    print \"R\"\n  \
                   when /hi/\n    print \"X\"\n  when Integer\n    print \"I\"\n  \
                   else\n    print \"O\"\n  end\nend\n\
                   label(15)\nlabel(\"hill\")\nlabel(5)\nlabel(3.5)\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("_sir_oop_case_eq("),
            "range/regex whens must use case_eq; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("_sir_oop_call_method") && a.source.contains("\"is_a?\""),
            "a class when must dispatch is_a?; got:\n{}",
            a.source
        );
        if let Some(stdout) = run_emitted_python(&a.source) {
            // 15→range(R), "hill"→regex(X), 5→Integer(I), 3.5→else(O).
            // `_sir_print` terminates each line with a newline.
            assert_eq!(stdout, "R\nX\nI\nO\n", "case-equality dispatch produced wrong branches");
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

    // ── O1: OOP object-model builtins gate the import + execute ───────────────

    #[test]
    fn oop_builtins_gate_the_oop_import_py() {
        // A module whose only OOP touch is a `__new__` (no `Feature::Classes`)
        // must still import the OOP runtime, else `_sir_oop_call_new` would be
        // undefined at runtime.  This proves the O1 import gating fires.
        let mut m = minimal_module();
        m.manifest = FeatureManifest::from_features(&[Feature::Strings]);
        m.functions[0].body.value = Expr::BuiltinCall {
            name: "__new__".into(),
            args: vec![Expr::StrLit { value: "Dog".into(), span: s() }],
            effects: EffectSet::PURE,
            span: s(),
        };
        let a = compile(&m).expect("compile");
        assert!(
            a.source.contains("call_new as _sir_oop_call_new"),
            "OOP import must be gated on __new__; got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_oop_new_and_dispatch_executes_py() {
        // O1 execution-proof (hand-built SIR — the frontend does not emit these
        // builtins until O2).  Model the classic `Dog.new(...).speak` shape:
        //   • a hoisted top-level `Dog_speak` returning "Rex says woof",
        //   • `__def_method__("Dog", "speak", MakeClosure(Dog_speak))` registers it,
        //   • `d = __new__("Dog")` allocates an instance,
        //   • `print(__method__(d, "speak"))` dispatches through the method table.
        // Running it through a real interpreter proves the O1 runtime wiring
        // (def_method → call_new → call_method) actually executes end to end.
        let speak_body = Block {
            stmts: vec![],
            value: Expr::StrLit { value: "Rex says woof".into(), span: s() },
            span: s(),
        };
        let speak_fn = Function {
            name: "Dog_speak".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: speak_body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };

        let def_method = Expr::BuiltinCall {
            name: "__def_method__".into(),
            args: vec![
                Expr::StrLit { value: "Dog".into(), span: s() },
                Expr::StrLit { value: "speak".into(), span: s() },
                Expr::MakeClosure { fn_name: "Dog_speak".into(), captures: vec![], span: s() },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        let new_dog = Expr::BuiltinCall {
            name: "__new__".into(),
            args: vec![Expr::StrLit { value: "Dog".into(), span: s() }],
            effects: EffectSet::PURE,
            span: s(),
        };
        let dispatch_speak = Expr::BuiltinCall {
            name: "__method__".into(),
            args: vec![
                Expr::VarRef { name: "d".into(), scope: Scope::Local, span: s() },
                Expr::StrLit { value: "speak".into(), span: s() },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        let print_stmt = Expr::BuiltinCall {
            name: "print".into(),
            args: vec![dispatch_speak],
            effects: EffectSet::PURE,
            span: s(),
        };

        let main_body = Block {
            stmts: vec![
                Stmt::ExprStmt { expr: def_method, span: s() },
                Stmt::LetBinding { name: "d".into(), sir_type: None, value: new_dog, span: s() },
            ],
            value: print_stmt,
            span: s(),
        };
        let main_fn = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: main_body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };

        let m = Module {
            name: "demo".into(),
            manifest: FeatureManifest::from_features(&[
                Feature::Classes,
                Feature::Closures,
                Feature::Strings,
            ]),
            imports: vec![],
            exports: vec![],
            functions: vec![speak_fn, main_fn],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("ruby")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };

        let a = compile(&m).expect("compile to python");
        // Shape: the three O1 helpers appear in the emitted source.
        assert!(a.source.contains("_sir_oop_def_method(\"Dog\", \"speak\","), "got:\n{}", a.source);
        assert!(a.source.contains("_sir_oop_call_new(\"Dog\")"), "got:\n{}", a.source);
        assert!(a.source.contains("_sir_oop_call_method(d, \"speak\")"), "got:\n{}", a.source);
        // Execution: the whole chain must run and print the method's result.
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "Rex says woof\n", "O1 dispatch produced wrong output");
        }
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
        // builtin.  `puts` now maps directly to the variadic runtime helper
        // `_sir_puts(...)` (like `print` → `_sir_print`), rather than routing
        // through the generic `_sir_call_builtin` dispatch, now that the
        // runtime implements Ruby `puts` semantics.
        let module = ruby_to_semantic_ir::compile_source("puts(\"hello\")\n", "demo")
            .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("def _sir_user_main():"),
            "expected a main function; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("_sir_puts(\"hello\")"),
            "expected the puts call with the string literal; got:\n{}",
            a.source
        );
        assert!(a.source.contains("_sir_user_main()"), "expected main invocation");
        assert!(a.filename.ends_with(".py"));

        // Execution proof: `puts "hello"` must emit exactly `hello\n` under a
        // real interpreter (Ruby's `puts` string+newline semantics).
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "hello\n", "puts should print `hello` + newline");
        }
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
            a.source.contains("_sir_puts(add(1, 2))"),
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
            a.source.contains("_sir_puts(_sir_plus(x, y))"),
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

    // -----------------------------------------------------------------------
    // Issue #59 — class-method defs (`def self.m`) + `super` as an expression.
    // Full Ruby SOURCE → SIR → Python, run through a real interpreter.
    // -----------------------------------------------------------------------

    #[test]
    fn end_to_end_ruby_class_method_def_and_call_executes_py() {
        // A REAL class method: `def self.zero` allocates and returns a
        // `Counter` (the classic factory idiom).  `Counter.zero` dispatches to
        // it via `__class_method__` → `_sir_oop_call_class_method`, and the
        // returned object answers an instance method.
        //
        // NOTE: `print` (not `puts`) — the `puts` builtin has no runtime
        // dispatch entry on this branch (a parallel PR adds it), so `puts` in a
        // *run* execution-proof raises `NameError`.  `print` maps to
        // `_sir_print`, which is in the core dispatch table and appends a
        // newline, so `print(c.val)` emits "42\n".
        let module = ruby_to_semantic_ir::compile_source(
            "class Counter\n\
            \x20 def self.zero\n\
            \x20   Counter.new\n\
            \x20 end\n\
            \x20 def val\n\
            \x20   42\n\
            \x20 end\n\
            end\n\
            c = Counter.zero\n\
            print(c.val)\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        // The class-method registration + dispatch wiring is present.
        assert!(
            a.source.contains("_sir_oop_def_class_method(\"Counter\", \"zero\""),
            "expected class-method registration; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("_sir_oop_call_class_method(\"Counter\", \"zero\")"),
            "expected class-method dispatch; got:\n{}",
            a.source
        );
        if let Some(out) = run_emitted_python(&a.source) {
            assert_eq!(out, "42\n", "Counter.zero.val must print 42");
        }
    }

    #[test]
    fn end_to_end_ruby_super_as_expression_executes_py() {
        // `super` used as an EXPRESSION: `def describe; super + 1; end` takes
        // the PARENT's `describe` result and adds 1.  Exercises both
        // super-as-subexpression lowering and the `__super__` runtime dispatch,
        // run end to end — the produced value flows into the enclosing `+`.
        //
        // The value is numeric (not `super + " tail"` string-concat) on
        // purpose: Ruby's `str + str` lowers to the numeric-init `_sir_plus`
        // (`add` seeds `total = 0`), so string `+` concatenation is a
        // PRE-EXISTING pipeline gap unrelated to #59 — see the CHANGELOG's
        // "deferred" note.  `super + 1` proves the #59 feature (super in
        // expression position) without depending on that separate gap.
        let module = ruby_to_semantic_ir::compile_source(
            "class Animal\n\
            \x20 def describe\n\
            \x20   40\n\
            \x20 end\n\
            end\n\
            class Cat < Animal\n\
            \x20 def describe\n\
            \x20   super + 1\n\
            \x20 end\n\
            end\n\
            c = Cat.new\n\
            print(c.describe)\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("_sir_oop_call_super(\"describe\", \"Cat\")"),
            "expected super dispatch from Cat; got:\n{}",
            a.source
        );
        // The super call sits INSIDE the `+` — proving expression position.
        assert!(
            a.source.contains("_sir_plus(_sir_oop_call_super(\"describe\", \"Cat\"), 1)"),
            "expected super to sit inside `+`; got:\n{}",
            a.source
        );
        if let Some(out) = run_emitted_python(&a.source) {
            assert_eq!(out, "41\n", "super (40) + 1 must be 41");
        }
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
    fn emits_register_ancestry_for_user_subclass_py() {
        // E2: a `class MyErr < StandardError` in a throwing module threads its
        // superclass edge to the exception runtime via a single program-init
        // `register_ancestry` call, before `main` runs.
        let module = ruby_to_semantic_ir::compile_source(
            "class MyErr < StandardError\nend\nbegin\n  raise MyErr, \"x\"\nrescue StandardError => e\n  print(\"caught\")\nend\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        let src = &a.source;
        assert!(
            src.contains("register_ancestry as _sir_exc_register_ancestry"),
            "expected the register_ancestry alias; got:\n{}",
            src
        );
        assert!(
            src.contains("_sir_exc_register_ancestry({\"MyErr\": \"StandardError\"})"),
            "expected the user ancestry registration; got:\n{}",
            src
        );
        // The registration must precede the user's main function so ancestry is
        // known before any rescue runs.
        let reg = src.find("_sir_exc_register_ancestry(").expect("reg present");
        let main = src.find("def _sir_user_main").expect("main present");
        assert!(reg < main, "registration must come before main; got:\n{}", src);
    }

    #[test]
    fn no_register_ancestry_when_no_user_superclass_py() {
        // A throwing module whose only class has *no* superclass (or has no
        // classes at all) must NOT emit an empty, meaningless registration.
        let module = ruby_to_semantic_ir::compile_source(
            "class Foo\nend\nbegin\n  raise RuntimeError, \"boom\"\nrescue RuntimeError => e\n  print(\"x\")\nend\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            !a.source.contains("_sir_exc_register_ancestry("),
            "should not register ancestry with no superclass edge; got:\n{}",
            a.source
        );
    }

    #[test]
    fn execution_user_subclass_rescued_by_ancestor_py() {
        // E2 execution-proof: `raise MyErr` (a user `StandardError` subclass)
        // *is* caught by `rescue StandardError` at runtime — proving the
        // registered user edge is walked by the matcher, not just emitted.
        let module = ruby_to_semantic_ir::compile_source(
            "class MyErr < StandardError\nend\nbegin\n  raise MyErr, \"x\"\nrescue StandardError => e\n  print(\"caught\")\nend\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        if let Some(stdout) = run_emitted_python(&a.source) {
            // `print("caught")` emits the string plus a trailing newline.
            assert_eq!(
                stdout, "caught\n",
                "user subclass should be rescued by its ancestor"
            );
        }
    }

    #[test]
    fn execution_unrelated_user_class_not_rescued_py() {
        // The dual: a user class that does NOT descend from the rescued type is
        // NOT caught, so the exception propagates (the interpreter exits
        // non-zero and `run_emitted_python` would panic on a success assertion).
        // We assert the *shape* of non-matching here and prove propagation via
        // a direct interpreter run that expects a failure exit.
        let module = ruby_to_semantic_ir::compile_source(
            "class Other < RuntimeError\nend\nbegin\n  raise Other, \"y\"\nrescue TypeError => e\n  print(\"wrong\")\nend\n",
            "demo",
        )
        .expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        let src = &a.source;
        // `Other` descends from RuntimeError, not TypeError, so the emitted
        // rescue names only TypeError and the registration records the true edge.
        assert!(
            src.contains("_sir_exc_register_ancestry({\"Other\": \"RuntimeError\"})"),
            "got:\n{}",
            src
        );
        assert!(
            src.contains("if _sir_exc_rescue_matches(__exc, [\"TypeError\"]):"),
            "got:\n{}",
            src
        );

        // Direct interpreter run: the program must *fail* (uncaught re-raise).
        let exe = ["python3", "python"].into_iter().find(|e| python_is_runnable(e));
        if let Some(exe) = exe {
            let py_root =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python");
            let pythonpath = std::env::join_paths([
                py_root.join("sir-runtime-core/src"),
                py_root.join("sir-runtime-oop/src"),
                py_root.join("sir-runtime-exceptions/src"),
            ])
            .expect("join PYTHONPATH");
            let file = std::env::temp_dir()
                .join(format!("sir_e2_nomatch_{}.py", std::process::id()));
            std::fs::write(&file, &a.source).expect("write temp python");
            let out = std::process::Command::new(exe)
                .arg(&file)
                .env("PYTHONPATH", &pythonpath)
                .output()
                .expect("spawn python");
            let _ = std::fs::remove_file(&file);
            assert!(
                !out.status.success(),
                "unrelated user class must NOT be rescued (expected propagation); stdout={:?}",
                String::from_utf8_lossy(&out.stdout)
            );
        }
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

    // ── KW2: keyword-parameter & keyword-argument emission ──────────────────
    //
    // These build SIR modules DIRECTLY (the Ruby/Python frontends do not yet
    // produce keyword params — that is KW7/KW8), so we hand-assemble the
    // `Keyword` params and `KeywordArg` call elements the validator now accepts.

    /// Build `def greet(greeting, *, name=<default "world">): return "<greeting>, <name>"`
    /// — one positional required param, one optional keyword param — as a
    /// `Function`.  Reused by several KW2 tests below.
    fn greet_function() -> Function {
        use semantic_ir::{Param, ParamKind, Scope};
        // Body: `return greeting + ", " + name` (SIR string concat).
        let body_value = Expr::StrConcat {
            parts: vec![
                Expr::VarRef { name: "greeting".into(), scope: Scope::Param, span: s() },
                Expr::StrLit { value: ", ".into(), span: s() },
                Expr::VarRef { name: "name".into(), scope: Scope::Param, span: s() },
            ],
            span: s(),
        };
        Function {
            name: "greet".into(),
            params: vec![
                Param {
                    name: "greeting".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: None,
                    span: s(),
                },
                Param {
                    name: "name".into(),
                    sir_type: None,
                    kind: ParamKind::Keyword,
                    default: Some(Box::new(Expr::StrLit { value: "world".into(), span: s() })),
                    span: s(),
                },
            ],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: body_value, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }
    }

    /// Assemble a module: `greet` (above) + a `main` whose body value is the
    /// given call expression, printed via `print(...)`.
    fn kw_module(main_value_call: Expr) -> Module {
        let print_stmt = semantic_ir::Stmt::ExprStmt {
            expr: Expr::BuiltinCall {
                name: "print".into(),
                args: vec![main_value_call],
                effects: EffectSet::PURE,
                span: s(),
            },
            span: s(),
        };
        let main = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![print_stmt], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        Module {
            name: "demo".into(),
            manifest: FeatureManifest::from_features(&[
                Feature::KeywordParams,
                Feature::DefaultParams,
                Feature::Strings,
                Feature::StringInterpolation,
                Feature::DynamicTyping,
            ]),
            imports: vec![],
            exports: vec![],
            functions: vec![greet_function(), main],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        }
    }

    /// A `greet("hi", <keyword args…>)` call expression.
    fn greet_call(kw_args: Vec<Expr>) -> Expr {
        let mut args = vec![Expr::StrLit { value: "hi".into(), span: s() }];
        args.extend(kw_args);
        Expr::DirectCall { fn_name: "greet".into(), args, effects: EffectSet::PURE, span: s() }
    }

    #[test]
    fn emit_keyword_param_def_uses_bare_star_separator_py() {
        // `def greet(greeting, *, name=...)`: the bare `*` opens Python's
        // keyword-only region, and the optional keyword param reuses the
        // sentinel + prologue default machinery (so it emits `name=_SIR_MISSING`,
        // NOT `name="world"`, giving call-time defaults).
        let a = compile(&kw_module(greet_call(vec![]))).expect("compile to python");
        assert!(
            a.source.contains("def greet(greeting, *, name=_SIR_MISSING):"),
            "keyword param must be keyword-only after a bare `*`; got:\n{}",
            a.source
        );
        // The default resolves in the body prologue, not at def time.
        assert!(
            a.source.contains("    if name is _SIR_MISSING:"),
            "optional keyword default must resolve via prologue; got:\n{}",
            a.source
        );
    }

    #[test]
    fn emit_required_keyword_param_has_no_default_py() {
        // A REQUIRED keyword param (`Keyword` + `default: None`) emits bare,
        // still after the `*`: `def h(a, *, b):`.
        use semantic_ir::{Param, ParamKind, Scope};
        let f = Function {
            name: "h".into(),
            params: vec![
                Param { name: "a".into(), sir_type: None, kind: ParamKind::Required, default: None, span: s() },
                Param { name: "b".into(), sir_type: None, kind: ParamKind::Keyword, default: None, span: s() },
            ],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef { name: "b".into(), scope: Scope::Param, span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        // main: print(h("x", b="y")) — required keyword must be supplied.
        let call = Expr::DirectCall {
            fn_name: "h".into(),
            args: vec![
                Expr::StrLit { value: "x".into(), span: s() },
                Expr::KeywordArg {
                    name: "b".into(),
                    value: Box::new(Expr::StrLit { value: "y".into(), span: s() }),
                    span: s(),
                },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        let print_stmt = semantic_ir::Stmt::ExprStmt {
            expr: Expr::BuiltinCall {
                name: "print".into(),
                args: vec![call],
                effects: EffectSet::PURE,
                span: s(),
            },
            span: s(),
        };
        let main = Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![print_stmt], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let m = Module {
            name: "demo".into(),
            manifest: FeatureManifest::from_features(&[
                Feature::KeywordParams,
                Feature::Strings,
                Feature::DynamicTyping,
            ]),
            imports: vec![],
            exports: vec![],
            functions: vec![f, main],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };
        let a = compile(&m).expect("compile to python");
        assert!(
            a.source.contains("def h(a, *, b):"),
            "required keyword param must be bare after `*`; got:\n{}",
            a.source
        );
        assert!(a.source.contains("h(\"x\", b=\"y\")"), "got:\n{}", a.source);
    }

    #[test]
    fn emit_keyword_param_after_rest_has_no_extra_star_py() {
        // When a `*args`/`Rest` param is present it ALREADY forces keyword-only,
        // so the backend must NOT inject a second bare `*` (that is a Python
        // SyntaxError): `def g(*rest, kw=_SIR_MISSING):`.
        use semantic_ir::{Param, ParamKind, Scope};
        let f = Function {
            name: "g".into(),
            params: vec![
                Param { name: "rest".into(), sir_type: None, kind: ParamKind::Rest, default: None, span: s() },
                Param {
                    name: "kw".into(),
                    sir_type: None,
                    kind: ParamKind::Keyword,
                    default: Some(Box::new(Expr::IntLit { value: 1, span: s() })),
                    span: s(),
                },
            ],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef { name: "kw".into(), scope: Scope::Param, span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let m = Module {
            name: "demo".into(),
            manifest: FeatureManifest::from_features(&[
                Feature::KeywordParams,
                Feature::DefaultParams,
                Feature::DynamicTyping,
            ]),
            imports: vec![],
            exports: vec![],
            functions: vec![f],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("test")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };
        let a = compile(&m).expect("compile to python");
        assert!(
            a.source.contains("def g(*rest, kw=_SIR_MISSING):"),
            "a Rest param already forces keyword-only; no extra `*`; got:\n{}",
            a.source
        );
        assert!(
            !a.source.contains("*rest, *,"),
            "must not emit a second bare `*` after `*rest`; got:\n{}",
            a.source
        );
    }

    #[test]
    fn emit_keyword_arg_at_call_site_py() {
        // Call side: a positional arg stays bare, a `KeywordArg` becomes
        // `name=value`: `greet("hi", name="ada")`.
        let call = greet_call(vec![Expr::KeywordArg {
            name: "name".into(),
            value: Box::new(Expr::StrLit { value: "ada".into(), span: s() }),
            span: s(),
        }]);
        let a = compile(&kw_module(call)).expect("compile to python");
        assert!(
            a.source.contains("greet(\"hi\", name=\"ada\")"),
            "keyword arg must emit as `name=value` after the positional; got:\n{}",
            a.source
        );
    }

    #[test]
    fn end_to_end_keyword_default_omitted_executes_py() {
        // Execution-proof: `greet("hi")` omits the optional keyword `name`, so
        // the prologue resolves it to "world" → prints `hi, world`.
        let a = compile(&kw_module(greet_call(vec![]))).expect("compile to python");
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "hi, world\n", "emitted python printed unexpected output");
        }
    }

    #[test]
    fn end_to_end_keyword_arg_supplied_executes_py() {
        // Execution-proof: `greet("hi", name="ada")` supplies the keyword, so
        // the default is suppressed → prints `hi, ada`.
        let call = greet_call(vec![Expr::KeywordArg {
            name: "name".into(),
            value: Box::new(Expr::StrLit { value: "ada".into(), span: s() }),
            span: s(),
        }]);
        let a = compile(&kw_module(call)).expect("compile to python");
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "hi, ada\n", "emitted python printed unexpected output");
        }
    }

    // ── O2: Ruby OOP end-to-end execution proofs ─────────────────────────
    //
    // Milestone O2 makes the Ruby frontend PRODUCE the OOP wiring — method
    // registration (`__def_method__`), construction (`__new__` → `initialize`),
    // `super` (`__super__`), `self` (`__self__`), and `attr_accessor`.  The O1
    // runtime + backend emit arms (already present) consume it.  These three
    // tests lower REAL Ruby source through `ruby-to-semantic-ir`, compile the
    // resulting SIR to Python, and run it under a real interpreter — the
    // payoff proof that object-oriented Ruby executes end to end
    // (Ruby → SIR → Python → CPython).
    //
    // They print with Ruby `print` rather than `puts`: `puts` is not in
    // `sir-runtime-core`'s native `call_builtin` dispatch table (a pre-existing,
    // OOP-unrelated backend coverage gap), while `print` is the natively-lowered
    // line writer.  Using `print` keeps the focus on the OOP mechanism.

    #[test]
    fn end_to_end_ruby_oop_new_and_method_dispatch_executes_py() {
        // P1 — construction + instance-method dispatch + `@ivar` through the
        // pushed self, with string interpolation in the method body:
        //   class Dog
        //     def initialize(name); @name = name; end
        //     def speak; "#{@name} says woof"; end
        //   end
        //   print Dog.new("Rex").speak     # => Rex says woof
        // Proves `__new__` runs `initialize` (setting @name on the new object),
        // `.speak` dispatches the registered instance method under a pushed
        // self, and the interpolation (`StrConcat`) works through the OOP path.
        let src = "class Dog\n  def initialize(name)\n    @name = name\n  end\n  \
                   def speak\n    \"#{@name} says woof\"\n  end\nend\n\
                   print Dog.new(\"Rex\").speak\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        // Shape: the registration + construction + dispatch are all present.
        assert!(
            a.source.contains("_sir_oop_def_method(\"Dog\", \"initialize\","),
            "initialize must be registered; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("_sir_oop_call_new(\"Dog\", \"Rex\")"),
            "Dog.new(\"Rex\") must lower to call_new; got:\n{}",
            a.source
        );
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "Rex says woof\n", "P1 OOP dispatch produced wrong output");
        }
    }

    #[test]
    fn end_to_end_ruby_inheritance_super_executes_py() {
        // P2 — inheritance + `super` + shared self:
        //   class Animal
        //     def initialize(name); @name = name; @legs = 4; end
        //   end
        //   class Cat < Animal
        //     def initialize(name); super(name); end
        //     def describe; "#{@name} with #{@legs} legs"; end
        //   end
        //   print Cat.new("Tom").describe   # => Tom with 4 legs
        // `Cat.new` runs `Cat#initialize`, which `super(name)`s into
        // `Animal#initialize` on the SAME self (setting @name/@legs), then
        // `.describe` reads them back.  Proves `__super__` threads the enclosing
        // method+class and re-dispatches on the parent with the receiver bound.
        let src = "class Animal\n  def initialize(name)\n    @name = name\n    @legs = 4\n  end\nend\n\
                   class Cat < Animal\n  def initialize(name)\n    super(name)\n  end\n  \
                   def describe\n    \"#{@name} with #{@legs} legs\"\n  end\nend\n\
                   print Cat.new(\"Tom\").describe\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("_sir_oop_call_super(\"initialize\", \"Cat\", name)"),
            "super(name) in Cat#initialize must lower to call_super; got:\n{}",
            a.source
        );
        // The two initializers hoist under distinct class-qualified names.
        assert!(
            a.source.contains("def Animal__initialize(") && a.source.contains("def Cat__initialize("),
            "parent + child initializers must be distinct functions; got:\n{}",
            a.source
        );
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "Tom with 4 legs\n", "P2 inheritance/super produced wrong output");
        }
    }

    #[test]
    fn end_to_end_ruby_attr_accessor_and_self_chain_executes_py() {
        // P3 — attr_accessor getter, `@ivar` mutation, and self-return chaining:
        //   class Counter
        //     attr_accessor :count
        //     def initialize; @count = 0; end
        //     def inc; @count = @count + 1; self; end
        //   end
        //   c = Counter.new
        //   c.inc.inc
        //   print c.count                  # => 2
        // Proves the synthesized `count` getter reads @count, `inc` mutates it
        // and returns `self` (so `c.inc.inc` chains on the same object), and the
        // final `c.count` dispatches the accessor.
        let src = "class Counter\n  attr_accessor :count\n  def initialize\n    @count = 0\n  end\n  \
                   def inc\n    @count = @count + 1\n    self\n  end\nend\n\
                   c = Counter.new\nc.inc.inc\nprint c.count\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        // The synthesized getter is registered under the bare `count`.
        assert!(
            a.source.contains("_sir_oop_def_method(\"Counter\", \"count\","),
            "attr_accessor must register a `count` getter; got:\n{}",
            a.source
        );
        // `self` in `inc` lowers to the current-self builtin.
        assert!(
            a.source.contains("_sir_oop_current_self()"),
            "self must lower to current_self(); got:\n{}",
            a.source
        );
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "2\n", "P3 attr_accessor/self-chain produced wrong output");
        }
    }

    // ── MX2: mixins (include / extend / MRO) end-to-end ──────────────────────
    //
    // These lower REAL Ruby `module`/`include`/`extend` through
    // `ruby-to-semantic-ir` (MX1, already merged), compile the SIR to Python, and
    // run it under CPython — proving the mixin mechanism executes end to end
    // (Ruby → SIR → Python → CPython) through the new `_sir_oop_include_module` /
    // `_sir_oop_extend_module` arms and the MRO walk in the OOP runtime.

    #[test]
    fn end_to_end_ruby_include_module_method_executes_py() {
        // A module instance method mixed into a class, called on an instance.
        //   module Greetable
        //     def greet; "hi"; end
        //   end
        //   class Robot
        //     include Greetable
        //   end
        //   print Robot.new.greet          # => hi
        let src = "module Greetable\n  def greet\n    \"hi\"\n  end\nend\n\
                   class Robot\n  include Greetable\nend\n\
                   print Robot.new.greet\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("_sir_oop_def_method(\"Greetable\", \"greet\","),
            "module body def must register under the module owner; got:\n{}",
            a.source
        );
        assert!(
            a.source.contains("_sir_oop_include_module(\"Robot\", \"Greetable\")"),
            "include Greetable must lower to include_module; got:\n{}",
            a.source
        );
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "hi\n", "MX2 include produced wrong output");
        }
    }

    #[test]
    fn end_to_end_ruby_class_method_shadows_included_module_executes_py() {
        // The class's own method wins over the included module's (class-first MRO).
        //   module Nameable
        //     def name; "module"; end
        //   end
        //   class Widget
        //     include Nameable
        //     def name; "class"; end
        //   end
        //   print Widget.new.name          # => class
        let src = "module Nameable\n  def name\n    \"module\"\n  end\nend\n\
                   class Widget\n  include Nameable\n  def name\n    \"class\"\n  end\nend\n\
                   print Widget.new.name\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "class\n", "MX2 class-shadows-module produced wrong output");
        }
    }

    #[test]
    fn end_to_end_ruby_extend_module_class_method_executes_py() {
        // extend mixes a module's instance methods in as class/singleton methods.
        //   module Counting
        //     def count; 7; end
        //   end
        //   class Widget
        //     extend Counting
        //   end
        //   print Widget.count             # => 7
        let src = "module Counting\n  def count\n    7\n  end\nend\n\
                   class Widget\n  extend Counting\nend\n\
                   print Widget.count\n";
        let module = ruby_to_semantic_ir::compile_source(src, "demo").expect("lower ruby");
        let a = compile(&module).expect("compile to python");
        assert!(
            a.source.contains("_sir_oop_extend_module(\"Widget\", \"Counting\")"),
            "extend Counting must lower to extend_module; got:\n{}",
            a.source
        );
        if let Some(stdout) = run_emitted_python(&a.source) {
            assert_eq!(stdout, "7\n", "MX2 extend produced wrong output");
        }
    }
}
