//! End-to-end proof for the `puts` builtin (Ruby semantics) in the Go
//! backend.
//!
//! Ruby's `puts` is the most common output method, and its semantics are
//! subtle enough that a *shape* assertion in a unit test is not enough — we
//! must prove the emitted Go actually produces the exact byte stream Ruby
//! would.  This test hand-builds a SIR module equivalent to the Ruby program
//!
//!     puts "hello"
//!     puts
//!     puts [1, 2, 3]
//!
//! emits Go, runs it with `go run`, and asserts stdout is **exactly**
//! `hello\n\n1\n2\n3\n` — the Ruby reference output (a line for `hello`, a
//! blank line for the no-arg `puts`, then each array element on its own
//! line).
//!
//! Gates on `go` being available; logs a skip rather than failing when the
//! toolchain is absent (mirrors `compile_and_run_loops.rs`).

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

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn var(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
}

/// `puts(args…)` as an effectful statement (MayPrint, matching the frontend).
fn puts_stmt(args: Vec<Expr>) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "puts".into(),
            args,
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// Build a module whose `main` runs `puts "hello"; puts; puts [1,2,3]`.
fn demo_module() -> Module {
    let stmts = vec![
        puts_stmt(vec![slit("hello")]),
        puts_stmt(vec![]),
        puts_stmt(vec![Expr::SeqLit { items: vec![ilit(1), ilit(2), ilit(3)], span: s() }]),
    ];

    Module {
        name: "puts_demo".into(),
        manifest: FeatureManifest::from_features(&[Feature::Sequences, Feature::Strings]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
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

/// Build a module equivalent to the Ruby program
///
///     a = [nil]
///     a[0] = a
///     puts a
///
/// i.e. a *self-referential* array.  Real Ruby is cycle-aware: `puts a`
/// prints `[...]` and terminates.  Before the cycle guard the emitted
/// `_sir_puts_one` recursed per-element with no bound, so this program
/// overflowed the Go stack and aborted (CWE-674, uncontrolled recursion — a
/// DoS).  The guard must make it terminate.
fn cyclic_module() -> Module {
    let stmts = vec![
        // a = [nil]  (a one-slot seq we can then point at itself)
        Stmt::LetBinding {
            name: "a".into(),
            sir_type: None,
            value: Expr::SeqLit { items: vec![Expr::NilLit { span: s() }], span: s() },
            span: s(),
        },
        // a[0] = a  (close the cycle)
        Stmt::SeqSet { seq: var("a"), index: ilit(0), value: var("a"), span: s() },
        // puts a
        puts_stmt(vec![var("a")]),
    ];

    Module {
        name: "puts_cyclic".into(),
        manifest: FeatureManifest::from_features(&[Feature::Sequences, Feature::Strings]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
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
fn puts_compiles_and_matches_ruby_output() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }

    let artifact = compile(&demo_module()).expect("module should compile to Go source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_puts_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

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

    // Exact byte-for-byte match against the Ruby reference output.
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    // Go may emit CRLF line endings on Windows via fmt; normalise before
    // comparing so the assertion tests the *semantics* (one line per unit),
    // not the platform's newline convention.
    let normalised = stdout.replace("\r\n", "\n");
    assert_eq!(
        normalised, "hello\n\n1\n2\n3\n",
        "unexpected puts output; full stdout (escaped): {stdout:?}"
    );

    let _ = std::fs::remove_file(&src_path);
}

/// Regression (security, CWE-674): `puts` on a self-referential array must
/// TERMINATE — printing a `[...]` cycle placeholder like Ruby — rather than
/// recursing until the Go stack overflows and the process aborts.
#[test]
fn puts_cyclic_array_terminates() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }

    let artifact = compile(&cyclic_module()).expect("cyclic module should compile to Go source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_puts_cyclic_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let run_out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");

    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let stderr = String::from_utf8_lossy(&run_out.stderr);
    let _ = std::fs::remove_file(&src_path);

    // The program must exit SUCCESSFULLY (no stack-overflow abort) …
    assert!(
        run_out.status.success(),
        "cyclic puts should terminate cleanly, not overflow; stderr: {stderr}"
    );
    // … and render the cycle as Ruby does: `[...]` on its own line.
    let normalised = stdout.replace("\r\n", "\n");
    assert_eq!(
        normalised, "[...]\n",
        "unexpected cyclic puts output; full stdout (escaped): {stdout:?}"
    );
}
