//! SIR21 T3b-2 execution proof: `div_floor`/`div_trunc`/`udiv_trunc`/
//! `div_true` on the C backend — hand-build modules calling each op
//! directly (bypassing the frontend, since no frontend emits these names
//! yet), emit C, compile with a real gcc/clang-style compiler, run, assert
//! stdout. Skips gracefully when no `cc` is present, mirroring
//! `compile_and_run_floats.rs`'s identical pattern.
//!
//! `div_floor`/`div_trunc`/`udiv_trunc` are renames of this backend's
//! existing `_sir_divide`/`_sir_itdiv`/`_sir_utdiv` (verified here by
//! checking they compute the SAME values those already-tested helpers do
//! — zero new logic, so these are regression tests, not new-behavior
//! tests). `div_true` is genuinely new (see `runtime.rs`'s `_sir_true_div`
//! doc comment) and gets the most coverage here, including its
//! zero-divisor behavior, which has no precedent to regress against.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span, Stmt,
    CURRENT_SIR_VERSION,
};

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    for cand in ["cc", "clang", "gcc"] {
        if Command::new(cand)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(cand.to_string());
        }
    }
    None
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn compile_and_link(module: &Module) -> Option<(PathBuf, String)> {
    let cc = find_cc()?;
    let artifact = semantic_ir_to_c::compile(module).expect("C backend compile (no panic)");
    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_div_{}_{}", std::process::id(), n);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-Werror=unused-variable", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        artifact.source
    );
    Some((exe, artifact.source))
}

/// Run and require success — for the ordinary (non-zero-divisor) cases.
fn run(module: &Module) -> Option<String> {
    let (exe, _src) = compile_and_link(module)?;
    let r = Command::new(&exe).output().expect("run");
    assert!(r.status.success(), "run failed (exit {:?})", r.status.code());
    Some(String::from_utf8_lossy(&r.stdout).replace("\r\n", "\n"))
}

/// Run and require FAILURE with the "divided by 0" message — for the
/// zero-divisor cases, which are expected to `exit(1)`, not print a result.
fn run_expect_zero_div_failure(module: &Module) -> Option<()> {
    let (exe, _src) = compile_and_link(module)?;
    let r = Command::new(&exe).output().expect("run");
    assert!(
        !r.status.success(),
        "expected a zero-divisor failure (nonzero exit), got success with stdout:\n{}",
        String::from_utf8_lossy(&r.stdout)
    );
    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        stderr.contains("divided by 0"),
        "expected 'divided by 0' on stderr, got:\n{stderr}"
    );
    Some(())
}

fn s() -> Span {
    Span::synthetic()
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}
fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn bin(name: &str, a: Expr, b: Expr) -> Expr {
    bc(name, vec![a, b])
}
fn puts(arg: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: bc(
            "__sys_write__",
            vec![
                Expr::StrLit { value: "stdout".into(), span: s() },
                Expr::StrLit { value: "per_value".into(), span: s() },
                Expr::BoolLit { value: true, span: s() },
                arg,
            ],
        ),
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
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s(),
    }
}

// ── §E3's own worked example, verbatim ──────────────────────────────────

#[test]
fn e3_worked_example() {
    // puts(div_floor(7, 2))    // 3   — Ruby's `/`
    // puts(div_trunc(-7, 2))   // -3  — C's `/`
    // puts(div_true(7, 2))     // 3.5 — Python's `/`
    match run(&div_module(vec![
        puts(bin("div_floor", ilit(7), ilit(2))),
        puts(bin("div_trunc", ilit(-7), ilit(2))),
        puts(bin("div_true", ilit(7), ilit(2))),
    ])) {
        Some(out) => assert_eq!(out, "3\n-3\n3.5\n"),
        None => eprintln!("skip: no cc"),
    }
}

// ── div_floor: a rename of `_sir_divide` — same floor/true-divide split ──

#[test]
fn div_floor_floors_toward_negative_infinity_on_integers() {
    match run(&div_module(vec![
        puts(bin("div_floor", ilit(7), ilit(2))),   // 3
        puts(bin("div_floor", ilit(-7), ilit(2))),  // -4 (floors, not truncates)
        puts(bin("div_floor", ilit(7), ilit(-2))),  // -4
        puts(bin("div_floor", ilit(-7), ilit(-2))), // 3
    ])) {
        Some(out) => assert_eq!(out, "3\n-4\n-4\n3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn div_floor_emits_the_same_helper_as_bare_slash() {
    // Zero behaviour change: both names must emit the identical call.
    let src = semantic_ir_to_c::compile(&div_module(vec![
        puts(bin("/", ilit(7), ilit(2))),
        puts(bin("div_floor", ilit(7), ilit(2))),
    ]))
    .expect("compile")
    .source;
    let slash_call = "_sir_divide(2, _sir_int(7LL), _sir_int(2LL))";
    assert_eq!(
        src.matches(slash_call).count(),
        2,
        "both `/` and `div_floor` must emit the identical `_sir_divide` call:\n{src}"
    );
}

// ── div_trunc/udiv_trunc: renames of tdiv/utdiv ──────────────────────────

#[test]
fn div_trunc_truncates_toward_zero() {
    match run(&div_module(vec![
        puts(bin("div_trunc", ilit(7), ilit(2))),   // 3
        puts(bin("div_trunc", ilit(-7), ilit(2))),  // -3 (truncates, not floors)
        puts(bin("div_trunc", ilit(7), ilit(-2))),  // -3
        puts(bin("div_trunc", ilit(-7), ilit(-2))), // 3
    ])) {
        Some(out) => assert_eq!(out, "3\n-3\n-3\n3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn udiv_trunc_matches_div_trunc_on_positive_operands() {
    // No unsigned literal constructor in this hand-built harness, so this
    // exercises udiv_trunc's ordinary (non-high-bit) path -- the emitted
    // helper is identical to div_trunc's signed twin except for the C-level
    // uint64_t cast, which only matters once the top bit is set (covered by
    // `_sir_utdiv`'s own existing unit tests in runtime.rs, unchanged here).
    match run(&div_module(vec![puts(bin("udiv_trunc", ilit(7), ilit(2)))])) {
        Some(out) => assert_eq!(out, "3\n"),
        None => eprintln!("skip: no cc"),
    }
}

// ── div_true: genuinely new — always coerces to float, never floors ─────

#[test]
fn div_true_always_true_divides_even_on_integer_operands() {
    match run(&div_module(vec![
        puts(bin("div_true", ilit(7), ilit(2))),   // 3.5, not 3
        puts(bin("div_true", ilit(-7), ilit(2))),  // -3.5, not -4
        puts(bin("div_true", ilit(6), ilit(3))),   // 2.0 (exact, still a float)
    ])) {
        Some(out) => assert_eq!(out, "3.5\n-3.5\n2.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn div_floor_and_div_trunc_zero_divisor_fails_loudly() {
    match run_expect_zero_div_failure(&div_module(vec![puts(bin("div_floor", ilit(7), ilit(0)))])) {
        Some(()) => {}
        None => eprintln!("skip: no cc"),
    }
    match run_expect_zero_div_failure(&div_module(vec![puts(bin("div_trunc", ilit(7), ilit(0)))])) {
        Some(()) => {}
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn div_true_zero_divisor_fails_loudly_not_silent_ieee_infinity() {
    // Unlike raw IEEE double division (which would silently yield `inf`),
    // div_true models Python's `/`, which raises ZeroDivisionError
    // unconditionally -- see `_sir_true_div`'s own doc comment in
    // runtime.rs for why this deliberately does NOT match the OLDER
    // `_sir_divide_v`'s silent-infinity float path.
    match run_expect_zero_div_failure(&div_module(vec![puts(bin("div_true", ilit(7), ilit(0)))])) {
        Some(()) => {}
        None => eprintln!("skip: no cc"),
    }
}
