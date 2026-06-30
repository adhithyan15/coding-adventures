//! End-to-end proof for SIR16 **Floats** + **ShortCircuit** in the Rust
//! backend.
//!
//! Unit tests assert the *shape* of the emitted source; this test goes
//! the whole way: it hand-builds a SIR module exercising both features,
//! emits Rust, compiles it with `rustc`, runs the binary, and checks the
//! program's stdout.  That closes the loop the unit tests cannot — it
//! proves the emitted runtime actually *compiles and behaves*.
//!
//! `rustc` ships with every Rust toolchain (it is what `cargo` drives),
//! so this runs in CI.  If `rustc` is somehow unavailable the test logs
//! a skip rather than failing — we never want a missing tool to redden a
//! build for reasons unrelated to the change.

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, FeatureManifest, Feature, Function, Metadata,
    Module, Span,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn flit(v: f64) -> Expr {
    Expr::FloatLit { value: v, span: s() }
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}
fn blit(v: bool) -> Expr {
    Expr::BoolLit { value: v, span: s() }
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
fn print_stmt(expr: Expr) -> semantic_ir::Stmt {
    semantic_ir::Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "print".into(),
            args: vec![expr],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// Build a module whose `main` prints, in order, the results of a small
/// battery of float + short-circuit expressions.
fn demo_module() -> Module {
    let stmts = vec![
        // 1.5 + 2.5  ⇒  Float(4.0)            → "4.0"
        print_stmt(call("+", vec![flit(1.5), flit(2.5)])),
        // 5.0 - 1    ⇒  int promoted to float → "4.0"
        print_stmt(call("-", vec![flit(5.0), ilit(1)])),
        // true && 5  ⇒  rhs (lhs truthy)      → "5"
        print_stmt(Expr::LogicalAnd {
            lhs: Box::new(blit(true)),
            rhs: Box::new(ilit(5)),
            span: s(),
        }),
        // false || 7 ⇒  rhs (lhs falsy)       → "7"
        print_stmt(Expr::LogicalOr {
            lhs: Box::new(blit(false)),
            rhs: Box::new(ilit(7)),
            span: s(),
        }),
        // false && 9 ⇒  lhs (short-circuits)  → "#f"
        print_stmt(Expr::LogicalAnd {
            lhs: Box::new(blit(false)),
            rhs: Box::new(ilit(9)),
            span: s(),
        }),
        // 1 = 1.0    ⇒  cross-type eq is true → "#t"
        print_stmt(call("=", vec![ilit(1), flit(1.0)])),
    ];

    Module {
        name: "floats_demo".into(),
        manifest: FeatureManifest::from_features(&[Feature::Floats, Feature::ShortCircuit]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts,
                value: Expr::NilLit { span: s() },
                span: s(),
            },
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
fn floats_and_short_circuit_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    // 1. Emit.
    let artifact = compile(&demo_module()).expect("module should compile to Rust source");

    // 2. Write the source to a unique temp file.
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_floats_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_floats_{nonce}{}",
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
    assert_eq!(
        lines,
        vec!["4.0", "4.0", "5", "7", "#f", "#t"],
        "unexpected program output; full stdout:\n{stdout}"
    );

    // 6. Best-effort cleanup (ignore errors — temp dir is ephemeral).
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
