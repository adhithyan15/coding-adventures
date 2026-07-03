//! End-to-end proof for SIR16 **Floats** + **ShortCircuit** in the Go
//! backend.
//!
//! Unit tests assert the *shape* of the emitted source; this test goes
//! the whole way: it hand-builds a SIR module exercising both features,
//! emits Go, writes it to a temp `.go` file, runs it with `go run`, and
//! checks the program's stdout.  That closes the loop the unit tests
//! cannot — it proves the emitted runtime actually *compiles and
//! behaves* under a real Go toolchain.
//!
//! The test gates on `go` being available (`go version`).  If the Go
//! toolchain is absent the test logs a skip rather than failing — we
//! never want a missing tool to redden a build for reasons unrelated to
//! the change (mirrors how the Rust backend's equivalent test gates on
//! `rustc`).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
};
use semantic_ir_to_go::compile;

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
        // 1.5 + 2.5  ⇒  float(4.0)             → "4.0"
        print_stmt(call("+", vec![flit(1.5), flit(2.5)])),
        // 5.0 - 1    ⇒  int promoted to float  → "4.0"
        print_stmt(call("-", vec![flit(5.0), ilit(1)])),
        // true and 5 ⇒  rhs (lhs truthy)       → "5"
        print_stmt(Expr::LogicalAnd {
            lhs: Box::new(blit(true)),
            rhs: Box::new(ilit(5)),
            span: s(),
        }),
        // false or 7 ⇒  rhs (lhs falsy)        → "7"
        print_stmt(Expr::LogicalOr {
            lhs: Box::new(blit(false)),
            rhs: Box::new(ilit(7)),
            span: s(),
        }),
        // false and 9 ⇒ lhs (short-circuits)   → "#f"
        print_stmt(Expr::LogicalAnd {
            lhs: Box::new(blit(false)),
            rhs: Box::new(ilit(9)),
            span: s(),
        }),
        // 1 = 1.0    ⇒  cross-type eq is true  → "#t"
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

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn floats_and_short_circuit_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }

    // 1. Emit.
    let artifact = compile(&demo_module()).expect("module should compile to Go source");

    // 2. Write the source to a unique temp file.  `go run` requires a
    //    `.go` extension.
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_floats_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    // 3. Compile + run with `go run` (arg vector — no shell).
    let run_out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");

    if !run_out.status.success() {
        let stderr = String::from_utf8_lossy(&run_out.stderr);
        let _ = std::fs::remove_file(&src_path);
        panic!(
            "emitted Go failed to compile/run:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    // 4. Assert the program's observable behaviour.
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec!["4.0", "4.0", "5", "7", "#f", "#t"],
        "unexpected program output; full stdout:\n{stdout}"
    );

    // 5. Best-effort cleanup (ignore errors — temp dir is ephemeral).
    let _ = std::fs::remove_file(&src_path);
}
