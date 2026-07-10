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
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, RescueClause,
    Scope, Span, Stmt,
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
        artifact.source.contains("function f(a, b = __Sir.plus(a, 1)) {"),
        "expected native JS default param (with polymorphic `+`), got:\n{}",
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
        // The gadget name is not on the allowlist, so — as with any unknown
        // method (T3) — it is rejected with a typed `NoMethodError` *before*
        // any `recv[name]` lookup.  The load-bearing property is that the
        // `"return 1"` payload was NEVER synthesised/executed (node threw
        // instead of printing a result), and the miss surfaces as our
        // NoMethodError, not an executed host `Function` payload.
        assert!(
            stderr.contains("NoMethodError") || stderr.contains("undefined method"),
            "expected the allowlist rejection (NoMethodError), got stderr:\n{stderr}"
        );
    }
}

// ── E1: exception execution-proof (run under `node`) ───────────────────
//
// The unit tests in `emit.rs` prove the emitted *shape*; these prove the
// emitted *behaviour* by compiling a hand-built SIR module and running the
// self-contained `.js` under Node, comparing stdout (or, for the
// re-raise case, asserting a non-zero exit).

/// A string literal expression.
fn str_(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: sp() }
}

/// A `Const`-scoped var-ref — how the Ruby frontend spells a bare class
/// name like `ArgumentError` at a `raise` site.
fn const_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Const, span: sp() }
}

/// A `raise Class, "msg"` statement.
fn raise(class: &str, msg: &str) -> Stmt {
    Stmt::ExprStmt { expr: bc("raise", vec![const_ref(class), str_(msg)]), span: sp() }
}

/// (a) Built-in ancestry: `begin; raise ArgumentError, "x"; rescue
/// StandardError => e; puts "caught"; end` → "caught".  `ArgumentError`
/// chains up to `StandardError` via the baked-in ancestry table.
#[test]
fn try_catch_builtin_ancestry_catches() {
    let try_catch = Stmt::TryCatch {
        body: vec![raise("ArgumentError", "x")],
        rescues: vec![RescueClause {
            exception_types: vec!["StandardError".into()],
            binding: Some("e".into()),
            body: vec![print(str_("caught"))],
            span: sp(),
        }],
        ensure_body: None,
        span: sp(),
    };
    let module = module_with_main(
        vec![try_catch],
        Expr::NilLit { span: sp() },
        &[Feature::Exceptions, Feature::Constants, Feature::Strings],
    );
    if let Some(stdout) = run_module(&module, "exc_builtin") {
        assert_eq!(stdout, "caught");
    }
}

/// (b) A bare `rescue` (no exception types) is a catch-all: it must catch
/// a `raise RuntimeError, "y"` and print "rescued".
#[test]
fn bare_rescue_catches_anything() {
    let try_catch = Stmt::TryCatch {
        body: vec![raise("RuntimeError", "y")],
        rescues: vec![RescueClause {
            exception_types: vec![], // bare `rescue`
            binding: None,
            body: vec![print(str_("rescued"))],
            span: sp(),
        }],
        ensure_body: None,
        span: sp(),
    };
    let module = module_with_main(
        vec![try_catch],
        Expr::NilLit { span: sp() },
        &[Feature::Exceptions, Feature::Constants, Feature::Strings],
    );
    if let Some(stdout) = run_module(&module, "exc_bare") {
        assert_eq!(stdout, "rescued");
    }
}

/// (c) An unmatched rescue type must NOT catch: `raise TypeError` under a
/// `rescue ArgumentError` re-raises past the inner handler, so the program
/// exits non-zero (uncaught exception escapes to node).
#[test]
fn unmatched_rescue_type_reraises() {
    let try_catch = Stmt::TryCatch {
        body: vec![raise("TypeError", "nope")],
        rescues: vec![RescueClause {
            exception_types: vec!["ArgumentError".into()],
            binding: None,
            body: vec![print(str_("should-not-print"))],
            span: sp(),
        }],
        ensure_body: None,
        span: sp(),
    };
    let module = module_with_main(
        vec![try_catch],
        Expr::NilLit { span: sp() },
        &[Feature::Exceptions, Feature::Constants, Feature::Strings],
    );
    if let Some(stderr) = run_module_expecting_failure(&module, "exc_reraise") {
        // The escaping exception is our TypeError, and the inner handler's
        // line never ran.
        assert!(stderr.contains("TypeError") || stderr.contains("nope"), "stderr:\n{stderr}");
        assert!(!stderr.contains("should-not-print"));
    }
}

/// (d) USER ancestry (E2): `class MyErr < StandardError; …; begin; raise
/// MyErr, "z"; rescue StandardError => e; puts "user-caught"; end` →
/// "user-caught".  The class edge is registered at init via
/// `__Sir.registerAncestry`, so `MyErr` chains up to `StandardError`.
#[test]
fn try_catch_user_ancestry_catches() {
    let class_def = Stmt::ClassDef {
        name: "MyErr".into(),
        superclass: Some("StandardError".into()),
        body: vec![],
        span: sp(),
    };
    let try_catch = Stmt::TryCatch {
        body: vec![raise("MyErr", "z")],
        rescues: vec![RescueClause {
            exception_types: vec!["StandardError".into()],
            binding: Some("e".into()),
            body: vec![print(str_("user-caught"))],
            span: sp(),
        }],
        ensure_body: None,
        span: sp(),
    };
    let module = module_with_main(
        vec![class_def, try_catch],
        Expr::NilLit { span: sp() },
        &[Feature::Exceptions, Feature::Classes, Feature::Constants, Feature::Strings],
    );
    // Shape check (runs without node): the user edge is registered once.
    let artifact = compile(&module).expect("compile");
    assert!(
        artifact.source.contains(r#"__Sir.registerAncestry({ "MyErr": "StandardError" });"#),
        "expected user ancestry registration, got:\n{}",
        artifact.source
    );
    if let Some(stdout) = run_module(&module, "exc_user_ancestry") {
        assert_eq!(stdout, "user-caught");
    }
}

// ── O3: user-defined-class OOP execution-proof (run under `node`) ──────
//
// The unit tests in `emit.rs` prove the emitted OOP *shape*; these prove
// the emitted *behaviour* by hand-building an SIR module the way the O2
// Ruby frontend will (method bodies hoisted to top-level functions,
// registered with `__def_method__`, instantiated with `__new__`, `super`
// via `__super__`, `@ivar` via `Scope::Instance`), compiling to
// self-contained JS, and running it under Node.

/// A required parameter named `n`.
fn param(n: &str) -> semantic_ir::Param {
    semantic_ir::Param {
        name: n.into(),
        sir_type: None,
        kind: semantic_ir::ParamKind::Required,
        default: None,
        span: sp(),
    }
}

/// A top-level function `name(params…) { stmts…; return value }` — the
/// shape the frontend hoists a method body into.
fn func(name: &str, params: Vec<semantic_ir::Param>, stmts: Vec<Stmt>, value: Expr) -> Function {
    Function {
        name: name.into(),
        params,
        return_type: None,
        captures: vec![],
        body: Block { stmts, value, span: sp() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    }
}

/// `new __Sir.Closure(...)` wrapping the top-level method function
/// `fn_name` with no captures — how the frontend passes a method body to
/// `__def_method__`.
fn method_closure(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: vec![], span: sp() }
}

/// `Class.def(name, <closure>)` → `__def_method__("Class","name",closure)`.
fn def_method(cls: &str, name: &str, fn_name: &str) -> Stmt {
    Stmt::ExprStmt {
        expr: bc("__def_method__", vec![str_(cls), str_(name), method_closure(fn_name)]),
        span: sp(),
    }
}

/// An instance-variable read (`@x`) / write.
fn ivar(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Instance, span: sp() }
}
fn ivar_set(name: &str, value: Expr) -> Stmt {
    Stmt::Assign { name: name.into(), scope: Scope::Instance, value, span: sp() }
}
fn param_ref(n: &str) -> Expr {
    Expr::VarRef { name: n.into(), scope: Scope::Param, span: sp() }
}

/// Assemble a module from method-body functions + a `main`, with the OOP
/// feature set flagged.
fn oop_module(methods: Vec<Function>, main_stmts: Vec<Stmt>) -> Module {
    let mut functions = methods;
    functions.push(func("main", vec![], main_stmts, Expr::NilLit { span: sp() }));
    Module {
        name: "oop".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Classes,
            Feature::InstanceVars,
            Feature::ClassVars,
            Feature::Closures,
            Feature::Strings,
            Feature::DynamicTyping,
            Feature::Constants,
            // M6 tests pass a `send`/`respond_to?` name as a `SymLit`.
            Feature::Symbols,
            // `@x = v` lowers to an `Assign`, which the validator counts
            // as a mutable binding.
            Feature::MutableBindings,
        ]),
        imports: vec![],
        exports: vec![],
        functions,
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    }
}

/// P1 — instantiation + instance method + `@ivar`:
///   class Dog; def initialize(name); @name = name; end
///             def speak; print(@name + " says woof"); end; end
///   Dog.new("Rex").speak   →   "Rex says woof"
#[test]
fn p1_dog_initialize_and_speak() {
    // def initialize(name); @name = name; end
    let dog_init = func(
        "Dog__initialize",
        vec![param("name")],
        vec![ivar_set("@name", param_ref("name"))],
        Expr::NilLit { span: sp() },
    );
    // def speak; print(@name + " says woof"); end
    let dog_speak = func(
        "Dog__speak",
        vec![],
        vec![print(bc("+", vec![ivar("@name"), str_(" says woof")]))],
        Expr::NilLit { span: sp() },
    );
    // main: register the methods, then `Dog.new("Rex").speak`.
    let make = bc("__new__", vec![str_("Dog"), str_("Rex")]);
    let speak = bc("__method__", vec![make, str_("speak")]);
    let main = vec![
        def_method("Dog", "initialize", "Dog__initialize"),
        def_method("Dog", "speak", "Dog__speak"),
        Stmt::ExprStmt { expr: speak, span: sp() },
    ];
    let module = oop_module(vec![dog_init, dog_speak], main);

    // Shape: the OOP builtins routed to the runtime helpers.
    let artifact = compile(&module).expect("compile");
    assert!(artifact.source.contains(r#"__Sir.callNew("Dog", "Rex")"#), "{}", artifact.source);
    assert!(artifact.source.contains(r#"__Sir.defMethod("Dog", "initialize""#), "{}", artifact.source);
    assert!(artifact.source.contains(r#"__Sir.ivarSet("@name", name)"#), "{}", artifact.source);
    assert!(artifact.source.contains(r#"__Sir.ivarGet("@name")"#), "{}", artifact.source);

    if let Some(stdout) = run_module(&module, "oop_p1") {
        assert_eq!(stdout, "Rex says woof");
    }
}

/// P2 — inheritance + `super` + ivar-from-parent:
///   class Animal; def initialize(legs); @legs = legs; end
///                def legs; @legs; end; end
///   class Cat < Animal
///     def initialize; super(4); @name = "Tom"; end
///     def describe; print(@name + " with " + super_legs); end   (via super)
///   end
/// Here `describe` reads `@name` (set in Cat#initialize) and calls
/// `legs` which reads `@legs` (set by Animal#initialize via `super(4)`).
/// Output: "Tom with 4".
#[test]
fn p2_inheritance_super_and_parent_ivar() {
    // Animal#initialize(legs): @legs = legs
    let animal_init = func(
        "Animal__initialize",
        vec![param("legs")],
        vec![ivar_set("@legs", param_ref("legs"))],
        Expr::NilLit { span: sp() },
    );
    // Animal#legs: return @legs (a value method — no print)
    let animal_legs = func("Animal__legs", vec![], vec![], ivar("@legs"));
    // Cat#initialize: super(4); @name = "Tom"
    //   super("initialize","Cat", 4) runs Animal#initialize with self bound.
    let super_init = bc("__super__", vec![str_("initialize"), str_("Cat"), int(4)]);
    let cat_init = func(
        "Cat__initialize",
        vec![],
        vec![
            Stmt::ExprStmt { expr: super_init, span: sp() },
            ivar_set("@name", str_("Tom")),
        ],
        Expr::NilLit { span: sp() },
    );
    // Cat#describe: print(@name + " with " + self.legs)
    //   self.legs dispatches to Animal#legs (inherited), reading @legs.
    let self_legs = bc("__method__", vec![bc("__self__", vec![]), str_("legs")]);
    let legs_str = bc("__method__", vec![self_legs, str_("toString")]);
    let describe_line = bc("+", vec![bc("+", vec![ivar("@name"), str_(" with ")]), legs_str]);
    let cat_describe = func(
        "Cat__describe",
        vec![],
        vec![print(describe_line)],
        Expr::NilLit { span: sp() },
    );

    // Register the ancestry edge (Cat < Animal) by declaring the class so
    // the emitter emits `registerAncestry`.  Method `def`s are hoisted, so
    // the ClassDef bodies are empty.
    let animal_class = Stmt::ClassDef {
        name: "Animal".into(),
        superclass: None,
        body: vec![],
        span: sp(),
    };
    let cat_class = Stmt::ClassDef {
        name: "Cat".into(),
        superclass: Some("Animal".into()),
        body: vec![],
        span: sp(),
    };

    let make = bc("__new__", vec![str_("Cat")]);
    let describe = bc("__method__", vec![make, str_("describe")]);
    let main = vec![
        animal_class,
        cat_class,
        def_method("Animal", "initialize", "Animal__initialize"),
        def_method("Animal", "legs", "Animal__legs"),
        def_method("Cat", "initialize", "Cat__initialize"),
        def_method("Cat", "describe", "Cat__describe"),
        Stmt::ExprStmt { expr: describe, span: sp() },
    ];
    let module = oop_module(
        vec![animal_init, animal_legs, cat_init, cat_describe],
        main,
    );

    let artifact = compile(&module).expect("compile");
    assert!(artifact.source.contains(r#"__Sir.callSuper("initialize", "Cat", 4)"#), "{}", artifact.source);
    assert!(artifact.source.contains(r#"__Sir.registerAncestry({ "Cat": "Animal" });"#), "{}", artifact.source);

    if let Some(stdout) = run_module(&module, "oop_p2") {
        assert_eq!(stdout, "Tom with 4");
    }
}

/// SECURITY — a class / method named `constructor` or `__proto__` must
/// NOT execute host code.  Dispatch is an explicit `Map` key lookup on
/// `(class, method)`; a class named `constructor` was never defined in the
/// table, so `callNew("constructor")` just allocates a bare instance with
/// NO host `constructor` invoked, and a method named `__proto__` on it is a
/// clean Map-miss → NoMethodError (node exits non-zero).  This mirrors the
/// RCE-gadget regression style of `runtime_rejects_constructor_gadget`.
#[test]
fn oop_constructor_and_proto_names_are_inert() {
    // Define a legit method under the ODDLY-NAMED class so we prove the
    // table works, but the gadget name itself is never registered.
    // main:
    //   obj = __new__("constructor")        // Map-miss on "constructor\x00initialize" → bare instance
    //   print(obj.__proto__)                // "__proto__" method miss → NoMethodError → non-zero exit
    let make = bc("__new__", vec![str_("constructor")]);
    let gadget_call = bc("__method__", vec![make, str_("__proto__"), str_("return 1")]);
    let main = vec![print(gadget_call)];
    let module = oop_module(vec![], main);

    // Shape check: the emitted dispatch keys on the dangerous *name strings*,
    // never a reflective member access.
    let artifact = compile(&module).expect("compile");
    assert!(artifact.source.contains(r#"__Sir.callNew("constructor")"#), "{}", artifact.source);
    assert!(
        artifact.source.contains(r#"__Sir.callMethod(__Sir.callNew("constructor"), "__proto__", "return 1")"#),
        "{}",
        artifact.source
    );

    if let Some(stderr) = run_module_expecting_failure(&module, "oop_gadget") {
        // The miss surfaces as our NoMethodError (a SirError), NOT any
        // executed host `constructor`/`Function` payload.
        assert!(
            stderr.contains("NoMethodError") || stderr.contains("undefined method"),
            "expected a clean method-miss error, got stderr:\n{stderr}"
        );
    }
}

/// SECURITY — a genuinely cyclic ancestry (A < B < A) must not loop
/// forever during method resolution.  We register a cycle and dispatch a
/// method that exists on neither class: the `seen`-guarded walk terminates
/// with a NoMethodError instead of hanging.
#[test]
fn oop_cyclic_ancestry_terminates() {
    // Register A<B and B<A via two ClassDefs → a cycle in `ancestry`.
    let a_class = Stmt::ClassDef {
        name: "A".into(),
        superclass: Some("B".into()),
        body: vec![],
        span: sp(),
    };
    let b_class = Stmt::ClassDef {
        name: "B".into(),
        superclass: Some("A".into()),
        body: vec![],
        span: sp(),
    };
    // obj = A.new; obj.missing  → walk A→B→A… guarded → NoMethodError.
    let make = bc("__new__", vec![str_("A")]);
    let call_missing = bc("__method__", vec![make, str_("missing")]);
    let main = vec![
        a_class,
        b_class,
        Stmt::ExprStmt { expr: call_missing, span: sp() },
    ];
    let module = oop_module(vec![], main);

    if let Some(stderr) = run_module_expecting_failure(&module, "oop_cycle") {
        assert!(
            stderr.contains("NoMethodError") || stderr.contains("undefined method"),
            "expected termination with a method-miss error, got stderr:\n{stderr}"
        );
    }
}

// ── MX4: mixin (module include / extend) execution-proof ───────────────
//
// A *module* registers its `def`s exactly like a class — via
// `__def_method__` keyed by the module NAME (an "owner" is now a class OR
// a module).  `include M` / `extend M` lower to the two mixin builtins
// `__include__("Owner","M")` / `__extend__("Owner","M")`, and the inlined
// OOP runtime's `resolveMethod` now follows Ruby's MRO (class → included
// modules most-recent-first → superclass → …).  These hand-built SIR
// modules mirror exactly what the (merged) Ruby MX1 frontend emits, then
// run the self-contained `.js` under Node and assert stdout.

/// `include M` inside owner `Owner` → `__include__("Owner", "M")`.
fn include_(owner: &str, module: &str) -> Stmt {
    Stmt::ExprStmt {
        expr: bc("__include__", vec![str_(owner), str_(module)]),
        span: sp(),
    }
}
/// `extend M` inside owner `Owner` → `__extend__("Owner", "M")`.
fn extend_(owner: &str, module: &str) -> Stmt {
    Stmt::ExprStmt {
        expr: bc("__extend__", vec![str_(owner), str_(module)]),
        span: sp(),
    }
}

/// Like `oop_module`, but also flags `Feature::Modules` (the feature the
/// frontend triggers for `module` / `include` / `extend`).
fn mixin_module(methods: Vec<Function>, main_stmts: Vec<Stmt>) -> Module {
    let mut functions = methods;
    functions.push(func("main", vec![], main_stmts, Expr::NilLit { span: sp() }));
    Module {
        name: "mixin".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Classes,
            Feature::Modules,
            Feature::InstanceVars,
            Feature::Closures,
            Feature::Strings,
            Feature::DynamicTyping,
            Feature::Constants,
            Feature::MutableBindings,
        ]),
        imports: vec![],
        exports: vec![],
        functions,
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    }
}

/// MX4 (a) — a module's instance method, `include`d into a class, is
/// found on an instance of that class:
///   module Greet; def hello; print("hi from module"); end; end
///   class Robot; include Greet; end
///   Robot.new.hello   →   "hi from module"
#[test]
fn mixin_included_module_method_is_callable() {
    let hello = func(
        "Greet__hello",
        vec![],
        vec![print(str_("hi from module"))],
        Expr::NilLit { span: sp() },
    );
    // Module method registers keyed by the MODULE name, exactly like a class.
    let make = bc("__new__", vec![str_("Robot")]);
    let call = bc("__method__", vec![make, str_("hello")]);
    let main = vec![
        def_method("Greet", "hello", "Greet__hello"),
        include_("Robot", "Greet"),
        Stmt::ExprStmt { expr: call, span: sp() },
    ];
    let module = mixin_module(vec![hello], main);

    // Shape: include lowers to the runtime registration.
    let artifact = compile(&module).expect("compile");
    assert!(
        artifact.source.contains(r#"__Sir.includeModule("Robot", "Greet")"#),
        "{}",
        artifact.source
    );

    if let Some(stdout) = run_module(&module, "mixin_include") {
        assert_eq!(stdout, "hi from module");
    }
}

/// MX4 (b) — a method the CLASS defines itself SHADOWS the module's
/// (class-first MRO):
///   module M; def who; print("module"); end; end
///   class C; include M; def who; print("class"); end; end
///   C.new.who   →   "class"
#[test]
fn mixin_class_method_shadows_module() {
    let mod_who = func("M__who", vec![], vec![print(str_("module"))], Expr::NilLit { span: sp() });
    let cls_who = func("C__who", vec![], vec![print(str_("class"))], Expr::NilLit { span: sp() });
    let make = bc("__new__", vec![str_("C")]);
    let call = bc("__method__", vec![make, str_("who")]);
    let main = vec![
        def_method("M", "who", "M__who"),
        // The class both includes M and defines its own `who`.
        include_("C", "M"),
        def_method("C", "who", "C__who"),
        Stmt::ExprStmt { expr: call, span: sp() },
    ];
    let module = mixin_module(vec![mod_who, cls_who], main);
    if let Some(stdout) = run_module(&module, "mixin_shadow") {
        assert_eq!(stdout, "class", "class-defined method must shadow the module's");
    }
}

/// MX4 (c) — a DIAMOND include resolves the shared module ONCE and finds
/// the method (the `seen`-guarded MRO walk de-dupes and terminates):
///   module Base; def tag; print("base"); end; end
///   module Left;  include Base; end
///   module Right; include Base; end
///   class C; include Left; include Right; end
///   C.new.tag   →   "base"     (Base reached via two paths, resolved once)
#[test]
fn mixin_diamond_include_resolves_once() {
    let tag = func("Base__tag", vec![], vec![print(str_("base"))], Expr::NilLit { span: sp() });
    let make = bc("__new__", vec![str_("C")]);
    let call = bc("__method__", vec![make, str_("tag")]);
    let main = vec![
        def_method("Base", "tag", "Base__tag"),
        // Two intermediate modules each include Base (the diamond's arms).
        include_("Left", "Base"),
        include_("Right", "Base"),
        // The class includes both arms.
        include_("C", "Left"),
        include_("C", "Right"),
        Stmt::ExprStmt { expr: call, span: sp() },
    ];
    let module = mixin_module(vec![tag], main);
    if let Some(stdout) = run_module(&module, "mixin_diamond") {
        assert_eq!(stdout, "base", "diamond include must resolve Base once and find `tag`");
    }
}

/// MX4 (d) — `extend M` makes M's instance methods CLASS methods of the
/// owner (callable as `Owner.method`):
///   module Counter; def describe; print("i am a class method"); end; end
///   class Widget; extend Counter; end
///   Widget.describe   →   "i am a class method"
#[test]
fn mixin_extend_makes_class_method() {
    let describe = func(
        "Counter__describe",
        vec![],
        vec![print(str_("i am a class method"))],
        Expr::NilLit { span: sp() },
    );
    // `Widget.describe` on a constant receiver → __class_method__("Widget","describe").
    let call = bc("__class_method__", vec![str_("Widget"), str_("describe")]);
    let main = vec![
        def_method("Counter", "describe", "Counter__describe"),
        extend_("Widget", "Counter"),
        Stmt::ExprStmt { expr: call, span: sp() },
    ];
    let module = mixin_module(vec![describe], main);

    let artifact = compile(&module).expect("compile");
    assert!(
        artifact.source.contains(r#"__Sir.extendModule("Widget", "Counter")"#),
        "{}",
        artifact.source
    );
    assert!(
        artifact.source.contains(r#"__Sir.callClassMethod("Widget", "describe")"#),
        "{}",
        artifact.source
    );

    if let Some(stdout) = run_module(&module, "mixin_extend") {
        assert_eq!(stdout, "i am a class method");
    }
}

/// MX4 (e, security) — a SELF-including module must not loop forever: the
/// shared `seen` set terminates the MRO walk.  `module M; include M; end`
/// then a call to a MISSING method walks M → M (guarded) → NoMethodError,
/// so Node exits non-zero rather than hanging.
#[test]
fn mixin_self_including_module_terminates() {
    let make = bc("__new__", vec![str_("C")]);
    let call = bc("__method__", vec![make, str_("missing")]);
    let main = vec![
        // M includes itself; C includes M — the walk must not loop.
        include_("M", "M"),
        include_("C", "M"),
        Stmt::ExprStmt { expr: call, span: sp() },
    ];
    let module = mixin_module(vec![], main);
    if let Some(stderr) = run_module_expecting_failure(&module, "mixin_cycle") {
        assert!(
            stderr.contains("NoMethodError") || stderr.contains("undefined method"),
            "self-including module must terminate with a method-miss, got stderr:\n{stderr}"
        );
    }
}

// ── puts builtin (Ruby semantics) ──────────────────────────────────────

/// Compile a hand-built module, run it under `node`, and return the **raw**
/// stdout (newlines preserved).  Unlike `run_module`, this does NOT trim
/// trailing newlines — `puts` semantics hinge on the exact byte stream (a
/// trailing blank line is meaningful).  Returns `None` when Node is absent.
fn run_module_raw(module: &Module, tag: &str) -> Option<String> {
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
    // Normalise CRLF → LF so the assertion tests the semantics (one line per
    // unit), not the platform newline convention.
    Some(String::from_utf8(output.stdout).expect("utf8 stdout").replace("\r\n", "\n"))
}

#[test]
fn puts_matches_ruby_output() {
    // Ruby: `puts "hello"; puts; puts [1, 2, 3]`
    //   → "hello\n"   (string + newline)
    //   → "\n"        (no-arg puts → one blank line)
    //   → "1\n2\n3\n" (each array element on its own line)
    let puts_hello = Stmt::ExprStmt {
        expr: bc("puts", vec![str_("hello")]),
        span: sp(),
    };
    let puts_bare = Stmt::ExprStmt { expr: bc("puts", vec![]), span: sp() };
    let puts_arr = Stmt::ExprStmt {
        expr: bc(
            "puts",
            vec![Expr::SeqLit { items: vec![int(1), int(2), int(3)], span: sp() }],
        ),
        span: sp(),
    };
    let module = module_with_main(
        vec![puts_hello, puts_bare, puts_arr],
        Expr::NilLit { span: sp() },
        &[Feature::Sequences, Feature::Strings],
    );
    if let Some(stdout) = run_module_raw(&module, "puts") {
        assert_eq!(
            stdout, "hello\n\n1\n2\n3\n",
            "unexpected puts output (escaped): {stdout:?}"
        );
    }
}

/// Regression (security, CWE-674): `puts` on a self-referential array must
/// TERMINATE — printing a `[...]` cycle placeholder like Ruby — rather than
/// recursing until Node throws `RangeError: Maximum call stack size exceeded`.
///
/// Ruby:  `a = [nil]; a[0] = a; puts a`  → `[...]\n`.
#[test]
fn puts_cyclic_array_terminates() {
    let stmts = vec![
        // a = [nil]
        let_("a", Expr::SeqLit { items: vec![Expr::NilLit { span: sp() }], span: sp() }),
        // a[0] = a
        Stmt::SeqSet { seq: local("a"), index: int(0), value: local("a"), span: sp() },
        // puts a
        Stmt::ExprStmt { expr: bc("puts", vec![local("a")]), span: sp() },
    ];
    let module = module_with_main(
        stmts,
        Expr::NilLit { span: sp() },
        &[Feature::Sequences, Feature::MutableBindings],
    );
    // `run_module_raw` asserts a clean (zero) exit; a stack overflow would
    // exit non-zero and fail the test — so reaching the assert proves
    // termination.  The output matches Ruby's `[...]` cycle rendering.
    if let Some(stdout) = run_module_raw(&module, "puts_cyclic") {
        assert_eq!(
            stdout, "[...]\n",
            "unexpected cyclic puts output (escaped): {stdout:?}"
        );
    }
}

/// Regression (security, CWE-674): the polymorphic `*`-join arm renders each
/// element with `format`, which — like `puts` — must be cycle-guarded so a
/// self-referential array TERMINATES (`[...]` placeholder) instead of blowing
/// the Node stack.  `puts (a * ", ")` on `a = [a]` must print `[...]\n`.
#[test]
fn poly_array_join_cyclic_terminates() {
    let stmts = vec![
        // a = [nil]
        let_("a", Expr::SeqLit { items: vec![Expr::NilLit { span: sp() }], span: sp() }),
        // a[0] = a  (self-reference)
        Stmt::SeqSet { seq: local("a"), index: int(0), value: local("a"), span: sp() },
        // puts (a * ", ")  → join renders the single element, which is `a`
        // itself → the cycle guard emits `[...]`.
        Stmt::ExprStmt {
            expr: bc("puts", vec![bc("*", vec![local("a"), str_(", ")])]),
            span: sp(),
        },
    ];
    let module = module_with_main(
        stmts,
        Expr::NilLit { span: sp() },
        &[Feature::Sequences, Feature::MutableBindings, Feature::Strings],
    );
    // Reaching the assert (clean exit) proves termination; an unguarded
    // recursion would overflow the Node stack and exit non-zero, failing the
    // test.  The joined element is `a` itself (`[a]`), so it renders one array
    // level then the `[...]` cycle placeholder → `[[...]]`; the load-bearing
    // property is that the `[...]` guard fired and the program TERMINATED.
    if let Some(stdout) = run_module_raw(&module, "poly_join_cyclic") {
        assert!(
            stdout.contains("[...]") && stdout.ends_with('\n'),
            "cyclic join must terminate with a `[...]` placeholder; got (escaped): {stdout:?}"
        );
    }
}

// ── PO3: polymorphic `+` / `*` (Ruby operator overloading) ─────────────
//
// Ruby overloads `+`/`*` by receiver type; all lower to the same SIR
// `+`/`*` builtins, so the JS backend dispatches at runtime on the first
// operand's type (`__Sir.plus` / `__Sir.times`).  These execution-proofs
// build hand-crafted SIR and assert the exact stdout under Node, covering
// every arm from the spec table plus the numeric regressions.

/// A sequence literal helper.
fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: sp() }
}

/// `"a" + "b"` → "ab" (String concat).
#[test]
fn poly_string_plus_concatenates() {
    let module = module_with_main(
        vec![print(bc("+", vec![str_("a"), str_("b")]))],
        Expr::NilLit { span: sp() },
        &[Feature::Strings],
    );
    if let Some(stdout) = run_module(&module, "poly_str_plus") {
        assert_eq!(stdout, "ab");
    }
}

/// `"ab" * 3` → "ababab" (String repeat).
#[test]
fn poly_string_times_repeats() {
    let module = module_with_main(
        vec![print(bc("*", vec![str_("ab"), int(3)]))],
        Expr::NilLit { span: sp() },
        &[Feature::Strings],
    );
    if let Some(stdout) = run_module(&module, "poly_str_times") {
        assert_eq!(stdout, "ababab");
    }
}

/// `[1] + [2]` → `[1, 2]` (Array concat, NEW array — no `[]+[]` string
/// coercion).  Printed via `format`, whose array display is `[1, 2]`.
#[test]
fn poly_array_plus_concatenates() {
    let module = module_with_main(
        vec![print(bc("+", vec![seq(vec![int(1)]), seq(vec![int(2)])]))],
        Expr::NilLit { span: sp() },
        &[Feature::Sequences],
    );
    if let Some(stdout) = run_module(&module, "poly_arr_plus") {
        // The JS backend's `format` renders an array as `[1, 2]` — the
        // native `[1] + [2]` would WRONGLY print the string `1,2`.
        assert_eq!(stdout, "[1, 2]");
    }
}

/// `[0] * 3` → `[0, 0, 0]` (Array repeat, NEW array).
#[test]
fn poly_array_times_int_repeats() {
    let module = module_with_main(
        vec![print(bc("*", vec![seq(vec![int(0)]), int(3)]))],
        Expr::NilLit { span: sp() },
        &[Feature::Sequences],
    );
    if let Some(stdout) = run_module(&module, "poly_arr_times_int") {
        assert_eq!(stdout, "[0, 0, 0]");
    }
}

/// `[1, 2] * ", "` → "1, 2" (Array join with a String separator, using
/// the SAME `format` display helper `puts` uses on each element).
#[test]
fn poly_array_times_string_joins() {
    let module = module_with_main(
        vec![print(bc("*", vec![seq(vec![int(1), int(2)]), str_(", ")]))],
        Expr::NilLit { span: sp() },
        &[Feature::Sequences, Feature::Strings],
    );
    if let Some(stdout) = run_module(&module, "poly_arr_join") {
        assert_eq!(stdout, "1, 2");
    }
}

/// Regression — numeric `+`/`*` are UNCHANGED: `1 + 2` → 3, `2 * 3` → 6.
#[test]
fn poly_numeric_plus_times_unchanged() {
    let module = module_with_main(
        vec![
            print(bc("+", vec![int(1), int(2)])),
            print(bc("*", vec![int(2), int(3)])),
        ],
        Expr::NilLit { span: sp() },
        &[],
    );
    if let Some(stdout) = run_module(&module, "poly_numeric") {
        assert_eq!(stdout, "3\n6");
    }
}

/// SECURITY (CWE-1284/400) — an oversized repeat count must raise a
/// Ruby-shaped `ArgumentError: argument too big` rather than OOMing or
/// throwing a raw `RangeError`.  `"ab" * 2^53` overflows the safe-integer
/// product guard; node exits non-zero with our ArgumentError.
#[test]
fn poly_string_repeat_overflow_is_rejected() {
    // 2^53 = 9007199254740992 > MAX_SAFE_INTEGER / 2, so `2 * count`
    // exceeds the cap and the guard fires before any allocation.
    let huge = int(9_007_199_254_740_992);
    let module = module_with_main(
        vec![print(bc("*", vec![str_("ab"), huge]))],
        Expr::NilLit { span: sp() },
        &[Feature::Strings],
    );
    if let Some(stderr) = run_module_expecting_failure(&module, "poly_overflow") {
        assert!(
            stderr.contains("argument too big"),
            "expected the ArgumentError overflow guard, got stderr:\n{stderr}"
        );
    }
}

// ── T3: typed runtime errors (ZeroDivision/Index/Key/NoMethod) ─────────
//
// The sir-typed-runtime-errors cascade: a faulting emitted runtime op must
// raise the CORRECT typed `SirError` (matching Ruby) so a translated
// `begin; …; rescue ZeroDivisionError => e; …; end` catches it.  These
// execution-proofs build the `begin/rescue` (a `Stmt::TryCatch`) around
// each faulting op and assert — under Node — that the specific typed clause
// fires (printing the class name via `e.class` … but the frontend has no
// `.class` yet, so instead we print a fixed marker from the matching
// clause).  The nil-returning index ops (`arr[oob]`/`h[miss]`) prove the
// non-over-raise: they print `nil`, NOT an error.

/// Wrap `body` in `begin; <body>; rescue <class> => e; puts <marker>; end`
/// and assert the marker prints (i.e. the typed clause caught the fault).
fn assert_typed_rescue_catches(
    body: Vec<Stmt>,
    class: &str,
    marker: &str,
    features: &[Feature],
    tag: &str,
) {
    let mut feats = vec![Feature::Exceptions, Feature::Constants, Feature::Strings];
    feats.extend_from_slice(features);
    let try_catch = Stmt::TryCatch {
        body,
        rescues: vec![RescueClause {
            exception_types: vec![class.into()],
            binding: Some("e".into()),
            body: vec![print(str_(marker))],
            span: sp(),
        }],
        ensure_body: None,
        span: sp(),
    };
    let module = module_with_main(vec![try_catch], Expr::NilLit { span: sp() }, &feats);
    if let Some(stdout) = run_module(&module, tag) {
        assert_eq!(stdout, marker, "expected the `{class}` clause to catch");
    }
}

/// `begin; 1 / 0; rescue ZeroDivisionError => e; puts "zde"; end` → "zde".
/// Native JS `1 / 0 === Infinity`; the runtime `divide` helper adds the
/// zero-divisor check and raises the typed `ZeroDivisionError`.
#[test]
fn t3_int_div_by_zero_raises_zero_division_error() {
    assert_typed_rescue_catches(
        vec![Stmt::ExprStmt { expr: bc("/", vec![int(1), int(0)]), span: sp() }],
        "ZeroDivisionError",
        "zde",
        &[],
        "t3_int_zde",
    );
}

/// `1.0 / 0` also raises `ZeroDivisionError` in Ruby (float receiver, integer
/// zero divisor) — the helper's `b === 0` test covers the float case too.
#[test]
fn t3_float_div_by_zero_raises_zero_division_error() {
    assert_typed_rescue_catches(
        vec![Stmt::ExprStmt { expr: bc("/", vec![float(1.0), int(0)]), span: sp() }],
        "ZeroDivisionError",
        "zde-f",
        &[Feature::Floats],
        "t3_float_zde",
    );
}

/// A `ZeroDivisionError` is also caught by `rescue StandardError` (it chains
/// up the built-in ancestry) — proving the typed error is a real SirError in
/// the hierarchy, not an over-broad host fault.
#[test]
fn t3_zero_division_caught_by_standard_error() {
    assert_typed_rescue_catches(
        vec![Stmt::ExprStmt { expr: bc("/", vec![int(1), int(0)]), span: sp() }],
        "StandardError",
        "zde-std",
        &[],
        "t3_zde_std",
    );
}

/// `arr.fetch(100)` out of bounds raises `IndexError`.
#[test]
fn t3_array_fetch_oob_raises_index_error() {
    let arr = seq(vec![int(10), int(20), int(30)]);
    assert_typed_rescue_catches(
        vec![Stmt::ExprStmt {
            expr: bc("__method__", vec![arr, str_("fetch"), int(100)]),
            span: sp(),
        }],
        "IndexError",
        "idx",
        &[Feature::Sequences, Feature::DynamicTyping],
        "t3_arr_fetch_oob",
    );
}

/// SECURITY (CWE-470): `arr.fetch("constructor")` — a non-integer,
/// source-controlled index — must raise `TypeError` (Ruby: "no implicit
/// conversion of String into Integer") rather than falling through the
/// `NaN`-poisoned bounds checks to `recv["constructor"]`, which would leak
/// the `Array` constructor / prototype gadgets and bypass the method
/// allowlist.  A translated `rescue TypeError` catches it.
#[test]
fn t3_array_fetch_non_integer_index_raises_type_error_not_gadget() {
    let arr = seq(vec![int(1), int(2)]);
    assert_typed_rescue_catches(
        vec![Stmt::ExprStmt {
            expr: bc("__method__", vec![arr, str_("fetch"), str_("constructor")]),
            span: sp(),
        }],
        "TypeError",
        "Integer",
        &[Feature::Sequences, Feature::Strings, Feature::DynamicTyping],
        "t3_arr_fetch_gadget",
    );
}

/// `hash.fetch(missing)` with no default raises `KeyError`.
#[test]
fn t3_hash_fetch_missing_raises_key_error() {
    let map = Expr::MapLit {
        entries: vec![MapEntry { key: str_("a"), value: int(1) }],
        span: sp(),
    };
    assert_typed_rescue_catches(
        vec![Stmt::ExprStmt {
            expr: bc("__method__", vec![map, str_("fetch"), str_("missing")]),
            span: sp(),
        }],
        "KeyError",
        "key",
        &[Feature::Maps, Feature::DynamicTyping],
        "t3_hash_fetch_miss",
    );
}

/// An unknown method (`arr.frobnicate`) raises `NoMethodError`, NOT a
/// JS-native TypeError — so a translated `rescue NoMethodError` catches it.
#[test]
fn t3_unknown_method_raises_no_method_error() {
    let arr = seq(vec![int(1)]);
    assert_typed_rescue_catches(
        vec![Stmt::ExprStmt {
            expr: bc("__method__", vec![arr, str_("frobnicate")]),
            span: sp(),
        }],
        "NoMethodError",
        "nme",
        &[Feature::Sequences, Feature::DynamicTyping],
        "t3_unknown_method",
    );
}

/// NON-over-raise: `arr[oob]` and `h[miss]` still return **nil** (Ruby does
/// NOT raise for `[]`).  A begin/rescue around them must NOT fire; the
/// program prints `nil` twice.
#[test]
fn t3_index_ops_return_nil_no_over_raise() {
    // begin; puts(arr[100]); puts(h["missing"]); rescue => e; puts "SHOULD-NOT"; end
    let arr = seq(vec![int(10), int(20)]);
    let map = Expr::MapLit {
        entries: vec![MapEntry { key: str_("a"), value: int(1) }],
        span: sp(),
    };
    let try_catch = Stmt::TryCatch {
        body: vec![
            print(Expr::SeqIndex { seq: Box::new(arr), index: Box::new(int(100)), span: sp() }),
            print(Expr::MapGet { map: Box::new(map), key: Box::new(str_("missing")), span: sp() }),
        ],
        rescues: vec![RescueClause {
            exception_types: vec![], // bare rescue — would catch ANY raise
            binding: None,
            body: vec![print(str_("SHOULD-NOT-RESCUE"))],
            span: sp(),
        }],
        ensure_body: None,
        span: sp(),
    };
    let module = module_with_main(
        vec![try_catch],
        Expr::NilLit { span: sp() },
        &[
            Feature::Exceptions,
            Feature::Constants,
            Feature::Strings,
            Feature::Sequences,
            Feature::Maps,
        ],
    );
    if let Some(stdout) = run_module(&module, "t3_index_nil") {
        // Two nils printed; the (bare) rescue never fired.
        assert_eq!(stdout, "nil\nnil", "index ops must return nil, not raise");
    }
}

/// `arr.fetch(oob, default)` with a supplied default returns the default
/// instead of raising — matching Ruby's `fetch(i, d)`.
#[test]
fn t3_array_fetch_with_default_returns_default() {
    // print(arr.fetch(100, 42)) → 42 (no raise).
    let arr = seq(vec![int(10)]);
    let module = module_with_main(
        vec![print(bc("__method__", vec![arr, str_("fetch"), int(100), int(42)]))],
        Expr::NilLit { span: sp() },
        &[Feature::Sequences, Feature::DynamicTyping, Feature::Strings],
    );
    if let Some(stdout) = run_module(&module, "t3_fetch_default") {
        assert_eq!(stdout, "42", "fetch with a default must return it, not raise");
    }
}

// ── M6: universal Object metaprogramming surface (run under `node`) ────
//
// `send`/`__send__`/`public_send`, `tap`, `then`/`yield_self`, `respond_to?`,
// and boolean `&`/`|`/`^` are mixed into EVERY receiver (Ruby Kernel/Object).
// These prove the ported JS behaviour matches the Python/TS references, and —
// load-bearing — that `send`'s dynamic name routes through the SAME dispatch
// gate (an unknown name raises NoMethodError; a gadget name never runs host
// code), never `recv[name]`/reflection.

/// A one-param top-level function `name(v) { <stmts>; return <value> }`, the
/// shape a `tap`/`then` block is hoisted into.
fn block_fn(name: &str, stmts: Vec<Stmt>, value: Expr) -> Function {
    func(name, vec![param("v")], stmts, value)
}

/// `send(:meth, args…)` on an INSTANCE routes to the user method table.
///   class Greeter; def greet(who); print("hi " + who); end; end
///   Greeter.new.send(:greet, "sam")   →   "hi sam"
#[test]
fn m6_send_routes_to_instance_method() {
    // def greet(who); print("hi " + who); end
    let greet = func(
        "Greeter__greet",
        vec![param("who")],
        vec![print(bc("+", vec![str_("hi "), param_ref("who")]))],
        Expr::NilLit { span: sp() },
    );
    let make = bc("__new__", vec![str_("Greeter")]);
    // obj.send(:greet, "sam")  →  __method__(obj, "send", :greet, "sam")
    let sent = bc(
        "__method__",
        vec![make, str_("send"), Expr::SymLit { name: "greet".into(), span: sp() }, str_("sam")],
    );
    let main = vec![
        def_method("Greeter", "greet", "Greeter__greet"),
        Stmt::ExprStmt { expr: sent, span: sp() },
    ];
    let module = oop_module(vec![greet], main);
    if let Some(stdout) = run_module(&module, "m6_send_instance") {
        assert_eq!(stdout, "hi sam");
    }
}

/// `send` with a STRING name and a primitive receiver routes through the
/// native-method allowlist: `"hello".send("upcase")` → "HELLO".
#[test]
fn m6_send_string_name_on_primitive() {
    let sent = bc("__method__", vec![str_("hello"), str_("send"), str_("upcase")]);
    let module = module_with_main(
        vec![print(sent)],
        Expr::NilLit { span: sp() },
        &[Feature::Strings, Feature::DynamicTyping],
    );
    if let Some(stdout) = run_module(&module, "m6_send_upcase") {
        assert_eq!(stdout, "HELLO");
    }
}

/// SECURITY: `send` of an UNKNOWN / gadget name raises NoMethodError — the
/// dynamic name is gated by the SAME allowlist a direct call uses, so node
/// exits non-zero and the `"return 1"` payload is never synthesised/run.
#[test]
fn m6_send_unknown_name_raises_no_method_error() {
    // "x".send("constructor", "return 1")  → NoMethodError (gadget blocked).
    let gadget = bc(
        "__method__",
        vec![str_("x"), str_("send"), str_("constructor"), str_("return 1")],
    );
    let module = module_with_main(
        vec![print(gadget)],
        Expr::NilLit { span: sp() },
        &[Feature::Strings, Feature::DynamicTyping],
    );
    if let Some(stderr) = run_module_expecting_failure(&module, "m6_send_gadget") {
        assert!(
            stderr.contains("NoMethodError") || stderr.contains("undefined method"),
            "send of a gadget name must raise NoMethodError, got stderr:\n{stderr}"
        );
    }
}

/// `tap` yields the receiver to the block (side effect) and returns the
/// RECEIVER.  `print( 7.tap { |v| print("tap") } )` prints `tap` then `7`.
#[test]
fn m6_tap_runs_block_and_returns_receiver() {
    // block: def _tap(v); print("tap"); end
    let blk = block_fn("m6_tap_blk", vec![print(str_("tap"))], Expr::NilLit { span: sp() });
    let tap_call = bc(
        "__method__",
        vec![int(7), str_("tap"), method_closure("m6_tap_blk")],
    );
    let module = oop_module(vec![blk], vec![print(tap_call)]);
    if let Some(stdout) = run_module(&module, "m6_tap") {
        // Block ran first (prints "tap"), then tap's result (the receiver 7).
        assert_eq!(stdout, "tap\n7");
    }
}

/// `then`/`yield_self` yields the receiver and returns the BLOCK'S RESULT.
///   print( 7.then { |v| v + 1 } )   →   8
#[test]
fn m6_then_returns_block_result() {
    // block: def _then(v); v + 1; end
    let blk = block_fn("m6_then_blk", vec![], bc("+", vec![param_ref("v"), int(1)]));
    let then_call = bc(
        "__method__",
        vec![int(7), str_("then"), method_closure("m6_then_blk")],
    );
    let module = oop_module(vec![blk], vec![print(then_call)]);
    if let Some(stdout) = run_module(&module, "m6_then") {
        assert_eq!(stdout, "8");
    }
}

/// `yield_self` is `then`'s alias — same "return the block result" rule.
#[test]
fn m6_yield_self_returns_block_result() {
    let blk = block_fn("m6_ys_blk", vec![], bc("+", vec![param_ref("v"), int(10)]));
    let call = bc(
        "__method__",
        vec![int(5), str_("yield_self"), method_closure("m6_ys_blk")],
    );
    let module = oop_module(vec![blk], vec![print(call)]);
    if let Some(stdout) = run_module(&module, "m6_yield_self") {
        assert_eq!(stdout, "15");
    }
}

/// `respond_to?` is honest: true for a name dispatch resolves, false
/// otherwise — checked against the SAME allowlist/method table dispatch uses.
#[test]
fn m6_respond_to_reports_dispatchable_names() {
    // print("x".respond_to?(:upcase))  → true   (allowlisted native)
    // print("x".respond_to?(:nope))    → false  (not resolvable)
    // print("x".respond_to?(:tap))     → true   (universal M6)
    let r_upcase = bc(
        "__method__",
        vec![str_("x"), str_("respond_to?"), Expr::SymLit { name: "upcase".into(), span: sp() }],
    );
    let r_nope = bc(
        "__method__",
        vec![str_("x"), str_("respond_to?"), Expr::SymLit { name: "nope".into(), span: sp() }],
    );
    let r_tap = bc(
        "__method__",
        vec![str_("x"), str_("respond_to?"), Expr::SymLit { name: "tap".into(), span: sp() }],
    );
    let module = module_with_main(
        vec![print(r_upcase), print(r_nope), print(r_tap)],
        Expr::NilLit { span: sp() },
        &[Feature::Strings, Feature::DynamicTyping, Feature::Constants, Feature::Symbols],
    );
    if let Some(stdout) = run_module(&module, "m6_respond_to") {
        // The runtime renders booleans as Lisp `#t`/`#f` via `format`.
        assert_eq!(stdout, "#t\n#f\n#t");
    }
}

/// `respond_to?` on an INSTANCE consults the user method table honestly.
#[test]
fn m6_respond_to_on_instance() {
    let bark = func("Pup__bark", vec![], vec![], str_("woof"));
    let make = bc("__new__", vec![str_("Pup")]);
    // print(obj.respond_to?(:bark))  → true ; print(obj.respond_to?(:meow)) → false
    let r_bark = bc(
        "__method__",
        vec![make.clone(), str_("respond_to?"), Expr::SymLit { name: "bark".into(), span: sp() }],
    );
    let r_meow = bc(
        "__method__",
        vec![make, str_("respond_to?"), Expr::SymLit { name: "meow".into(), span: sp() }],
    );
    let main = vec![
        def_method("Pup", "bark", "Pup__bark"),
        print(r_bark),
        print(r_meow),
    ];
    let module = oop_module(vec![bark], main);
    if let Some(stdout) = run_module(&module, "m6_respond_to_instance") {
        assert_eq!(stdout, "#t\n#f");
    }
}

/// Boolean `&`/`|`/`^` on a `true`/`false` receiver — Ruby's eager logical
/// operators (non-short-circuiting), coercing the operand by SIR truthiness.
#[test]
fn m6_boolean_operators() {
    // true & false → #f ; true | false → #t ; true ^ true → #f ; false | 0 → #t
    let and = bc("__method__", vec![Expr::BoolLit { value: true, span: sp() }, str_("&"), Expr::BoolLit { value: false, span: sp() }]);
    let or = bc("__method__", vec![Expr::BoolLit { value: true, span: sp() }, str_("|"), Expr::BoolLit { value: false, span: sp() }]);
    let xor = bc("__method__", vec![Expr::BoolLit { value: true, span: sp() }, str_("^"), Expr::BoolLit { value: true, span: sp() }]);
    // 0 is TRUTHY in SIR/Ruby, so `false | 0` is true.
    let or_zero = bc("__method__", vec![Expr::BoolLit { value: false, span: sp() }, str_("|"), int(0)]);
    let module = module_with_main(
        vec![print(and), print(or), print(xor), print(or_zero)],
        Expr::NilLit { span: sp() },
        &[Feature::Strings, Feature::DynamicTyping],
    );
    if let Some(stdout) = run_module(&module, "m6_bool_ops") {
        assert_eq!(stdout, "#f\n#t\n#f\n#t");
    }
}

// ── Ruby Numeric method catalog (hand-implemented, explicit dispatch) ──
//
// Exercises the `numericMethod` catalog end-to-end under Node: the emitted
// JS must route `(-5).abs`, `12.gcd(18)`, `123.digits`, block-taking
// `1.upto(3)`, … through the explicit numeric switch (never `recv[name]`)
// and produce Ruby-faithful values.

fn method(recv: Expr, name: &str, args: Vec<Expr>) -> Expr {
    let mut a = vec![recv, str_(name)];
    a.extend(args);
    bc("__method__", a)
}

#[test]
fn numeric_catalog_nonblock_methods() {
    let stmts = vec![
        print(method(int(-5), "abs", vec![])),        // 5
        print(method(int(12), "gcd", vec![int(18)])), // 6
        print(method(int(2), "pow", vec![int(8)])),   // 256
        print(method(int(123), "digits", vec![])),    // [3, 2, 1]
        print(method(float(3.2), "ceil", vec![])),    // 4
        print(method(float(2.5), "round", vec![])),   // 3
        print(method(int(5), "succ", vec![])),        // 6
        print(method(int(5), "pred", vec![])),        // 4
    ];
    let module =
        module_with_main(stmts, Expr::NilLit { span: sp() }, &[Feature::Floats, Feature::Strings]);
    if let Some(stdout) = run_module(&module, "numcatalog") {
        assert_eq!(stdout, "5\n6\n256\n[3, 2, 1]\n4\n3\n6\n4");
    }
}

#[test]
fn numeric_upto_runs_block() {
    // def blk(i); print(i); end ; 1.upto(3) { |i| print i }  → 1,2,3
    let blk = func("blk", vec![param("i")], vec![print(param_ref("i"))], Expr::NilLit { span: sp() });
    let call = method(int(1), "upto", vec![int(3), method_closure("blk")]);
    let module = oop_module(vec![blk], vec![Stmt::ExprStmt { expr: call, span: sp() }]);
    if let Some(stdout) = run_module(&module, "numupto") {
        assert_eq!(stdout, "1\n2\n3");
    }
}

// ── Ruby String method catalog (hand-implemented, explicit dispatch) ──
//
// Exercises the `stringMethod` catalog end-to-end under Node: `capitalize`,
// `reverse`, literal `sub`/`gsub`, `to_i`, `chomp`, rune `index`, and `chars`
// must route through the explicit string switch (never `recv[name]`) and
// produce Ruby-faithful values, while the already-aliased natives (`upcase`…)
// are untouched.
#[test]
fn string_catalog_methods() {
    let stmts = vec![
        print(method(str_("hELLO"), "capitalize", vec![])),                 // Hello
        print(method(str_("abc"), "reverse", vec![])),                      // cba
        print(method(str_("hello"), "sub", vec![str_("l"), str_("L")])),    // heLlo
        print(method(str_("hello"), "gsub", vec![str_("l"), str_("L")])),   // heLLo
        print(method(str_("42abc"), "to_i", vec![])),                       // 42
        print(method(str_("hi\n"), "chomp", vec![])),                       // hi
        print(method(str_("hello world"), "index", vec![str_("world")])),   // 6
        print(method(str_("abc"), "chars", vec![])),                        // [a, b, c]
    ];
    let module = module_with_main(stmts, Expr::NilLit { span: sp() }, &[Feature::Strings]);
    if let Some(stdout) = run_module(&module, "strcatalog") {
        assert_eq!(stdout, "Hello\ncba\nheLlo\nheLLo\n42\nhi\n6\n[a, b, c]");
    }
}

// v0.19.0 justify / swapcase String methods.  `ljust`/`rjust`/`center` pad in
// RUNES with a cyclic pad (center's odd extra pad on the RIGHT); a `width` no
// larger than the string is a no-op; `swapcase` flips ASCII case.  All route
// through the explicit `stringMethod` switch (never `recv[name]`).
#[test]
fn string_justify_swapcase_methods() {
    let stmts = vec![
        print(method(str_("hi"), "ljust", vec![int(5)])),                 // "hi   "
        print(method(str_("hi"), "ljust", vec![int(5), str_("*")])),      // hi***
        print(method(str_("hi"), "rjust", vec![int(5), str_("*")])),      // ***hi
        print(method(str_("hi"), "center", vec![int(6), str_("*")])),     // **hi**
        print(method(str_("hi"), "center", vec![int(5), str_("*")])),     // *hi**
        print(method(str_("abc"), "ljust", vec![int(1)])),                // abc (no-op)
        print(method(str_("abcdef"), "ljust", vec![int(10), str_("xy")])), // abcdefxyxy
        print(method(str_("Hello World"), "swapcase", vec![])),           // hELLO wORLD
    ];
    let module = module_with_main(stmts, Expr::NilLit { span: sp() }, &[Feature::Strings]);
    if let Some(stdout) = run_module(&module, "strjustify") {
        assert_eq!(
            stdout,
            "hi   \nhi***\n***hi\n**hi**\n*hi**\nabc\nabcdefxyxy\nhELLO wORLD"
        );
    }
}

// v0.20.0 slice-selection Array methods: `take`, `drop`, `values_at`.  All are
// index-clamped / bounds-guarded and route through the explicit `arrayMethod`
// switch (never `recv[name]`).
#[test]
fn array_take_drop_values_at_methods() {
    let stmts = vec![
        print(method(seq(vec![int(1), int(2), int(3), int(4), int(5)]), "take", vec![int(2)])), // [1, 2]
        print(method(seq(vec![int(1), int(2), int(3)]), "take", vec![int(9)])), // [1, 2, 3] (clamp)
        print(method(seq(vec![int(1), int(2), int(3), int(4), int(5)]), "drop", vec![int(2)])), // [3, 4, 5]
        print(method(seq(vec![int(1), int(2), int(3)]), "drop", vec![int(9)])), // [] (n >= len)
        print(method(
            seq(vec![int(10), int(20), int(30)]),
            "values_at",
            vec![int(0), int(2), int(-1)],
        )), // [10, 30, 30]
    ];
    let module =
        module_with_main(stmts, Expr::NilLit { span: sp() }, &[Feature::Sequences, Feature::Strings]);
    if let Some(stdout) = run_module(&module, "arrtakedrop") {
        assert_eq!(stdout, "[1, 2]\n[1, 2, 3]\n[3, 4, 5]\n[]\n[10, 30, 30]");
    }
}

// v0.21.0 non-block Array catch-up: `flatten`, `compact`, `rotate`, `zip` — the
// last of the reference (Go/Rust/Python/TS) non-block surface the JS backend was
// missing.  All route through the explicit `arrayMethod` switch (never `recv[name]`
// and never the depth-1 native `flat`).  Proven end-to-end under Node.
#[test]
fn array_flatten_compact_rotate_zip_methods() {
    let nil = || Expr::NilLit { span: sp() };
    let stmts = vec![
        // flatten fully flattens nested Arrays (not the depth-1 native `flat`)
        print(method(
            seq(vec![int(1), seq(vec![int(2), seq(vec![int(3)])])]),
            "flatten",
            vec![],
        )), // [1, 2, 3]
        // compact drops every nil
        print(method(
            seq(vec![int(1), nil(), int(2), nil()]),
            "compact",
            vec![],
        )), // [1, 2]
        // rotate: default 1, explicit 2, negative rotates right
        print(method(seq(vec![int(1), int(2), int(3), int(4)]), "rotate", vec![])), // [2, 3, 4, 1]
        print(method(seq(vec![int(1), int(2), int(3), int(4)]), "rotate", vec![int(2)])), // [3, 4, 1, 2]
        print(method(seq(vec![int(1), int(2), int(3), int(4)]), "rotate", vec![int(-1)])), // [4, 1, 2, 3]
        // zip pads a shorter operand with nil, keeps receiver length
        print(method(
            seq(vec![int(1), int(2), int(3)]),
            "zip",
            vec![seq(vec![int(4), int(5)])],
        )), // [[1, 4], [2, 5], [3, nil]]
    ];
    let module =
        module_with_main(stmts, Expr::NilLit { span: sp() }, &[Feature::Sequences, Feature::Strings]);
    if let Some(stdout) = run_module(&module, "arrflattenrotate") {
        assert_eq!(
            stdout,
            "[1, 2, 3]\n[1, 2]\n[2, 3, 4, 1]\n[3, 4, 1, 2]\n[4, 1, 2, 3]\n[[1, 4], [2, 5], [3, nil]]"
        );
    }
}

// v0.22.0 native-alias divergence fixes: `include?` and `index` now use Ruby
// VALUE equality (`valEq`) via the explicit `arrayMethod` switch, not native
// `Array#includes`/`indexOf` (which use identity and return `-1`).  So a nested
// Array matches structurally, and a missing element yields `nil` (not `-1`).
// `index` was previously ABSENT for arrays (NoMethodError).  Booleans render
// `#t`/`#f` here because the module's source language is non-Ruby ("handbuilt").
#[test]
fn array_include_and_index_value_equality() {
    let pair = |a, b| seq(vec![int(a), int(b)]);
    let stmts = vec![
        print(method(seq(vec![int(10), int(20), int(30)]), "include?", vec![int(20)])), // #t
        print(method(seq(vec![int(10), int(20), int(30)]), "include?", vec![int(99)])), // #f
        // structural: a nested Array matches by value, not identity
        print(method(seq(vec![pair(1, 2), pair(3, 4)]), "include?", vec![pair(1, 2)])), // #t
        print(method(seq(vec![int(10), int(20), int(30)]), "index", vec![int(20)])), // 1
        print(method(seq(vec![int(10), int(20), int(30)]), "index", vec![int(99)])), // nil
        print(method(seq(vec![pair(1, 2), pair(3, 4)]), "index", vec![pair(3, 4)])), // 1
    ];
    let module =
        module_with_main(stmts, Expr::NilLit { span: sp() }, &[Feature::Sequences, Feature::Strings]);
    if let Some(stdout) = run_module(&module, "arrincludeindex") {
        assert_eq!(stdout, "#t\n#f\n#t\n1\nnil\n1");
    }
}

// ── Ruby Hash method catalog (hand-implemented, explicit dispatch) ──
//
// Exercises the `hashMethod` catalog end-to-end under Node: `keys`/`values`/
// `to_a` must return real Arrays (not native Map iterators), `dig`/`merge`
// resolve faithfully, and `delete` mutates the receiver — all routed through
// the explicit Map switch (never `recv[name]`).
#[test]
fn hash_catalog_methods() {
    let mk = |pairs: Vec<(&str, Expr)>| Expr::MapLit {
        entries: pairs
            .into_iter()
            .map(|(k, v)| MapEntry { key: str_(k), value: v })
            .collect(),
        span: sp(),
    };
    let stmts = vec![
        let_("m", mk(vec![("a", int(1)), ("b", int(2))])),
        print(method(local("m"), "keys", vec![])),   // [a, b]
        print(method(local("m"), "values", vec![])), // [1, 2]
        print(method(local("m"), "size", vec![])),   // 2
        print(method(local("m"), "to_a", vec![])),   // [[a, 1], [b, 2]]
        print(method(local("m"), "dig", vec![str_("b")])), // 2
        print(method(
            method(local("m"), "merge", vec![mk(vec![("c", int(3))])]),
            "keys",
            vec![],
        )), // [a, b, c]
        print(method(local("m"), "delete", vec![str_("a")])), // 1 (mutates m)
        print(method(local("m"), "keys", vec![])),            // [b]
    ];
    let module =
        module_with_main(stmts, Expr::NilLit { span: sp() }, &[Feature::Maps, Feature::Strings]);
    if let Some(stdout) = run_module(&module, "hashcatalog") {
        assert_eq!(stdout, "[a, b]\n[1, 2]\n2\n[[a, 1], [b, 2]]\n2\n[a, b, c]\n1\n[b]");
    }
}

// ── source-language display convention: Ruby booleans (SIR spec) ──
//
// A Ruby-sourced module renders booleans as `true`/`false`; every other
// source language keeps the default Lisp `#t`/`#f`.  Proven end-to-end under
// Node for both conventions.
fn bool_display_module(source_language: &str) -> Module {
    let stmts = vec![
        print(Expr::BoolLit { value: true, span: sp() }),
        print(Expr::BoolLit { value: false, span: sp() }),
    ];
    Module {
        name: "dispbool".into(),
        manifest: FeatureManifest::from_features(&[]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: sp() }, span: sp() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: sp(),
        }],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language(source_language)
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    }
}

#[test]
fn ruby_source_prints_true_false() {
    if let Some(stdout) = run_module(&bool_display_module("ruby"), "dispruby") {
        assert_eq!(stdout, "true\nfalse", "Ruby source must render booleans as true/false");
    }
}

#[test]
fn twig_source_keeps_lisp_booleans() {
    if let Some(stdout) = run_module(&bool_display_module("twig"), "disptwig") {
        assert_eq!(stdout, "#t\n#f", "non-Ruby source keeps the default Lisp #t/#f");
    }
}

// ── Ruby Symbol method catalog (hand-implemented, explicit dispatch) ──
//
// Exercises the `symbolMethod` catalog end-to-end under Node.  Ruby's case
// methods return a new SYMBOL (`:foo.upcase == :FOO`), `to_s` a string,
// `inspect` the `:`-prefixed form — all via the explicit Sym switch (never
// `recv[name]`).
fn sym(name: &str) -> Expr {
    Expr::SymLit { name: name.into(), span: sp() }
}

#[test]
fn symbol_catalog_methods() {
    let stmts = vec![
        print(method(sym("hello"), "to_s", vec![])),       // hello
        print(method(sym("hello"), "upcase", vec![])),     // HELLO (a Symbol → bare name)
        print(method(sym("hello"), "length", vec![])),     // 5
        print(method(sym("hello"), "inspect", vec![])),    // :hello
        print(method(sym("FOO"), "downcase", vec![])),     // foo
        print(method(sym("hello"), "capitalize", vec![])), // Hello
    ];
    let module = module_with_main(
        stmts,
        Expr::NilLit { span: sp() },
        &[Feature::Symbols, Feature::Strings],
    );
    if let Some(stdout) = run_module(&module, "symcatalog") {
        assert_eq!(stdout, "hello\nHELLO\n5\n:hello\nfoo\nHello");
    }
}

// ── Ruby Array / Enumerable catalog (arrayMethod) ──────────────────────
//
// JS arrays previously had NO Ruby Array catalog — only native JS methods via
// the allowlist — so `select`/`reject`/`inject`/`any?` were unsupported and
// `sort` was lexicographic (wrong for numbers). This proves the new explicit
// `arrayMethod` catalog end-to-end under Node.
fn arr(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: sp() }
}

fn array_catalog_module() -> Module {
    // Block bodies referenced by the MakeClosures.
    let block_fns = vec![
        func("__ba_even", vec![param("x")], vec![], method(param_ref("x"), "even?", vec![])),
        func("__ba_id", vec![param("x")], vec![], param_ref("x")),
        func(
            "__ba_pair",
            vec![param("x")],
            vec![],
            arr(vec![param_ref("x"), param_ref("x")]),
        ),
        func(
            "__ba_lt3",
            vec![param("x")],
            vec![],
            bc("<", vec![param_ref("x"), int(3)]),
        ),
        func(
            "__ba_add",
            vec![param("a"), param("x")],
            vec![],
            bc("+", vec![param_ref("a"), param_ref("x")]),
        ),
    ];
    let a1234 = || arr(vec![int(1), int(2), int(3), int(4)]);
    let a312 = || arr(vec![int(3), int(1), int(2)]);
    let stmts = vec![
        print(method(a312(), "sort", vec![])),                                   // [1, 2, 3]
        print(method(a1234(), "select", vec![method_closure("__ba_even")])),     // [2, 4]
        print(method(a1234(), "reject", vec![method_closure("__ba_even")])),     // [1, 3]
        print(method(a1234(), "inject", vec![method_closure("__ba_add")])),      // 10
        print(method(a312(), "sort_by", vec![method_closure("__ba_id")])),       // [1, 2, 3]
        print(method(a1234(), "partition", vec![method_closure("__ba_even")])),  // [[2, 4], [1, 3]]
        print(method(
            arr(vec![int(1), int(2), int(3)]),
            "flat_map",
            vec![method_closure("__ba_pair")],
        )), // [1, 1, 2, 2, 3, 3]
        print(method(a1234(), "take_while", vec![method_closure("__ba_lt3")])),  // [1, 2]
        print(method(a1234(), "count", vec![method_closure("__ba_even")])),      // 2
        print(method(a312(), "min", vec![])),                                    // 1
        print(method(a312(), "max", vec![])),                                    // 3
        print(method(arr(vec![int(1), int(2), int(3)]), "sum", vec![])),         // 6
        print(method(arr(vec![int(1), int(1), int(2), int(3)]), "uniq", vec![])), // [1, 2, 3]
    ];
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts, value: Expr::NilLit { span: sp() }, span: sp() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: sp(),
    };
    let mut functions = vec![main];
    functions.extend(block_fns);
    Module {
        name: "arrcat".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Closures,
            Feature::Strings,
            Feature::Symbols,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions,
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("handbuilt")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: sp(),
    }
}

#[test]
fn array_catalog_methods() {
    if let Some(stdout) = run_module(&array_catalog_module(), "arrcat") {
        assert_eq!(
            stdout,
            "[1, 2, 3]\n[2, 4]\n[1, 3]\n10\n[1, 2, 3]\n[[2, 4], [1, 3]]\n\
             [1, 1, 2, 2, 3, 3]\n[1, 2]\n2\n1\n3\n6\n[1, 2, 3]"
        );
    }
}
