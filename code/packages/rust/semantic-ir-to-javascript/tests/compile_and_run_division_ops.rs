//! SIR21 T3b-2 execution proof: `div_floor`/`div_trunc`/`udiv_trunc`/
//! `div_true` on the JavaScript backend — hand-builds a module calling
//! each op directly (bypassing the frontend, since no frontend emits
//! these names yet), runs it with `node`, and asserts stdout/exit status.
//! Mirrors `compile_and_run_sys_write.rs`'s pattern; skips gracefully when
//! no `node` is on `PATH`.
//!
//! `div_floor` is a bare alias for this backend's existing `divide`
//! (already zero-check-correct on both int and float paths) — verified
//! here by checking it computes the SAME values that helper already
//! produces. `div_trunc`/`udiv_trunc`/`div_true` are genuinely new, and
//! get the most coverage here, including the zero-divisor case for every
//! op (an uncaught `SirError` — a native `Error` subclass — prints its
//! `name: message` via Node's default uncaught-exception handler and
//! exits non-zero).

use std::path::PathBuf;
use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    Stmt, CURRENT_SIR_VERSION,
};
use semantic_ir_to_javascript::compile;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

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
                Expr::StrLit { value: "once".into(), span: s() },
                Expr::BoolLit { value: false, span: s() },
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
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s(),
    }
}

/// Run a `div_module` under `node`, returning `(stdout, stderr, success)`,
/// or `None` to skip when no usable `node` is present. Unlike
/// `compile_and_run_sys_write.rs`'s `run` (which asserts success
/// unconditionally), this surfaces the exit status too, since the
/// zero-divisor tests below expect a NON-zero exit.
fn run_raw(module: &Module, tag: &str) -> Option<(String, String, bool)> {
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let artifact = compile(module).expect("compile to javascript");
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_js_div_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &artifact.source).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    Some((
        String::from_utf8(output.stdout).expect("utf8 stdout").replace("\r\n", "\n"),
        String::from_utf8(output.stderr).expect("utf8 stderr").replace("\r\n", "\n"),
        output.status.success(),
    ))
}

fn run(module: &Module, tag: &str) -> Option<String> {
    let (out, err, ok) = run_raw(module, tag)?;
    assert!(ok, "node exited non-zero for `{tag}`:\nstderr: {err}");
    Some(out)
}

// ── §E3's own worked example, verbatim ──────────────────────────────────

#[test]
fn e3_worked_example() {
    let m = div_module(vec![
        print_stmt(bin("div_floor", ilit(7), ilit(2))),
        print_stmt(bin("div_trunc", ilit(-7), ilit(2))),
        print_stmt(bin("div_true", ilit(7), ilit(2))),
    ]);
    if let Some(out) = run(&m, "e3") {
        assert_eq!(out, "3\n-3\n3.5\n");
    }
}

// ── div_floor: a bare alias for `divide`'s existing floor logic ─────────

#[test]
fn div_floor_floors_toward_negative_infinity() {
    let m = div_module(vec![
        print_stmt(bin("div_floor", ilit(7), ilit(2))),
        print_stmt(bin("div_floor", ilit(-7), ilit(2))),
        print_stmt(bin("div_floor", ilit(7), ilit(-2))),
        print_stmt(bin("div_floor", ilit(-7), ilit(-2))),
    ]);
    if let Some(out) = run(&m, "floor") {
        assert_eq!(out, "3\n-4\n-4\n3\n");
    }
}

// ── div_trunc/udiv_trunc: truncate toward zero ───────────────────────────

#[test]
fn div_trunc_truncates_toward_zero() {
    let m = div_module(vec![
        print_stmt(bin("div_trunc", ilit(7), ilit(2))),
        print_stmt(bin("div_trunc", ilit(-7), ilit(2))),
        print_stmt(bin("div_trunc", ilit(7), ilit(-2))),
        print_stmt(bin("div_trunc", ilit(-7), ilit(-2))),
    ]);
    if let Some(out) = run(&m, "trunc") {
        assert_eq!(out, "3\n-3\n-3\n3\n");
    }
}

#[test]
fn udiv_trunc_matches_div_trunc_on_positive_operands() {
    let m = div_module(vec![print_stmt(bin("udiv_trunc", ilit(7), ilit(2)))]);
    if let Some(out) = run(&m, "udiv") {
        assert_eq!(out, "3\n");
    }
}

// ── div_true: genuinely new — always true-divides ────────────────────────

#[test]
fn div_true_always_true_divides_even_on_integer_operands() {
    let m = div_module(vec![
        print_stmt(bin("div_true", ilit(7), ilit(2))),
        print_stmt(bin("div_true", ilit(-7), ilit(2))),
        print_stmt(bin("div_true", ilit(6), ilit(3))),
    ]);
    if let Some(out) = run(&m, "true") {
        assert_eq!(out, "3.5\n-3.5\n2.0\n");
    }
}

#[test]
fn zero_divisor_raises_zero_division_error_for_every_op() {
    for op in ["div_floor", "div_trunc", "udiv_trunc", "div_true"] {
        let m = div_module(vec![print_stmt(bin(op, ilit(7), ilit(0)))]);
        let Some((out, err, ok)) = run_raw(&m, op) else {
            eprintln!("skip: no node on PATH");
            break;
        };
        assert!(!ok, "[{op}] expected a non-zero exit; stdout={out:?}");
        assert!(
            err.contains("ZeroDivisionError") && err.contains("divided by 0"),
            "[{op}] expected 'ZeroDivisionError'/'divided by 0' on stderr, got:\n{err}"
        );
    }
}
