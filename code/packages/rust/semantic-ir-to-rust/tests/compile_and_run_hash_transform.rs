//! End-to-end proof for the Rust backend's **`Hash#transform_values` /
//! `Hash#transform_keys`** block methods, mirroring the Python
//! `sir-runtime-oop` reference (PR #7909).
//!
//! Both are non-mutating: they build a FRESH hash and yield exactly ONE block
//! argument per entry —
//!
//!   * `transform_values { |v| … }` replaces every value with the block result
//!     while copying keys verbatim (keys stay unique ⇒ no collision), so
//!     `{a:1, b:2}.transform_values { 99 }` ⇒ `{a: 99, b: 99}`.
//!   * `transform_keys { |k| … }` replaces every key with the block result
//!     while leaving values untouched.  Two source keys can collapse onto one
//!     new key; Ruby keeps the LAST such entry's value at the FIRST-seen
//!     position, so `{a:1, b:2}.transform_keys { :z }` ⇒ `{z: 2}`.
//!
//! The test hand-builds SIR modules that exercise these arms, emits Rust,
//! compiles it with `rustc`, runs the binary, and diffs stdout against the
//! values the Python/TS reference produces for the SAME operations.
//!
//! Like the sibling exec-proof tests, a missing `rustc`/linker logs a skip
//! rather than reddening the build; the host may point at a working linker via
//! `SIR_TEST_RUSTC_LINKER` (e.g. the toolchain's bundled `rust-lld`).

use std::process::Command;

use semantic_ir::nodes::MapEntry;
use semantic_ir::{
    Block, CaptureValue, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata,
    Module, Scope, Span, Stmt,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn symlit(v: &str) -> Expr {
    Expr::SymLit { name: v.into(), span: s() }
}

fn param(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

/// `recv.meth(args…)` — the `__method__` dispatch envelope.
fn method(recv: Expr, name: &str, mut args: Vec<Expr>) -> Expr {
    let mut all = vec![recv, slit(name)];
    all.append(&mut args);
    Expr::BuiltinCall { name: "__method__".into(), args: all, effects: EffectSet::PURE, span: s() }
}

/// A no-capture block closure over a top-level block function.
fn block(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: Vec::<CaptureValue>::new(), span: s() }
}

/// `{k: v, …}` from `(key, value)` expression pairs.
fn map_lit(entries: Vec<(Expr, Expr)>) -> Expr {
    Expr::MapLit {
        entries: entries.into_iter().map(|(key, value)| MapEntry { key, value }).collect(),
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
        name: "hash_transform_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Maps,
            Feature::Sequences,
            Feature::Strings,
            Feature::Symbols,
            Feature::Closures,
            // Block-body functions have untyped params.
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

/// `{a: 1, b: 2}` with symbol keys (so the printed form is `a`/`b`, matching
/// Ruby's surface — a `Symbol` renders bare).
fn ab_map() -> Expr {
    map_lit(vec![(symlit("a"), ilit(1)), (symlit("b"), ilit(2))])
}

fn transform_demo() -> Module {
    let block_fns = vec![
        // transform_values { |v| 99 } — constant body ⇒ predictable values.
        block_fn("__blk_const99", &["v"], ilit(99)),
        // transform_keys { |k| k } — identity ⇒ a faithful, non-colliding copy.
        block_fn("__blk_id", &["k"], param("k")),
        // transform_keys { |k| :z } — constant key ⇒ every entry collides.
        block_fn("__blk_const_z", &["k"], symlit("z")),
    ];

    let main_stmts = vec![
        // 1. {a:1,b:2}.transform_values { 99 } → {a: 99, b: 99}  (keys untouched)
        print_stmt(method(ab_map(), "transform_values", vec![block("__blk_const99")])),
        // 2. {a:1,b:2}.transform_keys { |k| k } → {a: 1, b: 2}  (values untouched)
        print_stmt(method(ab_map(), "transform_keys", vec![block("__blk_id")])),
        // 3. {a:1,b:2}.transform_keys { :z } → {z: 2}  (collision → last value wins)
        print_stmt(method(ab_map(), "transform_keys", vec![block("__blk_const_z")])),
    ];

    demo_module(main_stmts, block_fns)
}

fn rustc_available() -> bool {
    Command::new("rustc").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn hash_transform_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&transform_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_hash_transform_{nonce}.rs"));
    let bin_path =
        dir.join(format!("sir_hash_transform_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compile_out =
        cmd.arg(&src_path).arg("-o").arg(&bin_path).output().expect("invoke rustc");
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
            "{a: 99, b: 99}", // transform_values (keys untouched)
            "{a: 1, b: 2}",   // transform_keys identity (values untouched)
            "{z: 2}",         // transform_keys collision (last value wins)
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
