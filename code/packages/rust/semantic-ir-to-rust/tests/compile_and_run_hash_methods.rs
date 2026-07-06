//! End-to-end proof for the **Hash-method dispatch** additions in the Rust
//! backend (parity with the Python/TypeScript `sir-runtime-oop` reference).
//!
//! Method calls reach every backend as the narrow-waist envelope
//! `BuiltinCall("__method__", [recv, StrLit("meth"), …args, block?])`.  This
//! backend emits `__sir::call_method(recv, "meth", vec![…])` into an inline
//! `__sir` runtime whose `map_method` implements the Hash catalog by an
//! EXPLICIT `match name` (never reflection).
//!
//! This test hand-builds SIR modules that exercise the newly-added Hash
//! methods, emits Rust, compiles it with `rustc`, runs the binary, and diffs
//! stdout against the values the Ruby/Python/TS reference produces for the SAME
//! SIR module:
//!
//!   * `{a:1}.merge({b:2})`         → `{a: 1, b: 2}`   (fresh, other-wins)
//!   * `{a:1, b:2}.to_a`            → `[[a, 1], [b, 2]]`
//!   * `{a:{b:1}}.dig(:a, :b)`      → `1`              (nested fetch)
//!   * `{a:1, b:2}.invert`          → `{1: a, 2: b}`
//!   * `{a:1, b:2}.delete(:a)`      → `1`  (returns removed value)
//!   * `{a:1, b:2, c:3}` each_value → prints 1,2,3 (one per line), returns self
//!   * `{a:10, b:20}` each_pair     → prints each value, returns self
//!
//! (The `{a:...}` map keys are Ruby symbols; a symbol prints bare — `a`, not
//! `:a` — through the runtime `format`, and a Hash prints every entry as
//! `key: value`, so a symbol-keyed hash renders `{a: 1}`.)
//!
//! If `rustc` (or a usable linker) is unavailable the test logs a skip rather
//! than failing; a missing host tool must never redden a build.  The host can
//! point the test at a working linker via `SIR_TEST_RUSTC_LINKER`.

use std::process::Command;

use semantic_ir::{
    Block, CaptureValue, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, MapEntry,
    Metadata, Module, Scope, Span, Stmt,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn symlit(v: &str) -> Expr {
    Expr::SymLit { name: v.into(), span: s() }
}

fn param(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

fn map(entries: Vec<(Expr, Expr)>) -> Expr {
    Expr::MapLit {
        entries: entries.into_iter().map(|(key, value)| MapEntry { key, value }).collect(),
        span: s(),
    }
}

/// `recv.meth(args…)` — the `__method__` dispatch envelope.
fn method(recv: Expr, name: &str, mut args: Vec<Expr>) -> Expr {
    let mut all = vec![recv, Expr::StrLit { value: name.into(), span: s() }];
    all.append(&mut args);
    Expr::BuiltinCall { name: "__method__".into(), args: all, effects: EffectSet::PURE, span: s() }
}

/// A no-capture block closure over a top-level block function.
fn block(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: Vec::<CaptureValue>::new(), span: s() }
}

/// `print(expr)` as an expression (the block-body value form).
fn print_expr(expr: Expr) -> Expr {
    Expr::BuiltinCall {
        name: "print".into(),
        args: vec![expr],
        effects: EffectSet::PURE.with(Effect::MayPrint),
        span: s(),
    }
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

fn block_fn(name: &str, params: &[&str], value: Expr) -> Function {
    Function {
        name: name.into(),
        params: params
            .iter()
            .map(|p| semantic_ir::Param {
                name: (*p).into(),
                kind: semantic_ir::ParamKind::Required,
                sir_type: None,
                default: None,
                span: s(),
            })
            .collect(),
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value, span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    }
}

fn demo_module(main_stmts: Vec<Stmt>, block_fns: Vec<Function>) -> Module {
    let mut functions = vec![Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts: main_stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    }];
    functions.extend(block_fns);

    Module {
        name: "hash_methods_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Maps,
            Feature::Strings,
            Feature::Symbols,
            Feature::Closures,
            Feature::DynamicTyping,
        ]),
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

fn full_demo() -> Module {
    // Block bodies referenced by the MakeClosures.  `each_value` yields one
    // value; `each_pair` yields `[k, v]` (two params).  Each block prints its
    // yielded value, so the ORDER of iteration is observable on stdout — an
    // accumulation proof that needs no captured/shared state.
    let block_fns = vec![
        // each_value: { |v| print v }
        block_fn("__blk_print_v", &["v"], print_expr(param("v"))),
        // each_pair:  { |k, v| print v }
        block_fn("__blk_print_pair_v", &["k", "v"], print_expr(param("v"))),
    ];

    let main_stmts = vec![
        // 1. {a:1}.merge({b:2})  → {a: 1, b: 2}  (fresh; other wins)
        print_stmt(method(
            map(vec![(symlit("a"), ilit(1))]),
            "merge",
            vec![map(vec![(symlit("b"), ilit(2))])],
        )),
        // 2. {a:1, b:2}.to_a  → [[a, 1], [b, 2]]
        print_stmt(method(
            map(vec![(symlit("a"), ilit(1)), (symlit("b"), ilit(2))]),
            "to_a",
            vec![],
        )),
        // 3. {a:{b:1}}.dig(:a, :b)  → 1  (nested fetch)
        print_stmt(method(
            map(vec![(symlit("a"), map(vec![(symlit("b"), ilit(1))]))]),
            "dig",
            vec![symlit("a"), symlit("b")],
        )),
        // 3b. {a:{b:1}}.dig(:a, :z)  → nil  (missing nested level)
        print_stmt(method(
            map(vec![(symlit("a"), map(vec![(symlit("b"), ilit(1))]))]),
            "dig",
            vec![symlit("a"), symlit("z")],
        )),
        // 4. {a:1, b:2}.invert  → {1: a, 2: b}
        print_stmt(method(
            map(vec![(symlit("a"), ilit(1)), (symlit("b"), ilit(2))]),
            "invert",
            vec![],
        )),
        // 5. {a:1, b:2}.delete(:a)  → 1  (returns removed value)
        print_stmt(method(
            map(vec![(symlit("a"), ilit(1)), (symlit("b"), ilit(2))]),
            "delete",
            vec![symlit("a")],
        )),
        // 6. each_value accumulation: prints 1, 2, 3 (one per line), in order.
        print_stmt(method(
            map(vec![(symlit("a"), ilit(1)), (symlit("b"), ilit(2)), (symlit("c"), ilit(3))]),
            "each_value",
            vec![block("__blk_print_v")],
        )),
        // 7. each_pair accumulation: prints each value (10, 20), in order.
        print_stmt(method(
            map(vec![(symlit("a"), ilit(10)), (symlit("b"), ilit(20))]),
            "each_pair",
            vec![block("__blk_print_pair_v")],
        )),
    ];

    demo_module(main_stmts, block_fns)
}

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn hash_methods_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&full_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_hash_methods_{nonce}.rs"));
    let bin_path =
        dir.join(format!("sir_hash_methods_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compile_out = cmd
        .arg(&src_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("invoke rustc");
    if !compile_out.status.success() {
        let stderr = String::from_utf8_lossy(&compile_out.stderr);
        if stderr.contains("linker")
            && (stderr.contains("not found") || stderr.contains("No such file"))
        {
            eprintln!("skipping: no usable linker on host\n{stderr}");
            let _ = std::fs::remove_file(&src_path);
            return;
        }
        panic!(
            "emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero:\n{}",
        String::from_utf8_lossy(&run_out.stderr),
    );
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(
        lines,
        vec![
            "{a: 1, b: 2}",     // merge (fresh, other wins)
            "[[a, 1], [b, 2]]", // to_a
            "1",                // dig(:a, :b) — nested
            "nil",              // dig(:a, :z) — missing level
            "{1: a, 2: b}",     // invert
            "1",                // delete(:a) returns removed value
            "1",                // each_value → block prints value a
            "2",                // each_value → block prints value b
            "3",                // each_value → block prints value c
            "{a: 1, b: 2, c: 3}", // each_value returns self (printed)
            "10",               // each_pair → block prints value a
            "20",               // each_pair → block prints value b
            "{a: 10, b: 20}",   // each_pair returns self (printed)
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
