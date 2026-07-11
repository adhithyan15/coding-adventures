//! End-to-end proof for the Rust backend's **`Hash` catalog catch-up** — the
//! non-block methods `empty?`, `to_a`, `merge`, `dig` (nested), `invert`,
//! `store`/`[]=`, `delete`, `clear`, and the block methods `reject`,
//! `each_key`, `each_value`, bringing the Rust `map_method` catalog to parity
//! with the Go/JS/TS/Python `sir-runtime-oop` reference.
//!
//! The test hand-builds SIR modules that exercise each arm, emits Rust,
//! compiles it with `rustc`, runs the binary, and diffs stdout against the
//! values the Python reference produces for the SAME operations.  Symbol keys
//! (`{a: 1}`) are used so the printed form (`a`) matches Ruby's surface.
//!
//! A missing `rustc`/linker logs a skip rather than reddening the build; the
//! host may point at a working linker via `SIR_TEST_RUSTC_LINKER`.

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

/// A one-statement block that `print`s its parameter and returns nil — used by
/// the `each_key` / `each_value` demos so the yielded values are observable.
fn block_fn_print(name: &str, pname: &str) -> Function {
    Function {
        name: name.into(),
        params: vec![semantic_ir::Param {
            name: pname.into(),
            kind: semantic_ir::ParamKind::Required,
            sir_type: None,
            default: None,
            span: s(),
        }],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![print_stmt(param(pname))],
            value: Expr::NilLit { span: s() },
            span: s(),
        },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    }
}

/// A pure expression-bodied block over the given params.
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
        name: "hash_catalog_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Maps,
            Feature::Sequences,
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

/// `{a: 1, b: 2}` with symbol keys.
fn ab_map() -> Expr {
    map_lit(vec![(symlit("a"), ilit(1)), (symlit("b"), ilit(2))])
}

fn catalog_demo() -> Module {
    // `reject { |k, v| v.even? }` drops the even-valued pairs.
    let reject_even = block_fn("__blk_v_even", &["k", "v"], method(param("v"), "even?", vec![]));
    let block_fns = vec![
        reject_even,
        block_fn_print("__blk_print", "x"),
    ];

    // `{a: {b: 1}}` — a nested hash for the `dig` walk.
    let nested = || map_lit(vec![(symlit("a"), map_lit(vec![(symlit("b"), ilit(1))]))]);

    let main_stmts = vec![
        // empty? — non-empty ⇒ #f, empty ⇒ #t
        print_stmt(method(ab_map(), "empty?", vec![])),
        print_stmt(method(map_lit(vec![]), "empty?", vec![])),
        // to_a → [[a, 1], [b, 2]]
        print_stmt(method(ab_map(), "to_a", vec![])),
        // {a:1}.merge({b:2}) → {a: 1, b: 2}
        print_stmt(method(
            map_lit(vec![(symlit("a"), ilit(1))]),
            "merge",
            vec![map_lit(vec![(symlit("b"), ilit(2))])],
        )),
        // {a:1,b:2}.merge({b:9}) → {a: 1, b: 9}  (other wins on collision)
        print_stmt(method(ab_map(), "merge", vec![map_lit(vec![(symlit("b"), ilit(9))])])),
        // {a:{b:1}}.dig(:a,:b) → 1  (nested hit)
        print_stmt(method(nested(), "dig", vec![symlit("a"), symlit("b")])),
        // {a:{b:1}}.dig(:a,:z) → nil  (nested miss)
        print_stmt(method(nested(), "dig", vec![symlit("a"), symlit("z")])),
        // {a:1,b:2}.invert → {1: a, 2: b}
        print_stmt(method(ab_map(), "invert", vec![])),
        // {a:1,b:2}.store(:c, 3) → 3  (returns stored value)
        print_stmt(method(ab_map(), "store", vec![symlit("c"), ilit(3)])),
        // {a:1,b:2}.delete(:a) → 1  (returns removed value)
        print_stmt(method(ab_map(), "delete", vec![symlit("a")])),
        // {a:1}.delete(:z) → nil  (absent key)
        print_stmt(method(map_lit(vec![(symlit("a"), ilit(1))]), "delete", vec![symlit("z")])),
        // {a:1,b:2}.clear → {}
        print_stmt(method(ab_map(), "clear", vec![])),
        // {a:1,b:2}.reject { |k, v| v.even? } → {a: 1}
        print_stmt(method(ab_map(), "reject", vec![block("__blk_v_even")])),
        // {a:1,b:2}.each_key { |k| print k } → prints a, b, returns {a: 1, b: 2}
        print_stmt(method(ab_map(), "each_key", vec![block("__blk_print")])),
        // {a:1,b:2}.each_value { |v| print v } → prints 1, 2, returns {a: 1, b: 2}
        print_stmt(method(ab_map(), "each_value", vec![block("__blk_print")])),
    ];

    demo_module(main_stmts, block_fns)
}

fn rustc_available() -> bool {
    Command::new("rustc").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn hash_catalog_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&catalog_demo()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_hash_catalog_{nonce}.rs"));
    let bin_path =
        dir.join(format!("sir_hash_catalog_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
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
            "#f",               // empty? on non-empty
            "#t",               // empty? on {}
            "[[a, 1], [b, 2]]", // to_a
            "{a: 1, b: 2}",     // merge (append)
            "{a: 1, b: 9}",     // merge (collision → other wins)
            "1",                // dig nested hit
            "nil",              // dig nested miss
            "{1: a, 2: b}",     // invert
            "3",                // store → stored value
            "1",                // delete → removed value
            "nil",              // delete absent → nil
            "{}",               // clear
            "{a: 1}",           // reject { even? } drops b:2
            "a",                // each_key yields a
            "b",                // each_key yields b
            "{a: 1, b: 2}",     // each_key returns self
            "1",                // each_value yields 1
            "2",                // each_value yields 2
            "{a: 1, b: 2}",     // each_value returns self
        ],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
