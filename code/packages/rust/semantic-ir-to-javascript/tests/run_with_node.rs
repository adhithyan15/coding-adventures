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
        assert!(
            stderr.contains("not an allowed collection method"),
            "expected the allowlist TypeError, got stderr:\n{stderr}"
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
