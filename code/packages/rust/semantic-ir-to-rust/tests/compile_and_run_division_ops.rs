//! SIR21 T3b-2 execution proof: `div_floor`/`div_trunc`/`udiv_trunc`/
//! `div_true` on the Rust backend — hand-builds a module calling each op
//! directly (bypassing the frontend, since no frontend emits these names
//! yet), emits Rust, compiles with `rustc`, runs it, checks stdout/exit
//! status. Mirrors `compile_and_run_floats.rs`/
//! `compile_and_run_typed_runtime_errors.rs`'s identical pattern; gates on
//! `rustc` being usable (and a working linker) and skips (does not fail)
//! if either is absent.
//!
//! `div_floor` is a rename of this backend's existing `divide` (already
//! exercised end-to-end by `compile_and_run_floats.rs`'s `1 = 1.0` case and
//! `compile_and_run_typed_runtime_errors.rs`'s zero-divisor cases) —
//! verified here by checking it computes the SAME values that already-
//! tested helper does. `div_trunc`/`udiv_trunc`/`div_true` are genuinely
//! new, and get the most coverage here, including the zero-divisor raise
//! path for every op (an uncaught SIR `raise` exits non-zero and prints
//! `"ZeroDivisionError: divided by 0"` on stderr, per `report_uncaught`).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    Stmt,
};
use semantic_ir_to_rust::compile;

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
            // Needed for the top-level `catch_unwind`/`report_uncaught`
            // wrapper `emit_main` only emits when `Feature::Exceptions` is
            // declared — without it an uncaught `raise` (the zero-divisor
            // test below) surfaces as a raw, unhelpful Rust panic instead
            // of the clean "ZeroDivisionError: divided by 0" message.
            Feature::Exceptions,
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

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Compile emitted Rust and run it. Returns `None` if `rustc` is
/// unavailable, or the host has no usable linker — both are skips, never
/// failures.
fn run_raw(module: &Module, tag: &str) -> Option<std::process::Output> {
    if !rustc_available() {
        return None;
    }
    let artifact = compile(module).expect("module should compile to Rust source");
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_rs_div_{tag}_{nonce}.rs"));
    let bin_path =
        dir.join(format!("sir_rs_div_{tag}_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
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
            return None;
        }
        panic!(
            "emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    let out = Command::new(&bin_path).output().expect("run compiled binary");
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
    Some(out)
}

fn run(module: &Module, tag: &str) -> Option<String> {
    let out = run_raw(module, tag)?;
    assert!(
        out.status.success(),
        "emitted Rust failed at runtime:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

fn run_expect_zero_div_failure(module: &Module, tag: &str) -> Option<()> {
    let out = run_raw(module, tag)?;
    assert!(
        !out.status.success(),
        "expected a zero-divisor failure (nonzero exit), got success with stdout:\n{}",
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
        None => eprintln!("skip: no rustc/linker"),
    }
}

// ── div_floor: a rename of `divide`'s int path ────────────────────────

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
        None => eprintln!("skip: no rustc/linker"),
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
        None => eprintln!("skip: no rustc/linker"),
    }
}

/// `i64::MIN / -1` is not representable (the true quotient, `2^63`,
/// overflows `i64`) — plain Rust `/` PANICS uncatchably on exactly this
/// input pair. `trunc_div` must use `wrapping_div` instead, so this must
/// NOT crash the process; it wraps to `i64::MIN` (two's-complement
/// wraparound), matching this runtime's existing saturate-on-overflow
/// convention for fixed-width `i64` rather than an uncatchable host panic.
#[test]
fn div_trunc_wraps_instead_of_panicking_on_i64_min_div_neg_one() {
    match run(
        &div_module(vec![print_stmt(bin("div_trunc", ilit(i64::MIN), ilit(-1)))]),
        "trunc_overflow",
    ) {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec![i64::MIN.to_string()]),
        None => eprintln!("skip: no rustc/linker"),
    }
}

#[test]
fn udiv_trunc_matches_div_trunc_on_positive_operands() {
    match run(
        &div_module(vec![print_stmt(bin("udiv_trunc", ilit(7), ilit(2)))]),
        "udiv",
    ) {
        Some(out) => assert_eq!(out.lines().collect::<Vec<_>>(), vec!["3"]),
        None => eprintln!("skip: no rustc/linker"),
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
        None => eprintln!("skip: no rustc/linker"),
    }
}

#[test]
fn zero_divisor_fails_with_zero_division_error_for_every_op() {
    for op in ["div_floor", "div_trunc", "udiv_trunc", "div_true"] {
        match run_expect_zero_div_failure(&div_module(vec![print_stmt(bin(op, ilit(7), ilit(0)))]), op)
        {
            Some(()) => {}
            None => {
                eprintln!("skip: no rustc/linker");
                break;
            }
        }
    }
}
