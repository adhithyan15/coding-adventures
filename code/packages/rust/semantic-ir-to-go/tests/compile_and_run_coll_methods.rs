//! End-to-end proof for C5 **collection-method dispatch** in the Go
//! backend — the Go analogue of the Python/TS `sir-runtime-oop` catalog.
//!
//! Unit tests (`src/emit.rs`) assert the emitted *shape* (`_sir_call_method`
//! calls, catalog helpers present in the runtime).  This test goes the whole
//! way: it hand-builds SIR modules that exercise the catalog — `.map`,
//! `.select`, `.length`, `.push`, `.reduce`, `.join`, `.upcase`, `.keys`,
//! `.even?`, and `&:sym` — emits Go, runs it under a real `go run`, and
//! diffs stdout against the values the Python/TS reference backends produce
//! for the SAME SIR module (`[1,2,3].map { |x| x*2 }` → `[2,4,6]`, etc.).
//!
//! It also proves an **unknown method** (`[1].bogus_xyz`) fails CLEANLY — a
//! non-zero `go run` exit carrying the controlled "undefined method"
//! message — rather than a silent nil or a garbage result.
//!
//! Gated on `go version`: a missing toolchain logs a skip rather than
//! reddening the build (mirrors `compile_and_run_seq_maps.rs`).  Numeric /
//! array results are asserted via `print` of the returned value (StrLit
//! interpolation is out of the Go backend's accepted set — see the crate
//! tests — so we print structured results, matching prior Go exec tests).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope,
    Span, Stmt,
};
use semantic_ir_to_go::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn var_p(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

fn builtin(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

/// `recv.meth(extra…)` → `BuiltinCall("__method__", [recv, "meth", …extra])`.
fn method(recv: Expr, name: &str, extra: Vec<Expr>) -> Expr {
    let mut args = vec![recv, slit(name)];
    args.extend(extra);
    builtin("__method__", args)
}

fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "print".into(),
            args: vec![expr],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// One-parameter lambda `fn_name(x) -> body`, registered as a top-level
/// function; the caller builds a `MakeClosure` referencing it.
fn lambda_fn(fn_name: &str, param: &str, body: Expr) -> Function {
    Function {
        name: fn_name.into(),
        params: vec![semantic_ir::Param {
            name: param.into(),
            kind: semantic_ir::ParamKind::Required,
            sir_type: None,
            default: None,
            span: s(),
        }],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: body, span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    }
}

/// Two-parameter lambda (for `reduce`).
fn lambda_fn2(fn_name: &str, p0: &str, p1: &str, body: Expr) -> Function {
    Function {
        name: fn_name.into(),
        params: vec![
            semantic_ir::Param {
                name: p0.into(),
                kind: semantic_ir::ParamKind::Required,
                sir_type: None,
                default: None,
                span: s(),
            },
            semantic_ir::Param {
                name: p1.into(),
                kind: semantic_ir::ParamKind::Required,
                sir_type: None,
                default: None,
                span: s(),
            },
        ],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: body, span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    }
}

fn closure(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: vec![], span: s() }
}

fn manifest() -> FeatureManifest {
    FeatureManifest::from_features(&[
        Feature::Closures,
        Feature::Sequences,
        Feature::Maps,
        Feature::Strings,
        Feature::Symbols,
        Feature::MutableBindings,
        Feature::DynamicTyping,
    ])
}

fn program(functions: Vec<Function>) -> Module {
    Module {
        name: "coll_methods_demo".into(),
        manifest: manifest(),
        imports: vec![],
        exports: vec![],
        functions,
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

/// The catalog demo `main`.  Each printed line's expected value is what the
/// Python/TS reference runtime yields for the identical operation.
fn catalog_module() -> Module {
    // Lambdas the block methods drive.
    let double = lambda_fn("__lam_double", "x", builtin("*", vec![var_p("x"), ilit(2)]));
    let is_even = lambda_fn("__lam_even", "x", method(var_p("x"), "even?", vec![]));
    let add = lambda_fn2("__lam_add", "a", "b", builtin("+", vec![var_p("a"), var_p("b")]));

    let stmts = vec![
        // [1,2,3].length → 3
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "length", vec![])),
        // [1,2,3].map { |x| x*2 }  → [2, 4, 6]
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "map", vec![closure("__lam_double")])),
        // [1,2,3,4].select { |x| x.even? } → [2, 4]
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            "select",
            vec![closure("__lam_even")],
        )),
        // [1,2,3,4].reduce(0) { |a,b| a+b } → 10
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            "reduce",
            vec![ilit(0), closure("__lam_add")],
        )),
        // ["a","b","c"].join → "abc"
        print_stmt(method(seq(vec![slit("a"), slit("b"), slit("c")]), "join", vec![])),
        // ["a","b"].join("-") → "a-b"
        print_stmt(method(seq(vec![slit("a"), slit("b")]), "join", vec![slit("-")])),
        // [3,1,2].sort → [1, 2, 3]
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "sort", vec![])),
        // [1,2,3].reverse → [3, 2, 1]
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "reverse", vec![])),
        // "hello".upcase → "HELLO"
        print_stmt(method(slit("hello"), "upcase", vec![])),
        // "HeLLo".downcase → "hello"
        print_stmt(method(slit("HeLLo"), "downcase", vec![])),
        // "  hi  ".strip → "hi"
        print_stmt(method(slit("  hi  "), "strip", vec![])),
        // "a,b,c".split(",") → [a, b, c]
        print_stmt(method(slit("a,b,c"), "split", vec![slit(",")])),
        // 7.even? → #f ; 8.even? → #t
        print_stmt(method(ilit(7), "even?", vec![])),
        print_stmt(method(ilit(8), "even?", vec![])),
        // (-5).abs → 5
        print_stmt(method(ilit(-5), "abs", vec![])),
        // {a:1, b:2}.keys → [a, b]  ;  .size → 2
        print_stmt(method(
            Expr::MapLit {
                entries: vec![
                    semantic_ir::nodes::MapEntry { key: slit("a"), value: ilit(1) },
                    semantic_ir::nodes::MapEntry { key: slit("b"), value: ilit(2) },
                ],
                span: s(),
            },
            "keys",
            vec![],
        )),
        // [1,2,3].map(&:to_s).join → "123"  (Symbol#to_proc)
        print_stmt(method(
            method(
                seq(vec![ilit(1), ilit(2), ilit(3)]),
                "map",
                vec![builtin("block_pass", vec![Expr::SymLit { name: "to_s".into(), span: s() }])],
            ),
            "join",
            vec![],
        )),
    ];

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };

    program(vec![double, is_even, add, main])
}

/// A module that calls an out-of-catalog method — must fail cleanly.
fn unknown_method_module() -> Module {
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![print_stmt(method(seq(vec![ilit(1)]), "bogus_xyz", vec![]))],
            value: Expr::NilLit { span: s() },
            span: s(),
        },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };
    program(vec![main])
}

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_go(source: &str, tag: &str) -> std::process::Output {
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_coll_{tag}_{nonce}.go"));
    std::fs::write(&src_path, source).expect("write temp source");
    let out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");
    let _ = std::fs::remove_file(&src_path);
    out
}

#[test]
fn collection_methods_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let artifact = compile(&catalog_module()).expect("module should compile to Go source");
    let run_out = run_go(&artifact.source, "catalog");
    if !run_out.status.success() {
        panic!(
            "emitted Go failed:\n--- stderr ---\n{}\n--- source ---\n{}",
            String::from_utf8_lossy(&run_out.stderr),
            artifact.source,
        );
    }
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec![
            "3",         // [1,2,3].length
            "[2, 4, 6]", // .map { |x| x*2 }
            "[2, 4]",    // .select { |x| x.even? }
            "10",        // .reduce(0) { a+b }
            "abc",       // .join
            "a-b",       // .join("-")
            "[1, 2, 3]", // .sort
            "[3, 2, 1]", // .reverse
            "HELLO",     // "hello".upcase
            "hello",     // "HeLLo".downcase
            "hi",        // "  hi  ".strip
            "[a, b, c]", // "a,b,c".split(",")
            "#f",        // 7.even?
            "#t",        // 8.even?
            "5",         // (-5).abs
            "[a, b]",    // {a:1,b:2}.keys
            "123",       // [1,2,3].map(&:to_s).join
        ],
        "unexpected stdout:\n{stdout}"
    );
}

#[test]
fn unknown_method_fails_cleanly() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let artifact = compile(&unknown_method_module()).expect("compiles to Go");
    let run_out = run_go(&artifact.source, "unknown");
    // A controlled failure: non-zero exit AND the "undefined method" message —
    // NOT a silent nil / garbage / arbitrary behaviour.
    assert!(
        !run_out.status.success(),
        "unknown method must fail, not succeed; stdout: {}",
        String::from_utf8_lossy(&run_out.stdout)
    );
    let stderr = String::from_utf8_lossy(&run_out.stderr);
    assert!(
        stderr.contains("undefined method") && stderr.contains("bogus_xyz"),
        "expected a controlled undefined-method panic; stderr:\n{stderr}"
    );
}

/// Block-taking Array methods added in the array-block-breadth batch.
fn array_block_module() -> Module {
    let id = lambda_fn("__blk_id", "x", var_p("x"));
    let even = lambda_fn("__blk_even", "x", method(var_p("x"), "even?", vec![]));
    let pair = lambda_fn("__blk_pair", "x", seq(vec![var_p("x"), var_p("x")]));
    let lt3 = lambda_fn("__blk_lt3", "x", builtin("<", vec![var_p("x"), ilit(3)]));
    let ewo = lambda_fn2(
        "__blk_ewo",
        "x",
        "o",
        method(var_p("o"), "push", vec![builtin("*", vec![var_p("x"), ilit(10)])]),
    );
    let stmts = vec![
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "sort_by", vec![closure("__blk_id")])),
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "min_by", vec![closure("__blk_id")])),
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "max_by", vec![closure("__blk_id")])),
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            "partition",
            vec![closure("__blk_even")],
        )),
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "flat_map", vec![closure("__blk_pair")])),
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            "take_while",
            vec![closure("__blk_lt3")],
        )),
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            "drop_while",
            vec![closure("__blk_lt3")],
        )),
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            "count",
            vec![closure("__blk_even")],
        )),
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3)]),
            "each_with_object",
            vec![seq(vec![]), closure("__blk_ewo")],
        )),
    ];
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };
    program(vec![id, even, pair, lt3, ewo, main])
}

#[test]
fn array_block_methods_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let artifact = compile(&array_block_module()).expect("module should compile to Go source");
    let run_out = run_go(&artifact.source, "arrblk");
    if !run_out.status.success() {
        panic!(
            "emitted Go failed:\n--- stderr ---\n{}\n--- source ---\n{}",
            String::from_utf8_lossy(&run_out.stderr),
            artifact.source,
        );
    }
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec![
            "[1, 2, 3]",          // sort_by { x }
            "1",                  // min_by { x }
            "3",                  // max_by { x }
            "[[2, 4], [1, 3]]",   // partition { even? }
            "[1, 1, 2, 2, 3, 3]", // flat_map { [x, x] }
            "[1, 2]",             // take_while { x < 3 }
            "[3, 4]",             // drop_while { x < 3 }
            "2",                  // count { even? }
            "[10, 20, 30]",       // each_with_object([]) { push x*10 }
        ],
        "unexpected stdout:\n{stdout}"
    );
}
