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

/// The v0.22.0 **more non-block Array methods**: `zip`, `rotate`, `to_h`,
/// `tally`.  Same dispatch envelope, same explicit `(type, name)` catalog.
/// Assertions stay integer-keyed and never PRINT a `nil` / string element, so
/// the proof is independent of the display convention (this module's
/// `source_language` is `"test"`, i.e. the Lisp default):
///
///   * `[1, 2, 3].zip([4, 5, 6])`         → `[[1, 4], [2, 5], [3, 6]]`
///   * `[1, 2, 3].zip([7]).length`        → `3`  (result length = receiver's)
///   * `[1, 2, 3].zip([7]).last.length`   → `2`  (short operand PADS to 2-tuple)
///   * `[1, 2, 3, 4, 5].rotate(2)`        → `[3, 4, 5, 1, 2]`
///   * `[1, 2, 3].rotate`                 → `[2, 3, 1]`  (default n = 1)
///   * `[1, 2, 3].rotate(-1)`             → `[3, 1, 2]`  (negative rotates right)
///   * `[[1, 10], [2, 20]].to_h.fetch(2)` → `20`  (pairs → Hash)
///   * `[1, 1, 2, 1].tally.fetch(1)`      → `3`   (occurrence count)
///   * `[1, 1, 2, 1].tally.fetch(2)`      → `1`
fn more_methods_demo() -> Module {
    // `[1, 2, 3].zip([7])` — a 3-element receiver, 1-element operand.
    let short_zip =
        method(seq(vec![ilit(1), ilit(2), ilit(3)]), "zip", vec![seq(vec![ilit(7)])]);

    let main_stmts = vec![
        // 1. equal-length zip
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3)]),
            "zip",
            vec![seq(vec![ilit(4), ilit(5), ilit(6)])],
        )),
        // 2. result length is the receiver's
        print_stmt(method(short_zip.clone(), "length", vec![])),
        // 3. a short operand pads the tuple to width 2 (checked via length,
        //    NOT by printing the padding `nil`)
        print_stmt(method(method(short_zip, "last", vec![]), "length", vec![])),
        // 4. rotate(2)
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4), ilit(5)]),
            "rotate",
            vec![ilit(2)],
        )),
        // 5. rotate with the default n = 1
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "rotate", vec![])),
        // 6. negative n rotates right
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "rotate", vec![ilit(-1)])),
        // 7. to_h then look a value up by key
        print_stmt(method(
            method(
                seq(vec![seq(vec![ilit(1), ilit(10)]), seq(vec![ilit(2), ilit(20)])]),
                "to_h",
                vec![],
            ),
            "fetch",
            vec![ilit(2)],
        )),
        // 8/9. tally then read two counts
        print_stmt(method(
            method(seq(vec![ilit(1), ilit(1), ilit(2), ilit(1)]), "tally", vec![]),
            "fetch",
            vec![ilit(1)],
        )),
        print_stmt(method(
            method(seq(vec![ilit(1), ilit(1), ilit(2), ilit(1)]), "tally", vec![]),
            "fetch",
            vec![ilit(2)],
        )),
    ];

    demo_module(main_stmts, vec![])
}

#[test]
fn array_more_methods_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&more_methods_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_array_more_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_array_more_{nonce}{}",
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

    assert_eq!(
        lines,
        vec![
            "[[1, 4], [2, 5], [3, 6]]", // zip (equal length)
            "3",                        // zip result length = receiver's
            "2",                        // short operand pads tuple to width 2
            "[3, 4, 5, 1, 2]",          // rotate(2)
            "[2, 3, 1]",                // rotate (default 1)
            "[3, 1, 2]",                // rotate(-1) (right)
            "20",                       // to_h.fetch(2)
            "3",                        // tally.fetch(1)
            "1",                        // tally.fetch(2)
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}

/// The v0.23.0 **slice-selection Array methods**: `take`, `drop`, `values_at`.
/// All index-clamped / bounds-guarded and never panic.  Assertions stay
/// integer-valued and never print a `nil` element, so the proof is independent
/// of the display convention (this module's `source_language` is `"test"`).
fn take_drop_demo() -> Module {
    let main_stmts = vec![
        // take(2) of a 5-element array
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3), ilit(4), ilit(5)]), "take", vec![ilit(2)])),
        // take(9) clamps to a full copy (n > len)
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "take", vec![ilit(9)])),
        // drop(2)
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3), ilit(4), ilit(5)]), "drop", vec![ilit(2)])),
        // drop(9) -> [] (n >= len)
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "drop", vec![ilit(9)])),
        // values_at(0, 2, -1) -> [10, 30, 30] (negative folds from the end)
        print_stmt(method(
            seq(vec![ilit(10), ilit(20), ilit(30)]),
            "values_at",
            vec![ilit(0), ilit(2), ilit(-1)],
        )),
    ];
    demo_module(main_stmts, vec![])
}

#[test]
fn take_drop_values_at_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&take_drop_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_take_drop_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_take_drop_{nonce}{}",
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

    assert_eq!(
        lines,
        vec![
            "[1, 2]",       // take(2)
            "[1, 2, 3]",    // take(9) clamps
            "[3, 4, 5]",    // drop(2)
            "[]",           // drop(9) -> empty
            "[10, 30, 30]", // values_at(0, 2, -1)
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}

// ── Array each_slice / each_cons / chunk_while ─────────────────────────────
//
// The consecutive-grouping family, mirroring the Python reference (#8031) and
// the Go backend (#8036).  `each_slice`/`each_cons` are non-block (take an int
// `n`); `chunk_while` is a block method (the block is called on each ADJACENT
// pair).
fn slice_demo() -> Module {
    // `{ |a, b| b - a == 1 }` — adjacent-pair predicate (SIR equality builtin is
    // `=`, subtraction `-`).
    let adj = block_fn("__b_adj", &["a", "b"], call("=", vec![call("-", vec![param("b"), param("a")]), ilit(1)]));
    let main_stmts = vec![
        // [1,2,3,4,5].each_slice(2) → [[1, 2], [3, 4], [5]]
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4), ilit(5)]),
            "each_slice",
            vec![ilit(2)],
        )),
        // [1,2,3].each_slice(0) → []  (never-panic floor)
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "each_slice", vec![ilit(0)])),
        // [1,2,3,4].each_cons(2) → [[1, 2], [2, 3], [3, 4]]
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]), "each_cons", vec![ilit(2)])),
        // [1,2].each_cons(3) → []  (window larger than the array)
        print_stmt(method(seq(vec![ilit(1), ilit(2)]), "each_cons", vec![ilit(3)])),
        // [1,2,4,5,7].chunk_while { |a,b| b-a==1 } → [[1, 2], [4, 5], [7]]
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(4), ilit(5), ilit(7)]),
            "chunk_while",
            vec![block("__b_adj")],
        )),
        // [].chunk_while { … } → []
        print_stmt(method(seq(vec![]), "chunk_while", vec![block("__b_adj")])),
    ];
    demo_module(main_stmts, vec![adj])
}

#[test]
fn array_each_slice_each_cons_chunk_while_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&slice_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_arr_slice_{nonce}.rs"));
    let bin_path =
        dir.join(format!("sir_arr_slice_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compile_out = cmd.arg(&src_path).arg("-o").arg(&bin_path).output().expect("invoke rustc");
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
            "[[1, 2], [3, 4], [5]]",    // each_slice(2)
            "[]",                       // each_slice(0) → []
            "[[1, 2], [2, 3], [3, 4]]", // each_cons(2)
            "[]",                       // each_cons(3) on len-2 → []
            "[[1, 2], [4, 5], [7]]",    // chunk_while { b-a==1 }
            "[]",                       // [].chunk_while → []
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}

// ── Array slice_when ───────────────────────────────────────────────────────
//
// `slice_when { |a, b| pred }` is the INVERSE of `chunk_while`: it starts a NEW
// run BETWEEN an adjacent pair exactly WHERE the block is truthy.  Mirrors the
// Python reference (#8070) and the Go backend (#8073).
fn slice_when_demo() -> Module {
    // `{ |a, b| b - a > 1 }` — split on an upward gap greater than one.
    let gap = block_fn("__b_gap", &["a", "b"], call(">", vec![call("-", vec![param("b"), param("a")]), ilit(1)]));
    let main_stmts = vec![
        // [1,2,4,9,10,11,12].slice_when { |a,b| b-a>1 } → [[1, 2], [4], [9, 10, 11, 12]]
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(4), ilit(9), ilit(10), ilit(11), ilit(12)]),
            "slice_when",
            vec![block("__b_gap")],
        )),
        // [9].slice_when { … } → [[9]]  (single element)
        print_stmt(method(seq(vec![ilit(9)]), "slice_when", vec![block("__b_gap")])),
        // [].slice_when { … } → []
        print_stmt(method(seq(vec![]), "slice_when", vec![block("__b_gap")])),
    ];
    demo_module(main_stmts, vec![gap])
}

#[test]
fn array_slice_when_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&slice_when_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_arr_slicewhen_{nonce}.rs"));
    let bin_path =
        dir.join(format!("sir_arr_slicewhen_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compile_out = cmd.arg(&src_path).arg("-o").arg(&bin_path).output().expect("invoke rustc");
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
            "[[1, 2], [4], [9, 10, 11, 12]]", // slice_when { b-a>1 }
            "[[9]]",                          // single element
            "[]",                             // [].slice_when → []
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
