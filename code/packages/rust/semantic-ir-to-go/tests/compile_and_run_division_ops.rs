//! SIR21 T3b-2 execution proof: `div_floor`/`div_trunc`/`udiv_trunc`/
//! `div_true` on the Go backend — hand-builds a module calling each op
//! directly (bypassing the frontend, since no frontend emits these names
//! yet), emits Go, runs it with `go run`, checks stdout/exit status.
//! Mirrors `compile_and_run_floats.rs`'s identical pattern; gates on `go`
//! being on `PATH` and skips (does not fail) if absent.
//!
//! `div_floor`/`div_trunc` are renames of this backend's existing
//! `_sir_divide` (int path) — verified here by checking they compute the
//! SAME values that already-tested helper does. `udiv_trunc`/`div_true`
//! are genuinely new (see `runtime.rs`'s doc comments for each), and get
//! the most coverage here, including the zero-divisor panic path, which
//! Go's uncaught-panic default behaviour turns into a nonzero exit +
//! `SirError.Error()`'s formatted message on stderr.

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
fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn bin(name: &str, a: Expr, b: Expr) -> Expr {
    call(name, vec![a, b])
}
fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "__sys_write__".into(),
            args: vec![
                Expr::StrLit { value: "stdout".into(), span: s() },
                Expr::StrLit { value: "per_value".into(), span: s() },
                Expr::BoolLit { value: true, span: s() },
                expr,
            ],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

fn div_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "divprog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::ConsoleIO,
            Feature::Strings,
            Feature::Floats,
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

/// Compile, write to a unique temp file, `go run` it. Returns `None` if
/// `go` is unavailable (caller should skip, not fail).
fn run_raw(module: &Module, tag: &str) -> Option<std::process::Output> {
    if !go_available() {
        return None;
    }
    let artifact = compile(module).expect("module should compile to Go source");
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_div_{tag}_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");
    let out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");
    let _ = std::fs::remove_file(&src_path);
    Some(out)
}

fn run(module: &Module, tag: &str) -> Option<String> {
    let out = run_raw(module, tag)?;
    assert!(
        out.status.success(),
        "emitted Go failed to compile/run: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

fn run_expect_zero_div_panic(module: &Module, tag: &str) -> Option<()> {
    let out = run_raw(module, tag)?;
    assert!(
        !out.status.success(),
        "expected a zero-divisor panic (nonzero exit), got success with stdout:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("ZeroDivisionError: divided by 0"),
        "expected 'ZeroDivisionError: divided by 0' on stderr, got:\n{stderr}"
    );
    Some(())
}

// ── §E3's own worked example, verbatim ──────────────────────────────────

#[test]
fn e3_worked_example() {
    match run(
        &div_module(vec![
            print_stmt(bin("div_floor", ilit(7), ilit(2))),
            print_stmt(bin("div_trunc", ilit(-7), ilit(2))),
            print_stmt(bin("div_true", ilit(7), ilit(2))),
        ]),
        "e3",
    ) {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["3", "-3", "3.5"]),
        None => eprintln!("skip: no go"),
    }
}

// ── div_floor: a rename of `_sir_divide`'s int path ──────────────────────

#[test]
fn div_floor_floors_toward_negative_infinity() {
    match run(
        &div_module(vec![
            print_stmt(bin("div_floor", ilit(7), ilit(2))),
            print_stmt(bin("div_floor", ilit(-7), ilit(2))),
            print_stmt(bin("div_floor", ilit(7), ilit(-2))),
            print_stmt(bin("div_floor", ilit(-7), ilit(-2))),
        ]),
        "floor",
    ) {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["3", "-4", "-4", "3"]),
        None => eprintln!("skip: no go"),
    }
}

// ── div_trunc/udiv_trunc: truncate toward zero ───────────────────────────

#[test]
fn div_trunc_truncates_toward_zero() {
    match run(
        &div_module(vec![
            print_stmt(bin("div_trunc", ilit(7), ilit(2))),
            print_stmt(bin("div_trunc", ilit(-7), ilit(2))),
            print_stmt(bin("div_trunc", ilit(7), ilit(-2))),
            print_stmt(bin("div_trunc", ilit(-7), ilit(-2))),
        ]),
        "trunc",
    ) {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["3", "-3", "-3", "3"]),
        None => eprintln!("skip: no go"),
    }
}

#[test]
fn udiv_trunc_matches_div_trunc_on_positive_operands() {
    match run(
        &div_module(vec![print_stmt(bin("udiv_trunc", ilit(7), ilit(2)))]),
        "udiv",
    ) {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["3"]),
        None => eprintln!("skip: no go"),
    }
}

// ── div_true: genuinely new — always true-divides ────────────────────────

#[test]
fn div_true_always_true_divides_even_on_integer_operands() {
    match run(
        &div_module(vec![
            print_stmt(bin("div_true", ilit(7), ilit(2))),
            print_stmt(bin("div_true", ilit(-7), ilit(2))),
            print_stmt(bin("div_true", ilit(6), ilit(3))),
        ]),
        "true",
    ) {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["3.5", "-3.5", "2.0"]),
        None => eprintln!("skip: no go"),
    }
}

#[test]
fn zero_divisor_panics_with_zero_division_error_for_every_op() {
    for op in ["div_floor", "div_trunc", "udiv_trunc", "div_true"] {
        match run_expect_zero_div_panic(
            &div_module(vec![print_stmt(bin(op, ilit(7), ilit(0)))]),
            op,
        ) {
            Some(()) => {}
            None => {
                eprintln!("skip: no go");
                break;
            }
        }
    }
}
