//! End-to-end proof for the Go backend's **Array collection-method parity**
//! additions — `min`, `max`, `sum`, `uniq`, `flatten`, `compact`, `to_a`, and
//! `each_with_index` — bringing the Go runtime level with the Python/TS
//! `sir-runtime-oop` catalogs.
//!
//! Like `compile_and_run_coll_methods.rs`, this hand-builds SIR modules that
//! exercise the new catalog entries, emits Go, runs it under a real `go run`,
//! and diffs stdout against the values the Python/TS reference backends yield
//! for the SAME operation (`[3,1,2].max` → `3`, `[1,2,2,3].uniq` → `[1,2,3]`,
//! `[[1,[2]],3].flatten` → `[1,2,3]`, `[1,nil,2].compact` → `[1,2]`,
//! `[1,2,3].sum` → `6`, `[10,20].each_with_index { |x,i| ... }` → the pairs).
//!
//! Gated on `go version`: a missing toolchain logs a skip rather than
//! reddening the build (mirrors `compile_and_run_coll_methods.rs`).

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

fn nil() -> Expr {
    Expr::NilLit { span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn var_p(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
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

fn closure(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: vec![], span: s() }
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

/// `puts arg` — used by the `each_with_index` block so it emits one line per
/// pair (interpolation is out of the Go backend's accepted set, so we `puts`
/// a pre-joined string built with `+`).
fn puts_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "puts".into(),
            args: vec![expr],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// Two-parameter lambda (for `each_with_index`), registered top-level.
fn lambda_fn2(fn_name: &str, p0: &str, p1: &str, body: Block) -> Function {
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
        body,
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    }
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
        name: "array_methods_demo".into(),
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

/// The parity demo `main`.  Each printed line's expected value is what the
/// Python/TS reference runtime yields for the identical operation.
fn catalog_module() -> Module {
    // `each_with_index` block: `|x, i| puts (i.to_s + ":" + x.to_s)`.
    // Builds "0:10", "1:20" — proving both element AND index reach the block.
    let ewi_body = Block {
        stmts: vec![puts_stmt(builtin(
            "+",
            vec![
                builtin(
                    "+",
                    vec![method(var_p("i"), "to_s", vec![]), slit(":")],
                ),
                method(var_p("x"), "to_s", vec![]),
            ],
        ))],
        value: nil(),
        span: s(),
    };
    let ewi = lambda_fn2("__lam_ewi", "x", "i", ewi_body);

    let stmts = vec![
        // [3,1,2].min → 1
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "min", vec![])),
        // [3,1,2].max → 3
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "max", vec![])),
        // [].max → nil
        print_stmt(method(seq(vec![]), "max", vec![])),
        // [1,2,3].sum → 6
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "sum", vec![])),
        // [].sum → 0
        print_stmt(method(seq(vec![]), "sum", vec![])),
        // [1,2,2,3,1].uniq → [1, 2, 3]  (first-occurrence order)
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(2), ilit(3), ilit(1)]),
            "uniq",
            vec![],
        )),
        // [[1,[2]],3].flatten → [1, 2, 3]  (recursive)
        print_stmt(method(
            seq(vec![seq(vec![ilit(1), seq(vec![ilit(2)])]), ilit(3)]),
            "flatten",
            vec![],
        )),
        // [1,nil,2,nil].compact → [1, 2]
        print_stmt(method(
            seq(vec![ilit(1), nil(), ilit(2), nil()]),
            "compact",
            vec![],
        )),
        // [1,2,3].to_a → [1, 2, 3]  (returns self)
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "to_a", vec![])),
        // [10,20].each_with_index { |x,i| puts "#{i}:#{x}" }  → 0:10 / 1:20
        print_stmt(method(
            seq(vec![ilit(10), ilit(20)]),
            "each_with_index",
            vec![closure("__lam_ewi")],
        )),
    ];

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts, value: nil(), span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };

    program(vec![ewi, main])
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
    let src_path = dir.join(format!("sir_go_arr_{tag}_{nonce}.go"));
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
fn array_methods_compile_and_run() {
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
            "1",         // [3,1,2].min
            "3",         // [3,1,2].max
            "nil",       // [].max  (print of nil is "nil")
            "6",         // [1,2,3].sum
            "0",         // [].sum
            "[1, 2, 3]", // [1,2,2,3,1].uniq
            "[1, 2, 3]", // [[1,[2]],3].flatten
            "[1, 2]",    // [1,nil,2,nil].compact
            "[1, 2, 3]", // [1,2,3].to_a
            "0:10",      // each_with_index pair 0
            "1:20",      // each_with_index pair 1
            "[10, 20]",  // each_with_index returns the receiver (then print's it)
        ],
        "unexpected stdout:\n{stdout}"
    );
}

// ── Array each_slice / each_cons / chunk_while ─────────────────────────────
//
// The consecutive-grouping family, mirroring the Python reference (#8031).
// `each_slice`/`each_cons` are non-block (take an int `n`); `chunk_while` is a
// block method (the block is called on each ADJACENT pair).
fn slice_module() -> Function {
    Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![
                // [1,2,3,4,5].each_slice(2) → [[1, 2], [3, 4], [5]]
                print_stmt(method(
                    seq(vec![ilit(1), ilit(2), ilit(3), ilit(4), ilit(5)]),
                    "each_slice",
                    vec![ilit(2)],
                )),
                // [1,2,3].each_slice(0) → []  (never-panic floor)
                print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "each_slice", vec![ilit(0)])),
                // [1,2,3,4].each_cons(2) → [[1, 2], [2, 3], [3, 4]]
                print_stmt(method(
                    seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
                    "each_cons",
                    vec![ilit(2)],
                )),
                // [1,2].each_cons(3) → []  (window larger than the array)
                print_stmt(method(seq(vec![ilit(1), ilit(2)]), "each_cons", vec![ilit(3)])),
                // [1,2,4,5,7].chunk_while { |a,b| b-a==1 } → [[1, 2], [4, 5], [7]]
                print_stmt(method(
                    seq(vec![ilit(1), ilit(2), ilit(4), ilit(5), ilit(7)]),
                    "chunk_while",
                    vec![closure("__lam_adj")],
                )),
                // [].chunk_while { … } → []
                print_stmt(method(seq(vec![]), "chunk_while", vec![closure("__lam_adj")])),
            ],
            value: nil(),
            span: s(),
        },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    }
}

#[test]
fn array_each_slice_each_cons_chunk_while_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let adj = lambda_fn2(
        "__lam_adj",
        "a",
        "b",
        Block {
            stmts: vec![],
            value: builtin("=", vec![builtin("-", vec![var_p("b"), var_p("a")]), ilit(1)]),
            span: s(),
        },
    );
    let module = program(vec![slice_module(), adj]);
    let artifact = compile(&module).expect("module should compile to Go source");
    let run_out = run_go(&artifact.source, "slice");
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
            "[[1, 2], [3, 4], [5]]",    // each_slice(2)
            "[]",                       // each_slice(0) → []
            "[[1, 2], [2, 3], [3, 4]]", // each_cons(2)
            "[]",                       // each_cons(3) on len-2 → []
            "[[1, 2], [4, 5], [7]]",    // chunk_while { b-a==1 }
            "[]",                       // [].chunk_while → []
        ],
        "unexpected stdout:\n{stdout}"
    );
}
