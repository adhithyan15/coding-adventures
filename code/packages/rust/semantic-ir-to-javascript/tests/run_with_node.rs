//! End-to-end integration test: Twig source → SIR → JavaScript → `node`.
//!
//! The whole point of this backend is *self-contained* JavaScript that
//! runs as-is.  Unit tests prove the emitted *shape*; this test proves
//! the emitted *behaviour* by actually executing the artifact under
//! Node.js and comparing stdout.
//!
//! Node is optional at test time.  When `node --version` does not
//! succeed (CI image without Node, locked-down sandbox, …) the test
//! degrades to the syntactic checks and skips execution, printing a
//! note rather than failing — mirroring the spec's "without `node`, the
//! syntactic tests still verify the output shape".

use std::path::PathBuf;
use std::process::Command;

use semantic_ir::nodes::MapEntry;
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope, Span, Stmt,
};
use semantic_ir_to_javascript::compile;

/// Is a working `node` on PATH?
fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── hand-built SIR helpers ─────────────────────────────────────────────
//
// The Twig frontend is a Lisp dialect and does not yet produce the SIR16
// nodes (sequences, maps, loops, mutation, short-circuit), so the SIR16
// behaviour tests construct SIR modules directly.  Each helper keeps the
// call sites in the tests terse.

fn sp() -> Span {
    Span::synthetic()
}

fn int(v: i64) -> Expr {
    Expr::IntLit { value: v, span: sp() }
}

fn float(v: f64) -> Expr {
    Expr::FloatLit { value: v, span: sp() }
}

fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: sp() }
}

fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: sp() }
}

fn print(arg: Expr) -> Stmt {
    Stmt::ExprStmt { expr: bc("print", vec![arg]), span: sp() }
}

fn let_(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding { name: name.into(), sir_type: None, value, span: sp() }
}

/// Wrap a `main` function (the `stmts` run for effect, `value` is its
/// return) into a complete, SIR16-flagged module ready for `compile`.
fn module_with_main(stmts: Vec<Stmt>, value: Expr, features: &[Feature]) -> Module {
    Module {
        name: "sir16".into(),
        manifest: FeatureManifest::from_features(features),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value, span: sp() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        }],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    }
}

/// Compile a hand-built module, run it under `node`, and return stdout
/// (trailing newlines trimmed).  Returns `None` when Node is unavailable.
fn run_module(module: &Module, tag: &str) -> Option<String> {
    let artifact = compile(module).expect("compile to javascript");
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_js_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &artifact.source).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero for `{tag}`:\nstdout: {}\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        artifact.source,
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    Some(stdout.trim_end_matches(['\n', '\r']).to_string())
}

/// Compile Twig `src` to a `.js` file in a unique temp path, run it with
/// `node`, and return its stdout (trailing newline trimmed).  Returns
/// `None` when Node is unavailable (caller should skip the assertion).
fn emit_and_run(src: &str, module_name: &str, tag: &str) -> Option<String> {
    let module = twig_to_semantic_ir::compile_source(src, module_name).expect("lower twig");
    let artifact = compile(&module).expect("compile to javascript");

    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }

    // Unique path per process so parallel test runs never collide.
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_js_{}_{}_{}.js", tag, std::process::id(), module_name));
    std::fs::write(&path, &artifact.source).expect("write temp js");

    let output = Command::new("node")
        .arg(&path)
        .output()
        .expect("spawn node");

    // Best-effort cleanup; a leftover temp file is harmless.
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node exited non-zero for `{tag}`:\nstdout: {}\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        artifact.source,
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    Some(stdout.trim_end_matches(['\n', '\r']).to_string())
}

#[test]
fn add_program_prints_three() {
    // (define (add a b) (+ a b)) ; (print (add 1 2)) → 3
    let out = emit_and_run("(define (add a b) (+ a b))\n(print (add 1 2))", "addprog", "add");
    if let Some(stdout) = out {
        assert_eq!(stdout, "3", "expected 3 from add(1, 2)");
    }
}

#[test]
fn factorial_program_prints_120() {
    let src = "(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))\n(print (fact 5))";
    let out = emit_and_run(src, "factprog", "fact");
    if let Some(stdout) = out {
        assert_eq!(stdout, "120", "expected 5! = 120");
    }
}

#[test]
fn closure_adder_program_prints_eight() {
    // A higher-order program: `adder` returns a closure capturing `n`;
    // `add5` is the closure; `(add5 3)` applies it → 8.  Exercises
    // MakeClosure (capture), the global init, and applyClosure together.
    let src =
        "(define (adder n) (lambda (x) (+ x n)))\n(define add5 (adder 5))\n(print (add5 3))";
    let out = emit_and_run(src, "closprog", "closure");
    if let Some(stdout) = out {
        assert_eq!(stdout, "8", "expected add5(3) = 8");
    }
}

// ── SIR16 behaviour tests (hand-built modules) ─────────────────────────

#[test]
fn floats_arithmetic_promotion_prints_3_5() {
    // print(1 + 2.5) → 3.5 (int/float mix promotes to float natively).
    let module = module_with_main(
        vec![print(bc("+", vec![int(1), float(2.5)]))],
        Expr::NilLit { span: sp() },
        &[Feature::Floats],
    );
    if let Some(stdout) = run_module(&module, "floats") {
        assert_eq!(stdout, "3.5");
    }
}

#[test]
fn short_circuit_does_not_evaluate_rhs() {
    // (false && <print "boom">) must NOT print "boom"; the whole
    // expression prints #f.  Routing through truthy keeps `false` falsy.
    let and = Expr::LogicalAnd {
        lhs: Box::new(Expr::BoolLit { value: false, span: sp() }),
        rhs: Box::new(Expr::Block(Box::new(Block {
            stmts: vec![print(Expr::StrLit { value: "boom".into(), span: sp() })],
            value: int(99),
            span: sp(),
        }))),
        span: sp(),
    };
    let module = module_with_main(
        vec![print(and)],
        Expr::NilLit { span: sp() },
        &[Feature::ShortCircuit, Feature::Strings],
    );
    if let Some(stdout) = run_module(&module, "shortcircuit") {
        // Only `#f` printed — the rhs block (which would print "boom")
        // never ran.
        assert_eq!(stdout, "#f", "rhs must not be evaluated");
    }
}

#[test]
fn short_circuit_or_returns_first_truthy() {
    // (false || 7) → 7.
    let or = Expr::LogicalOr {
        lhs: Box::new(Expr::BoolLit { value: false, span: sp() }),
        rhs: Box::new(int(7)),
        span: sp(),
    };
    let module = module_with_main(
        vec![print(or)],
        Expr::NilLit { span: sp() },
        &[Feature::ShortCircuit],
    );
    if let Some(stdout) = run_module(&module, "or") {
        assert_eq!(stdout, "7");
    }
}

#[test]
fn sequence_build_index_len_set() {
    // let xs = [10, 20, 30];
    // xs[1] = 99;
    // print(xs[1]); print(len(xs));   → 99 then 3
    let stmts = vec![
        let_("xs", Expr::SeqLit { items: vec![int(10), int(20), int(30)], span: sp() }),
        Stmt::SeqSet { seq: local("xs"), index: int(1), value: int(99), span: sp() },
        print(Expr::SeqIndex {
            seq: Box::new(local("xs")),
            index: Box::new(int(1)),
            span: sp(),
        }),
        print(Expr::SeqLen { seq: Box::new(local("xs")), span: sp() }),
    ];
    let module = module_with_main(stmts, Expr::NilLit { span: sp() }, &[Feature::Sequences]);
    if let Some(stdout) = run_module(&module, "sequence") {
        assert_eq!(stdout, "99\n3");
    }
}

#[test]
fn map_build_get_set() {
    // let m = {"a": 1};
    // m["b"] = 2;
    // print(m["a"]); print(m["b"]); print(m["missing"]);  → 1, 2, nil
    let stmts = vec![
        let_(
            "m",
            Expr::MapLit {
                entries: vec![MapEntry {
                    key: Expr::StrLit { value: "a".into(), span: sp() },
                    value: int(1),
                }],
                span: sp(),
            },
        ),
        Stmt::MapSet {
            map: local("m"),
            key: Expr::StrLit { value: "b".into(), span: sp() },
            value: int(2),
            span: sp(),
        },
        print(Expr::MapGet {
            map: Box::new(local("m")),
            key: Box::new(Expr::StrLit { value: "a".into(), span: sp() }),
            span: sp(),
        }),
        print(Expr::MapGet {
            map: Box::new(local("m")),
            key: Box::new(Expr::StrLit { value: "b".into(), span: sp() }),
            span: sp(),
        }),
        print(Expr::MapGet {
            map: Box::new(local("m")),
            key: Box::new(Expr::StrLit { value: "missing".into(), span: sp() }),
            span: sp(),
        }),
    ];
    let module =
        module_with_main(stmts, Expr::NilLit { span: sp() }, &[Feature::Maps, Feature::Strings]);
    if let Some(stdout) = run_module(&module, "map") {
        // A missing key reads as nil (`null` → "nil" via format).
        assert_eq!(stdout, "1\n2\nnil");
    }
}

#[test]
fn while_loop_counts_to_three() {
    // let i = 0; while (i < 3) { print(i); i = i + 1; }  → 0,1,2
    let stmts = vec![
        let_("i", int(0)),
        Stmt::While {
            cond: bc("<", vec![local("i"), int(3)]),
            body: Block {
                stmts: vec![
                    print(local("i")),
                    Stmt::Assign {
                        name: "i".into(),
                        scope: Scope::Local,
                        value: bc("+", vec![local("i"), int(1)]),
                        span: sp(),
                    },
                ],
                value: Expr::NilLit { span: sp() },
                span: sp(),
            },
            span: sp(),
        },
    ];
    let module = module_with_main(
        stmts,
        Expr::NilLit { span: sp() },
        &[Feature::Loops, Feature::MutableBindings],
    );
    if let Some(stdout) = run_module(&module, "while") {
        assert_eq!(stdout, "0\n1\n2");
    }
}

#[test]
fn for_range_accumulator_sums_to_ten() {
    // let sum = 0; for i in range(0, 5, 1) { sum = sum + i; }  print(sum) → 10
    let stmts = vec![
        let_("sum", int(0)),
        Stmt::ForRange {
            var: "i".into(),
            start: int(0),
            stop: int(5),
            step: int(1),
            body: Block {
                stmts: vec![Stmt::Assign {
                    name: "sum".into(),
                    scope: Scope::Local,
                    value: bc("+", vec![local("sum"), local("i")]),
                    span: sp(),
                }],
                value: Expr::NilLit { span: sp() },
                span: sp(),
            },
            span: sp(),
        },
        print(local("sum")),
    ];
    let module = module_with_main(
        stmts,
        Expr::NilLit { span: sp() },
        &[Feature::Loops, Feature::MutableBindings],
    );
    if let Some(stdout) = run_module(&module, "forrange") {
        assert_eq!(stdout, "10");
    }
}

#[test]
fn for_range_descending_step_counts_down() {
    // for i in range(3, 0, -1) { print(i); }  → 3,2,1
    let stmts = vec![Stmt::ForRange {
        var: "i".into(),
        start: int(3),
        stop: int(0),
        step: int(-1),
        body: Block {
            stmts: vec![print(local("i"))],
            value: Expr::NilLit { span: sp() },
            span: sp(),
        },
        span: sp(),
    }];
    let module = module_with_main(stmts, Expr::NilLit { span: sp() }, &[Feature::Loops]);
    if let Some(stdout) = run_module(&module, "forrangedown") {
        assert_eq!(stdout, "3\n2\n1");
    }
}

#[test]
fn for_each_over_sequence() {
    // for x in [4, 5, 6] { print(x); }  → 4,5,6
    let stmts = vec![Stmt::ForEach {
        var: "x".into(),
        iter: Expr::SeqLit { items: vec![int(4), int(5), int(6)], span: sp() },
        body: Block {
            stmts: vec![print(local("x"))],
            value: Expr::NilLit { span: sp() },
            span: sp(),
        },
        span: sp(),
    }];
    let module = module_with_main(
        stmts,
        Expr::NilLit { span: sp() },
        &[Feature::Loops, Feature::Sequences],
    );
    if let Some(stdout) = run_module(&module, "foreach") {
        assert_eq!(stdout, "4\n5\n6");
    }
}

// ── P2d: default parameters (hand-built module) ────────────────────────

#[test]
fn default_param_is_call_time_and_param_scoped() {
    // The discriminating test for P2d.  Build a module with:
    //
    //   function f(a, b = a + 1) { return a + b; }
    //   function main() { print(f(5)); print(f(5, 10)); }
    //
    // and run it under node.  `f(5)` omits `b`, so the native JS default
    // fires: `b = a + 1 = 6`, and `f` returns `a + b = 5 + 6 = 11`?  No —
    // the test prints `b` itself, not `a + b`: the body returns `b` so the
    // call's *value* is the bound `b`.  We use that to read the default
    // directly:
    //
    //   f(5)      → b defaults to a + 1 = 6   → prints 6
    //   f(5, 10)  → b is supplied as 10       → prints 10
    //
    // Printing 6 then 10 proves the default is (a) evaluated at call time
    // — it depends on the actual argument `a = 5`, not on a compile-time
    // constant — AND (b) evaluated in param scope, since it references the
    // earlier param `a` by name.
    use semantic_ir::Param;

    fn param(name: &str) -> Param {
        Param {
            name: name.into(),
            sir_type: None,
            kind: semantic_ir::ParamKind::Required,
            default: None,
            span: sp(),
        }
    }

    fn param_ref(name: &str) -> Expr {
        Expr::VarRef { name: name.into(), scope: Scope::Param, span: sp() }
    }

    // b's default = (a + 1), referencing the earlier param `a`.
    let b_default = bc("+", vec![param_ref("a"), int(1)]);

    let f = Function {
        name: "f".into(),
        params: vec![
            param("a"),
            Param {
                name: "b".into(),
                sir_type: None,
                kind: semantic_ir::ParamKind::Required,
                default: Some(Box::new(b_default)),
                span: sp(),
            },
        ],
        return_type: None,
        captures: vec![],
        // Body returns `b` so the printed value IS the (possibly defaulted)
        // second parameter.
        body: Block { stmts: vec![], value: param_ref("b"), span: sp() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };

    let call_one_arg = Expr::DirectCall {
        fn_name: "f".into(),
        args: vec![int(5)],
        effects: EffectSet::PURE,
        span: sp(),
    };
    let call_two_args = Expr::DirectCall {
        fn_name: "f".into(),
        args: vec![int(5), int(10)],
        effects: EffectSet::PURE,
        span: sp(),
    };

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![print(call_one_arg), print(call_two_args)],
            value: Expr::NilLit { span: sp() },
            span: sp(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };

    let module = Module {
        name: "defaultparams".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::DefaultParams,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![f, main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    };

    // Shape check first (runs even without node): the emitted callee carries
    // a native JS default referencing the earlier param, and the one-arg
    // call is NOT padded.
    let artifact = compile(&module).expect("compile to javascript");
    assert!(
        artifact.source.contains("function f(a, b = (a + 1)) {"),
        "expected native JS default param, got:\n{}",
        artifact.source
    );
    assert!(artifact.source.contains("__Sir.print(f(5))"), "got:\n{}", artifact.source);
    assert!(artifact.source.contains("__Sir.print(f(5, 10))"), "got:\n{}", artifact.source);

    if let Some(stdout) = run_module(&module, "defaultparams") {
        assert_eq!(stdout, "6\n10", "f(5) must default b to a+1=6; f(5,10) must use 10");
    }
}

#[test]
fn mutable_reassignment_updates_binding() {
    // let x = 1; x = 2; x = x + 40; print(x);  → 42
    let stmts = vec![
        let_("x", int(1)),
        Stmt::Assign { name: "x".into(), scope: Scope::Local, value: int(2), span: sp() },
        Stmt::Assign {
            name: "x".into(),
            scope: Scope::Local,
            value: bc("+", vec![local("x"), int(40)]),
            span: sp(),
        },
        print(local("x")),
    ];
    let module = module_with_main(
        stmts,
        Expr::NilLit { span: sp() },
        &[Feature::MutableBindings],
    );
    if let Some(stdout) = run_module(&module, "mutable") {
        assert_eq!(stdout, "42");
    }
}

#[test]
fn keyword_params_options_object_omitted_and_supplied() {
    // KW4 discriminating execution-proof.  Build a module with a function
    // that has one positional and one *optional* keyword param:
    //
    //   function add(base, __kw) { const { delta = 10 } = __kw ?? {};
    //                              return base + delta; }
    //   function main() { print(add(5)); print(add(5, delta: 100)); }
    //
    // - `add(5)`             omits the keyword → default 10 fires → 15.
    // - `add(5, delta: 100)` supplies it       → 100 used         → 105.
    //
    // Printing 15 then 105 proves (a) the callee destructures `__kw` and its
    // JS default fills an omitted keyword, and (b) a supplied `KeywordArg`
    // collapses into the trailing options object and overrides the default.
    use semantic_ir::{Param, ParamKind};

    let base = Param {
        name: "base".into(),
        sir_type: None,
        kind: ParamKind::Required,
        default: None,
        span: sp(),
    };
    let delta = Param {
        name: "delta".into(),
        sir_type: None,
        kind: ParamKind::Keyword,
        default: Some(Box::new(int(10))),
        span: sp(),
    };
    let param_ref = |n: &str| Expr::VarRef { name: n.into(), scope: Scope::Param, span: sp() };

    let add = Function {
        name: "add".into(),
        params: vec![base, delta],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![],
            value: bc("+", vec![param_ref("base"), param_ref("delta")]),
            span: sp(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };

    let kw_arg = |name: &str, value: Expr| Expr::KeywordArg {
        name: name.into(),
        value: Box::new(value),
        span: sp(),
    };
    let call_omitted = Expr::DirectCall {
        fn_name: "add".into(),
        args: vec![int(5)],
        effects: EffectSet::PURE,
        span: sp(),
    };
    let call_supplied = Expr::DirectCall {
        fn_name: "add".into(),
        args: vec![int(5), kw_arg("delta", int(100))],
        effects: EffectSet::PURE,
        span: sp(),
    };

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![print(call_omitted), print(call_supplied)],
            value: Expr::NilLit { span: sp() },
            span: sp(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };

    let module = Module {
        name: "kwparams".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::KeywordParams,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![add, main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    };

    // Shape check first (runs even without node): the callee folds the
    // keyword into a trailing `__kw` object, destructures it with the JS
    // default, and the supplied call collapses the keyword into an object.
    let artifact = compile(&module).expect("compile to javascript");
    assert!(
        artifact.source.contains("function add(base, __kw) {"),
        "expected trailing __kw options object, got:\n{}",
        artifact.source
    );
    assert!(
        artifact.source.contains("const { delta = 10 } = __kw ?? {};"),
        "expected keyword destructuring prologue, got:\n{}",
        artifact.source
    );
    assert!(
        artifact.source.contains("__Sir.print(add(5))"),
        "omitted-keyword call should have no options object, got:\n{}",
        artifact.source
    );
    assert!(
        artifact.source.contains("__Sir.print(add(5, { delta: 100 }))"),
        "supplied keyword should collapse into a trailing object, got:\n{}",
        artifact.source
    );

    if let Some(stdout) = run_module(&module, "kwparams") {
        assert_eq!(
            stdout, "15\n105",
            "add(5) must default delta to 10 (→15); add(5, delta: 100) must use 100 (→105)"
        );
    }
}

#[test]
fn required_keyword_param_supplied_by_call() {
    // A *required* keyword (no default) must destructure bare and be filled
    // by the caller.  Mirrors the spec's `greet(greeting:, …)` shape:
    //
    //   function pick(__kw) { const { chosen } = __kw ?? {}; return chosen; }
    //   function main() { print(pick(chosen: 7)); }   → 7
    use semantic_ir::{Param, ParamKind};

    let chosen = Param {
        name: "chosen".into(),
        sir_type: None,
        kind: ParamKind::Keyword,
        default: None,
        span: sp(),
    };
    let pick = Function {
        name: "pick".into(),
        params: vec![chosen],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![],
            value: Expr::VarRef { name: "chosen".into(), scope: Scope::Param, span: sp() },
            span: sp(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };
    let call = Expr::DirectCall {
        fn_name: "pick".into(),
        args: vec![Expr::KeywordArg {
            name: "chosen".into(),
            value: Box::new(int(7)),
            span: sp(),
        }],
        effects: EffectSet::PURE,
        span: sp(),
    };
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![print(call)], value: Expr::NilLit { span: sp() }, span: sp() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };
    let module = Module {
        name: "kwreq".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::KeywordParams,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![pick, main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    };

    let artifact = compile(&module).expect("compile to javascript");
    assert!(
        artifact.source.contains("function pick(__kw) {"),
        "keyword-only function should have __kw as its sole param, got:\n{}",
        artifact.source
    );
    assert!(
        artifact.source.contains("const { chosen } = __kw ?? {};"),
        "required keyword destructures bare (no default), got:\n{}",
        artifact.source
    );

    if let Some(stdout) = run_module(&module, "kwreq") {
        assert_eq!(stdout, "7", "pick(chosen: 7) must return 7");
    }
}

// ── SECURITY: runtime `callMethod` allowlist blocks the RCE gadget ─────
//
// `callMethod(recv, name, …)` performs a dynamic `recv[name]` lookup with an
// attacker-controlled `name`.  Without a gate, `name = "constructor"` on a
// function receiver yields the global `Function` constructor and lets a
// translated untrusted program synthesise and run arbitrary code — a remote
// code-execution hole.  The runtime now dispatches only through a fixed
// allowlist of safe collection/String/Number methods; anything else throws a
// `TypeError` *before* the lookup.  This test builds a module that emits
// `__Sir.callMethod(fn, "constructor", "…")` directly (bypassing the frontend
// denylist, to exercise the runtime's own gate) and asserts node throws
// rather than executing the payload.

/// Compile + run a module expecting node to FAIL (non-zero exit).  Returns
/// the captured stderr so the caller can assert on the thrown message.
/// Returns `None` when node is unavailable.
fn run_module_expecting_failure(module: &Module, tag: &str) -> Option<String> {
    let artifact = compile(module).expect("compile to javascript");
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_js_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &artifact.source).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        !output.status.success(),
        "SECURITY: node UNEXPECTEDLY SUCCEEDED for `{tag}` — the gadget was \
         not blocked!\nstdout: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        artifact.source,
    );
    Some(String::from_utf8_lossy(&output.stderr).to_string())
}

#[test]
fn runtime_rejects_constructor_gadget() {
    // Build:  function id(x) { return x; }
    //         function main() { print(callMethod(id, "constructor", "return 1")); }
    // where `callMethod(id, "constructor", …)` is the raw __method__ envelope.
    // Unguarded, `id["constructor"]` is `Function`, and invoking it would
    // build+run code.  The allowlist must reject "constructor" with a
    // TypeError so node exits non-zero.
    use semantic_ir::Param;

    let id = Function {
        name: "id".into(),
        params: vec![Param {
            name: "x".into(),
            sir_type: None,
            kind: semantic_ir::ParamKind::Required,
            default: None,
            span: sp(),
        }],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![],
            value: Expr::VarRef { name: "x".into(), scope: Scope::Param, span: sp() },
            span: sp(),
        },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };

    // The raw method-dispatch envelope: BuiltinCall("__method__",
    // [receiver, "constructor", payload]) → __Sir.callMethod(id, "constructor",
    // "return 1").  `id` is referenced as a global function handle.
    let gadget = bc(
        "__method__",
        vec![
            Expr::VarRef { name: "id".into(), scope: Scope::Global, span: sp() },
            Expr::StrLit { value: "constructor".into(), span: sp() },
            Expr::StrLit { value: "return 1".into(), span: sp() },
        ],
    );

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![print(gadget)], value: Expr::NilLit { span: sp() }, span: sp() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };

    let module = Module {
        name: "rce_gadget".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Strings,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![id, main],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    };

    // Shape check (runs without node): the emitted call routes to callMethod
    // with the dangerous name — proving we are genuinely exercising the gate.
    let artifact = compile(&module).expect("compile to javascript");
    assert!(
        artifact.source.contains(r#"__Sir.callMethod(id, "constructor", "return 1")"#),
        "expected the raw constructor gadget in emitted source, got:\n{}",
        artifact.source
    );

    if let Some(stderr) = run_module_expecting_failure(&module, "rce_gadget") {
        assert!(
            stderr.contains("not an allowed collection method"),
            "expected the allowlist TypeError, got stderr:\n{stderr}"
        );
    }
}
