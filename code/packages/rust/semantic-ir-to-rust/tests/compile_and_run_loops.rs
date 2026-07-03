//! End-to-end proof for SIR16 **MutableBindings** + **Loops** in the
//! Rust backend.
//!
//! Unit tests assert the *shape* of the emitted source; this test goes
//! the whole way: it hand-builds a SIR module exercising mutable
//! reassignment, a `while` loop, and a `for-range` accumulator, emits
//! Rust, compiles it with `rustc`, runs the binary, and checks the
//! program's stdout.  That closes the loop the unit tests cannot — it
//! proves the emitted runtime actually *compiles and behaves*.
//!
//! `rustc` ships with every Rust toolchain (it is what `cargo` drives),
//! so this runs in CI.  If `rustc` is somehow unavailable the test logs
//! a skip rather than failing — we never want a missing tool to redden a
//! build for reasons unrelated to the change.

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata,
    Module, Scope, Span, Stmt,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn local(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
}

/// `(name arg0 arg1 ...)` builtin call, pure.
fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall {
        name: name.into(),
        args,
        effects: EffectSet::PURE,
        span: s(),
    }
}

/// `print(expr)` as an effectful statement.
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

fn let_local(name: &str, value: Expr) -> Stmt {
    Stmt::LetBinding {
        name: name.into(),
        sir_type: None,
        value,
        span: s(),
    }
}

fn assign_local(name: &str, value: Expr) -> Stmt {
    Stmt::Assign {
        name: name.into(),
        scope: Scope::Local,
        value,
        span: s(),
    }
}

fn block(stmts: Vec<Stmt>, value: Expr) -> Block {
    Block { stmts, value, span: s() }
}

/// Build a module whose `main`:
///
///  1. Uses a `for-range` accumulator to sum `0..5`  ⇒  10  → "10"
///  2. Uses a `while` loop to count `n` down from 3 to 0, printing the
///     final value (mutable reassignment of `n`)        → "0"
///  3. Prints a running product `1*1*2*3` via for-range with a mutated
///     accumulator                                       → "6"
fn demo_module() -> Module {
    // 1. sum = 0; for i in 0..5 { sum = sum + i }; print(sum)
    let sum_loop = vec![
        let_local("sum", ilit(0)),
        Stmt::ForRange {
            var: "i".into(),
            start: ilit(0),
            stop: ilit(5),
            step: ilit(1),
            body: block(
                vec![assign_local("sum", call("+", vec![local("sum"), local("i")]))],
                Expr::NilLit { span: s() },
            ),
            span: s(),
        },
        print_stmt(local("sum")),
    ];

    // 2. n = 3; while (n > 0) { n = n - 1 }; print(n)
    let countdown = vec![
        let_local("n", ilit(3)),
        Stmt::While {
            cond: call(">", vec![local("n"), ilit(0)]),
            body: block(
                vec![assign_local("n", call("-", vec![local("n"), ilit(1)]))],
                Expr::NilLit { span: s() },
            ),
            span: s(),
        },
        print_stmt(local("n")),
    ];

    // 3. prod = 1; for k in 1..4 { prod = prod * k }; print(prod)  ⇒ 6
    let product = vec![
        let_local("prod", ilit(1)),
        Stmt::ForRange {
            var: "k".into(),
            start: ilit(1),
            stop: ilit(4),
            step: ilit(1),
            body: block(
                vec![assign_local("prod", call("*", vec![local("prod"), local("k")]))],
                Expr::NilLit { span: s() },
            ),
            span: s(),
        },
        print_stmt(local("prod")),
    ];

    let mut stmts = Vec::new();
    stmts.extend(sum_loop);
    stmts.extend(countdown);
    stmts.extend(product);

    Module {
        name: "loops_demo".into(),
        manifest: FeatureManifest::from_features(&[Feature::Loops, Feature::MutableBindings]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: block(stmts, Expr::NilLit { span: s() }),
            effects: EffectSet::PURE.with(Effect::MayPrint),
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn loops_and_mutable_bindings_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    // 1. Emit.
    let artifact = compile(&demo_module()).expect("module should compile to Rust source");

    // 2. Write the source to a unique temp file.
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_loops_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_loops_{nonce}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    // 3. Compile with rustc.  `--edition 2021` is required: the emitted
    //    runtime uses raw identifiers (`r#fn`) and 2018+ closure capture.
    //
    //    Linker selection: this shells out to `rustc` directly, bypassing
    //    the workspace `.cargo/config.toml`.  CI provides a default MSVC
    //    linker (`link.exe`), so no override is needed there.  Hosts whose
    //    default linker is absent can point the test at a working one via
    //    `SIR_TEST_RUSTC_LINKER` (e.g. the toolchain's bundled `rust-lld`).
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
        // A *missing linker* is a host-environment issue, not a defect in
        // the emitted code — skip rather than redden the build.  Any other
        // compile failure (a genuine codegen bug) still fails the test.
        if stderr.contains("linker") && (stderr.contains("not found") || stderr.contains("No such file"))
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

    // 4. Run the binary and capture stdout.
    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero:\n{}",
        String::from_utf8_lossy(&run_out.stderr),
    );
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // 5. Assert the program's observable behaviour.
    //    sum 0..5 = 10, countdown ends at 0, product 1*1*2*3 = 6.
    assert_eq!(
        lines,
        vec!["10", "0", "6"],
        "unexpected program output; full stdout:\n{stdout}"
    );

    // 6. Best-effort cleanup (ignore errors — temp dir is ephemeral).
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
