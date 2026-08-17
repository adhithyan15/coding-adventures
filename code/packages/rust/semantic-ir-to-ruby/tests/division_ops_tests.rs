//! SIR21 T3b-2 execution proof: `div_floor`/`div_trunc`/`udiv_trunc`/
//! `div_true` on the Ruby backend — hand-builds a module calling each op
//! directly (bypassing the frontend, since no frontend emits these names
//! yet), emits Ruby, runs it with a real `ruby` interpreter, and asserts
//! stdout/exit status. Mirrors `sys_write_tests.rs`'s pattern; skips
//! (does not fail) when no `ruby` is on `PATH`.
//!
//! `div_floor` is a bare alias for the pre-existing `/` (already exercised
//! everywhere `/` is used) — verified here by checking it computes the SAME
//! values that op already produces. `div_trunc`/`udiv_trunc`/`div_true` are
//! the ones with real new logic (`div_trunc`/`udiv_trunc` share the
//! pre-existing `sir_tdiv` helper `tdiv`/`utdiv` already use; `div_true` is
//! entirely new), and get the most coverage here, including the
//! zero-divisor case — where `div_floor`/`div_trunc`/`udiv_trunc` raise via
//! Ruby's own native `Integer#/0`, but `div_true` needs its own explicit
//! check (Ruby's native `Float#/0` silently returns `Infinity`).

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    Stmt,
};

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
        metadata: Metadata::new().with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

/// Run emitted Ruby, returning `(stdout, stderr, success)`, or `None` to
/// skip when no `ruby` is on `PATH`. Unlike `sys_write_tests.rs`'s
/// `run_ruby` (which panics on non-zero exit), this surfaces the exit
/// status too, since the zero-divisor tests below expect a NON-zero exit.
fn run_division_program(m: &Module) -> Option<(String, String, bool)> {
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);

    let source = semantic_ir_to_ruby::compile(m).expect("ruby emit").source;
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "sir_ruby_div_{}_{}.rb",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::File::create(&path).ok()?.write_all(source.as_bytes()).ok()?;
    let out = std::process::Command::new("ruby").arg(&path).output().ok()?;
    let _ = std::fs::remove_file(&path);
    Some((
        String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"),
        String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n"),
        out.status.success(),
    ))
}

fn run_division_ok(m: &Module) -> Option<String> {
    let (out, err, ok) = run_division_program(m)?;
    assert!(ok, "emitted ruby exited non-zero:\n{err}");
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
    match run_division_ok(&m) {
        Some(out) => assert_eq!(out, "3\n-3\n3.5\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

// ── div_floor: a bare alias for `/`'s existing floor logic ───────────────

#[test]
fn div_floor_floors_toward_negative_infinity() {
    let m = div_module(vec![
        print_stmt(bin("div_floor", ilit(7), ilit(2))),
        print_stmt(bin("div_floor", ilit(-7), ilit(2))),
        print_stmt(bin("div_floor", ilit(7), ilit(-2))),
        print_stmt(bin("div_floor", ilit(-7), ilit(-2))),
    ]);
    match run_division_ok(&m) {
        Some(out) => assert_eq!(out, "3\n-4\n-4\n3\n"),
        None => eprintln!("skip: no ruby on PATH"),
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
    match run_division_ok(&m) {
        Some(out) => assert_eq!(out, "3\n-3\n-3\n3\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn udiv_trunc_matches_div_trunc_on_positive_operands() {
    let m = div_module(vec![print_stmt(bin("udiv_trunc", ilit(7), ilit(2)))]);
    match run_division_ok(&m) {
        Some(out) => assert_eq!(out, "3\n"),
        None => eprintln!("skip: no ruby on PATH"),
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
    match run_division_ok(&m) {
        Some(out) => assert_eq!(out, "3.5\n-3.5\n2.0\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

/// `div_floor`/`div_trunc`/`udiv_trunc` raise via Ruby's own NATIVE
/// `Integer#/0` (no explicit check needed in the emitted code — confirmed
/// by `sir_tdiv`'s own doc comment: "Division by zero raises (as in C it
/// is undefined)"). `div_true` needs its own explicit check in
/// `sir_true_div` — Ruby's native `Float#/0` silently returns `Infinity`
/// rather than raising, which this op must NOT let through.
#[test]
fn zero_divisor_raises_zero_division_error_for_every_op() {
    for op in ["div_floor", "div_trunc", "udiv_trunc", "div_true"] {
        let m = div_module(vec![print_stmt(bin(op, ilit(7), ilit(0)))]);
        let Some((out, err, ok)) = run_division_program(&m) else {
            eprintln!("skip: no ruby on PATH");
            break;
        };
        assert!(!ok, "[{op}] expected a non-zero exit; stdout={out:?}");
        assert!(
            err.contains("ZeroDivisionError") && err.contains("divided by 0"),
            "[{op}] expected 'ZeroDivisionError'/'divided by 0' on stderr, got:\n{err}"
        );
    }
}
