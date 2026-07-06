//! Regression proofs for two Ruby index/`fetch` parity fixes in the Go
//! backend, closing tasks #66 and #67 (found during the typed-runtime-errors
//! cascade):
//!
//!   #66 — `arr[i]` (the `[]` index op) must RETURN NIL out of bounds and let
//!         a negative index count from the end, matching Ruby + the other
//!         backends.  The Go `_sir_seq_index` previously PANICKED on any OOB.
//!
//!   #67 — `arr.fetch("x")` with a non-integer index must raise a catchable
//!         `TypeError` ("no implicit conversion of String into Integer"),
//!         matching Ruby — not the raw `_sir_as_int` "expected int" panic
//!         (which surfaced only as a generic StandardError).
//!
//! Both go the whole way: hand-built SIR → emitted Go → `go run` → observed
//! stdout/stderr.  Gated on `go` being available (skip, never fail, if absent).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    Stmt,
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
fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}
fn index(seq_expr: Expr, idx: Expr) -> Expr {
    Expr::SeqIndex { seq: Box::new(seq_expr), index: Box::new(idx), span: s() }
}
fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
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

fn module_of(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "index_parity".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Strings,
            Feature::Symbols,
            Feature::DynamicTyping,
        ]),
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

fn run_go(source: &str, tag: &str) -> std::process::Output {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("sir_go_{}_{}.go", tag, std::process::id()));
    std::fs::write(&path, source).expect("write temp source");
    let out = Command::new("go").arg("run").arg(&path).output().expect("invoke go run");
    let _ = std::fs::remove_file(&path);
    out
}

/// #66 — `arr[i]` returns nil OOB and counts negatives from the end.
#[test]
fn index_read_out_of_bounds_returns_nil_and_negatives_count_from_end() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let arr = || seq(vec![ilit(10), ilit(20), ilit(30)]);
    let stmts = vec![
        print_stmt(index(arr(), ilit(5))),   // OOB high  → nil
        print_stmt(index(arr(), ilit(-1))),  // last      → 30
        print_stmt(index(arr(), ilit(-3))),  // first     → 10
        print_stmt(index(arr(), ilit(-9))),  // OOB low   → nil
        print_stmt(index(arr(), ilit(1))),   // in range  → 20
    ];
    let artifact = compile(&module_of(stmts)).expect("compiles to Go");
    let out = run_go(&artifact.source, "index_nil");
    assert!(
        out.status.success(),
        "arr[oob] must NOT panic; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lines: Vec<&str> = std::str::from_utf8(&out.stdout).unwrap().lines().collect();
    assert_eq!(
        lines,
        vec!["nil", "30", "10", "nil", "20"],
        "unexpected index output; full stdout:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// #67 — `arr.fetch("x")` (non-integer index) raises a typed `TypeError`,
/// not the raw `_sir_as_int` "expected int" panic.  Asserted the way
/// `unknown_send_fails_cleanly` does: controlled error message, no native panic.
#[test]
fn fetch_non_integer_index_raises_type_error_not_native_panic() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let arr = seq(vec![ilit(1), ilit(2), ilit(3)]);
    let stmts =
        vec![print_stmt(call("__method__", vec![arr, slit("fetch"), slit("x")]))];
    let artifact = compile(&module_of(stmts)).expect("compiles to Go");
    let out = run_go(&artifact.source, "fetch_typeerror");
    assert!(
        !out.status.success(),
        "fetch(non-int) must raise, not succeed; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("TypeError")
            && stderr.contains("no implicit conversion")
            && stderr.contains("Integer"),
        "expected a typed TypeError; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("expected int"),
        "the type guard must replace the raw _sir_as_int panic; stderr:\n{stderr}"
    );
}
