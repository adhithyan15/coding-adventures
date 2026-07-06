//! End-to-end proof for the C6 **aggregate / reshape Array methods** in the
//! Rust backend: `min`, `max`, `sum`, `uniq`, `flatten`, `compact`, `to_a`,
//! and the block-taking `each_with_index`.
//!
//! These join the collection catalog that reaches every backend as the
//! narrow-waist envelope
//! `BuiltinCall("__method__", [recv, StrLit("meth"), …args, block?])`, which
//! this backend emits as `__sir::call_method(recv, "meth", vec![…])` into an
//! inline `__sir` runtime.  The catalog is an EXPLICIT `(type, name)` match
//! (never reflection), ported from the Python/TypeScript `sir-runtime-oop`
//! reference for behavioural parity.
//!
//! This test hand-builds SIR modules that exercise each new method, emits
//! Rust, compiles it with `rustc`, runs the binary, and diffs stdout against
//! the values the Python/TS reference produces for the SAME SIR module:
//!
//!   * `[3, 1, 2].max`                       → `3`
//!   * `[3, 1, 2].min`                       → `1`
//!   * `[1, 2, 3].sum`                       → `6`
//!   * `[1, 2, 2, 3].uniq`                   → `[1, 2, 3]`
//!   * `[[1, [2]], 3].flatten`               → `[1, 2, 3]`
//!   * `[1, nil, 2].compact`                 → `[1, 2]`
//!   * `[1, 2, 3].to_a`                      → `[1, 2, 3]` (identity)
//!   * `[10, 20].each_with_index { |x, i| print x + i }`  → `10`, `21`
//!     (block sees element AND index; receiver returned)
//!
//! If `rustc` (or a usable linker) is unavailable the test logs a skip rather
//! than failing; a missing host tool must never redden a build.  The host can
//! point the test at a working linker via `SIR_TEST_RUSTC_LINKER` (e.g. the
//! toolchain's bundled `rust-lld`).

use std::process::Command;

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

fn nil() -> Expr {
    Expr::NilLit { span: s() }
}

fn param(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

/// `recv.meth(args…)` — the `__method__` dispatch envelope.
fn method(recv: Expr, name: &str, mut args: Vec<Expr>) -> Expr {
    let mut all = vec![recv, Expr::StrLit { value: name.into(), span: s() }];
    all.append(&mut args);
    Expr::BuiltinCall {
        name: "__method__".into(),
        args: all,
        effects: EffectSet::PURE,
        span: s(),
    }
}

/// A no-capture block closure over a top-level block function.
fn block(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: Vec::<CaptureValue>::new(), span: s() }
}

fn print_expr(expr: Expr) -> Expr {
    Expr::BuiltinCall {
        name: "print".into(),
        args: vec![expr],
        effects: EffectSet::PURE.with(Effect::MayPrint),
        span: s(),
    }
}

fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt { expr: print_expr(expr), span: s() }
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
        effects: EffectSet::PURE.with(Effect::MayPrint),
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
        name: "array_aggregates_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Strings,
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
    // { |x, i| print x + i } — proves the block receives (element, index).
    let block_fns =
        vec![block_fn("__blk_ewi", &["x", "i"], print_expr(call("+", vec![param("x"), param("i")])))];

    let main_stmts = vec![
        // 1. [3, 1, 2].max  → 3
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "max", vec![])),
        // 2. [3, 1, 2].min  → 1
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "min", vec![])),
        // 3. [1, 2, 3].sum  → 6
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "sum", vec![])),
        // 4. [1, 2, 2, 3].uniq  → [1, 2, 3]
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(2), ilit(3)]), "uniq", vec![])),
        // 5. [[1, [2]], 3].flatten  → [1, 2, 3]
        print_stmt(method(
            seq(vec![seq(vec![ilit(1), seq(vec![ilit(2)])]), ilit(3)]),
            "flatten",
            vec![],
        )),
        // 6. [1, nil, 2].compact  → [1, 2]
        print_stmt(method(seq(vec![ilit(1), nil(), ilit(2)]), "compact", vec![])),
        // 7. [1, 2, 3].to_a  → [1, 2, 3] (identity)
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "to_a", vec![])),
        // 8. [10, 20].each_with_index { |x, i| print x + i }
        //      → prints 10 (10+0) then 21 (20+1); the receiver is returned.
        print_stmt(method(
            seq(vec![ilit(10), ilit(20)]),
            "each_with_index",
            vec![block("__blk_ewi")],
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
fn array_aggregates_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&full_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_array_aggregates_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_array_aggregates_{nonce}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
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

    // Diff against the Python/TS reference output for the same SIR module.
    assert_eq!(
        lines,
        vec![
            "3",         // [3,1,2].max
            "1",         // [3,1,2].min
            "6",         // [1,2,3].sum
            "[1, 2, 3]", // [1,2,2,3].uniq
            "[1, 2, 3]", // [[1,[2]],3].flatten
            "[1, 2]",    // [1,nil,2].compact
            "[1, 2, 3]", // [1,2,3].to_a
            "10",        // each_with_index: 10 + 0
            "21",        // each_with_index: 20 + 1
            "[10, 20]",  // each_with_index returns receiver
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
