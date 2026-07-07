//! End-to-end proof for the C6 **collection-method dispatch** runtime in
//! the Rust backend.
//!
//! Method calls reach every backend as the narrow-waist envelope
//! `BuiltinCall("__method__", [recv, StrLit("meth"), …args, block?])`.
//! This backend emits `__sir::call_method(recv, "meth", vec![…])` into an
//! inline `__sir` runtime whose `call_method` implements the collection
//! catalog by an EXPLICIT `(type, name)` match (Array/Map/String/Numeric),
//! ported from the Python/TypeScript `sir-runtime-oop` reference for
//! behavioural parity.
//!
//! Unit tests (in `emit.rs`) assert the *shape* of the emitted dispatch;
//! this test hand-builds SIR modules that exercise the catalog, emits Rust,
//! compiles it with `rustc`, runs the binary, and diffs stdout against the
//! values the Python/TS reference produces for the SAME SIR module:
//!
//!   * `[1,2,3].map { |x| x*2 }`            → `[2, 4, 6]`
//!   * `[1,2,3,4].select { |x| x.even? }`   → `[2, 4]`
//!   * `[1,2,3].length`                     → `3`
//!   * `[1,2,3].reduce(0) { |a,x| a+x }`    → `6`  (and `.inject` too)
//!   * `[1,2,3].map(&:to_s).join(",")`      → `"1,2,3"`  (`Symbol#to_proc`)
//!
//! (An *unknown* method now raises a typed `NoMethodError` rather than
//! flooring to `nil` — cascade `sir-typed-runtime-errors`, T5 — so that case
//! moved to `compile_and_run_typed_runtime_errors.rs`, which catches it.)
//!
//! `StrLit` interpolation is not accepted by the Rust backend, so — like
//! the other Rust exec-proof tests — assertions are on numeric / array /
//! joined-string results the `print` path renders.
//!
//! If `rustc` (or a usable linker) is unavailable the test logs a skip
//! rather than failing; a missing host tool must never redden a build.  The
//! host can point the test at a working linker via `SIR_TEST_RUSTC_LINKER`
//! (e.g. the toolchain's bundled `rust-lld`).

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

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn symlit(v: &str) -> Expr {
    Expr::SymLit { name: v.into(), span: s() }
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
    let mut all = vec![recv, slit(name)];
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

/// Assemble a module: a `main` printing each demo expression, plus the
/// block-body functions the `MakeClosure`s reference.
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
        name: "coll_methods_demo".into(),
        manifest: FeatureManifest::from_features(&[
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

fn full_demo() -> Module {
    // Block bodies referenced by the MakeClosures.
    let block_fns = vec![
        // { |x| x * 2 }
        block_fn("__blk_double", &["x"], call("*", vec![param("x"), ilit(2)])),
        // { |x| x.even? }
        block_fn("__blk_even", &["x"], method(param("x"), "even?", vec![])),
        // { |a, x| a + x }
        block_fn("__blk_sum", &["a", "x"], call("+", vec![param("a"), param("x")])),
    ];

    let main_stmts = vec![
        // 1. [1,2,3].map { |x| x*2 }  → [2, 4, 6]
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "map", vec![block("__blk_double")])),
        // 2. [1,2,3,4].select { |x| x.even? }  → [2, 4]
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            "select",
            vec![block("__blk_even")],
        )),
        // 3. [1,2,3].length  → 3
        print_stmt(method(seq(vec![ilit(1), ilit(2), ilit(3)]), "length", vec![])),
        // 4. [1,2,3].reduce(0) { |a,x| a+x }  → 6
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3)]),
            "reduce",
            vec![ilit(0), block("__blk_sum")],
        )),
        // 5. [1,2,3].inject { |a,x| a+x }  (seedless)  → 6
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3)]),
            "inject",
            vec![block("__blk_sum")],
        )),
        // 6. [1,2,3].map(&:to_s).join(",")  → "1,2,3"  (Symbol#to_proc)
        print_stmt(method(
            method(
                seq(vec![ilit(1), ilit(2), ilit(3)]),
                "map",
                vec![call("block_pass", vec![symlit("to_s")])],
            ),
            "join",
            vec![slit(",")],
        )),
        // 7. "hello".upcase  → "HELLO"
        print_stmt(method(slit("hello"), "upcase", vec![])),
        // 8. [3,1,2].sort  → [1, 2, 3]
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "sort", vec![])),
        // (An unknown method — `[1].bogus_xyz` — now raises a *typed*
        //  `NoMethodError` rather than flooring to `nil` (cascade
        //  `sir-typed-runtime-errors`, T5).  That surfaced boundary is proved
        //  end-to-end in `compile_and_run_typed_runtime_errors.rs`, which
        //  wraps it in `begin/rescue NoMethodError`; here we only exercise the
        //  *resolvable* catalog, so the closed dispatch stays observable
        //  without aborting this program.)
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
fn collection_methods_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&full_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_coll_methods_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_coll_methods_{nonce}{}",
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
            "[2, 4, 6]", // map { x*2 }
            "[2, 4]",    // select { even? }
            "3",         // length
            "6",         // reduce(0) { a+x }
            "6",         // inject { a+x }
            "1,2,3",     // map(&:to_s).join(",")
            "HELLO",     // "hello".upcase
            "[1, 2, 3]", // sort
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}

/// Block-taking Array methods added in the array-block-breadth batch:
/// `sort_by`, `min_by`/`max_by`, `partition`, `flat_map`, `take_while`/
/// `drop_while`, `count` (block + arg), and `each_with_object`.
fn block_breadth_demo() -> Module {
    let block_fns = vec![
        block_fn("__blk_id", &["x"], param("x")),
        block_fn("__blk_even", &["x"], method(param("x"), "even?", vec![])),
        block_fn("__blk_pair", &["x"], seq(vec![param("x"), param("x")])),
        block_fn("__blk_lt3", &["x"], call("<", vec![param("x"), ilit(3)])),
        block_fn(
            "__blk_ewo",
            &["x", "o"],
            method(param("o"), "push", vec![call("*", vec![param("x"), ilit(10)])]),
        ),
    ];
    let main_stmts = vec![
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "sort_by", vec![block("__blk_id")])),
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "min_by", vec![block("__blk_id")])),
        print_stmt(method(seq(vec![ilit(3), ilit(1), ilit(2)]), "max_by", vec![block("__blk_id")])),
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            "partition",
            vec![block("__blk_even")],
        )),
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3)]),
            "flat_map",
            vec![block("__blk_pair")],
        )),
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            "take_while",
            vec![block("__blk_lt3")],
        )),
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            "drop_while",
            vec![block("__blk_lt3")],
        )),
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3), ilit(4)]),
            "count",
            vec![block("__blk_even")],
        )),
        print_stmt(method(seq(vec![ilit(1), ilit(1), ilit(2), ilit(3)]), "count", vec![ilit(1)])),
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3)]),
            "each_with_object",
            vec![seq(vec![]), block("__blk_ewo")],
        )),
    ];
    demo_module(main_stmts, block_fns)
}

#[test]
fn array_block_methods_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let artifact = compile(&block_breadth_demo()).expect("module should compile to Rust source");
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_arrblk_{nonce}.rs"));
    let bin_path = dir.join(format!("sir_arrblk_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
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
        if stderr.contains("linker") && (stderr.contains("not found") || stderr.contains("No such file")) {
            eprintln!("skipping: no usable linker on host\n{stderr}");
            let _ = std::fs::remove_file(&src_path);
            return;
        }
        panic!("emitted Rust failed to compile:\n{stderr}\n--- source ---\n{}", artifact.source);
    }
    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(run_out.status.success(), "binary exited non-zero:\n{}", String::from_utf8_lossy(&run_out.stderr));
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec![
            "[1, 2, 3]",           // sort_by { x }
            "1",                   // min_by { x }
            "3",                   // max_by { x }
            "[[2, 4], [1, 3]]",    // partition { even? }
            "[1, 1, 2, 2, 3, 3]",  // flat_map { [x, x] }
            "[1, 2]",              // take_while { x < 3 }
            "[3, 4]",              // drop_while { x < 3 }
            "2",                   // count { even? }
            "2",                   // count(1)
            "[10, 20, 30]",        // each_with_object([]) { push x*10 }
        ],
        "unexpected output; full stdout:\n{stdout}"
    );
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
