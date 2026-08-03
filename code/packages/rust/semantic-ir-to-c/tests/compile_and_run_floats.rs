//! Execution proof for SIR16 floats on the C backend — hand-build modules
//! (producer-agnostic), emit C, compile with a real gcc/clang-style compiler,
//! run, assert stdout. Skips gracefully when no `cc` is present.
//!
//! `Feature::Floats` gates ONLY `Expr::FloatLit`. The `SIR_FLOAT` tag and its
//! runtime (constructor, arithmetic int→float promotion, IEEE division,
//! `_sir_fmt_float`) have existed since v0, so this batch is purely the emitter
//! arm — but the tests still exercise the whole float path end-to-end through a
//! real `cc`: literal display (integral floats keep `.0`), native arithmetic,
//! the division frontier (Float promotes, two Integers floor), non-finite
//! results and non-finite LITERALS (the `<math.h>` `INFINITY`/`NAN` macros).
//! Hand-built to bypass the frontend, which masks `FloatLit`.

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

fn run(module: &Module) -> Option<String> {
    let cc = find_cc()?;
    let artifact = semantic_ir_to_c::compile(module).expect("C backend compile (no panic)");
    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_flt_{}_{}", std::process::id(), n);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-Werror=unused-variable", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
        .output()
        .expect("spawn cc");
    assert!(
        out.status.success(),
        "compile failed:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        artifact.source
    );
    let r = Command::new(&exe).output().expect("run");
    assert!(r.status.success(), "run failed (exit {:?})", r.status.code());
    Some(String::from_utf8_lossy(&r.stdout).replace("\r\n", "\n"))
}

fn s() -> Span {
    Span::synthetic()
}
fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}
fn flit(value: f64) -> Expr {
    Expr::FloatLit { value, span: s() }
}
fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn bin(name: &str, a: Expr, b: Expr) -> Expr {
    bc(name, vec![a, b])
}
fn puts(arg: Expr) -> Stmt {
    Stmt::ExprStmt { expr: bc("puts", vec![arg]), span: s() }
}
/// A `main` module declaring only `Floats` (arithmetic / `puts` are builtins,
/// gated by the builtin allowlist rather than by a feature).
fn float_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "fltprog".into(),
        manifest: FeatureManifest::from_features(&[Feature::Floats]),
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

#[test]
fn float_literal_emits_a_float_constructor() {
    // Emit-shape: an integral `FloatLit` must build a `_sir_float(7.0)` — a
    // `double` literal with a point, never the Integer path `_sir_int(7LL)`.
    let src = semantic_ir_to_c::compile(&float_module(vec![puts(flit(7.0))]))
        .expect("compile")
        .source;
    assert!(
        src.contains("_sir_float(7.0)"),
        "integral FloatLit builds a double constructor:\n{src}"
    );
    // The `<math.h>` include is present for the non-finite macros.
    assert!(src.contains("#include <math.h>"), "math.h is included:\n{src}");
}

#[test]
fn non_finite_literal_emits_the_math_macros() {
    // A non-finite `FloatLit` has no C floating token — it uses `INFINITY`/`NAN`.
    let src = semantic_ir_to_c::compile(&float_module(vec![
        puts(flit(f64::INFINITY)),
        puts(flit(f64::NEG_INFINITY)),
        puts(flit(f64::NAN)),
    ]))
    .expect("compile")
    .source;
    assert!(src.contains("_sir_float(INFINITY)"), "positive infinity:\n{src}");
    assert!(src.contains("_sir_float(-INFINITY)"), "negative infinity:\n{src}");
    assert!(src.contains("_sir_float(NAN)"), "NaN:\n{src}");
}

#[test]
fn float_literal_displays_with_a_trailing_point() {
    // `puts 7.0` → `7.0` (integral float keeps its `.0`), `3.25` → `3.25`,
    // `-0.0` → `-0.0` (the sign of zero survives), all via `_sir_fmt_float`.
    match run(&float_module(vec![puts(flit(7.0)), puts(flit(3.25)), puts(flit(-0.0))])) {
        Some(out) => assert_eq!(out, "7.0\n3.25\n-0.0\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn float_arithmetic_promotes_and_stays_float() {
    // `+`/`-`/`*` on floats stay Float even when the result is integral (`4.0`,
    // not `4`) — the runtime's int→float promotion, matching every backend.
    match run(&float_module(vec![
        puts(bin("+", flit(1.5), flit(2.5))), // 4.0
        puts(bin("*", flit(2.0), flit(3.0))), // 6.0
        puts(bin("-", flit(7.0), flit(0.5))), // 6.5
    ])) {
        Some(out) => assert_eq!(out, "4.0\n6.0\n6.5\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn float_and_integer_division_follow_the_frontier() {
    // A Float operand promotes to true division (`7.0 / 2 == 3.5`); two Integers
    // floor (`7 / 2 == 3`). A regression guard that adding floats did not
    // disturb the integer division frontier.
    match run(&float_module(vec![
        puts(bin("/", flit(7.0), ilit(2))),   // 3.5
        puts(bin("/", flit(6.0), flit(2.0))), // 3.0
        puts(bin("/", ilit(7), ilit(2))),     // 3 (Integer floor)
    ])) {
        Some(out) => assert_eq!(out, "3.5\n3.0\n3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn non_finite_arithmetic_and_literals_display_named() {
    // Float division by zero yields IEEE `Infinity`/`-Infinity` (NOT a trap —
    // that is Integer-only), `0.0/0.0` is `NaN`, and a non-finite LITERAL
    // round-trips through the `INFINITY`/`NAN` macros — all rendered by
    // `_sir_fmt_float`.
    match run(&float_module(vec![
        puts(bin("/", flit(1.0), flit(0.0))),  // Infinity
        puts(bin("/", flit(-1.0), flit(0.0))), // -Infinity
        puts(bin("/", flit(0.0), flit(0.0))),  // NaN
        puts(flit(f64::INFINITY)),             // Infinity (literal)
        puts(flit(f64::NAN)),                  // NaN (literal)
    ])) {
        Some(out) => assert_eq!(out, "Infinity\n-Infinity\nNaN\nInfinity\nNaN\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn float_equality_is_value_based() {
    // `7.0 == 7.0` is true; a Float equals a numerically-equal Integer
    // (`7.0 == 7`, both numeric); `7.0 == 7.5` is false. Displayed in the
    // default (Lisp) convention as `#t`/`#f`.
    match run(&float_module(vec![
        puts(bin("=", flit(7.0), flit(7.0))), // #t
        puts(bin("=", flit(7.0), ilit(7))),   // #t (numeric cross-type)
        puts(bin("=", flit(7.0), flit(7.5))), // #f
    ])) {
        Some(out) => assert_eq!(out, "#t\n#t\n#f\n"),
        None => eprintln!("skip: no cc"),
    }
}
