//! Execution proof for SIR16 short-circuit on the C backend — hand-build
//! modules (producer-agnostic), emit C, compile with a real gcc/clang-style
//! compiler, run, assert stdout. Skips gracefully when no `cc` is present.
//!
//! `Feature::ShortCircuit` gates `Expr::LogicalAnd` / `Expr::LogicalOr`. The C
//! backend already short-circuits the eager `and`/`or` builtins the same way,
//! so the node lowering is a mirror: assign the left operand into the
//! destination, then conditionally overwrite with the right — the right is
//! never evaluated when the left decides, and the result is the DECIDING
//! OPERAND (not a bool). Hand-built to bypass the frontend, which constant-folds
//! a literal `&&`/`||`.

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
    let stem = format!("sirc_sc_{}_{}", std::process::id(), n);
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
fn blit(value: bool) -> Expr {
    Expr::BoolLit { value, span: s() }
}
fn nil_lit() -> Expr {
    Expr::NilLit { span: s() }
}
fn land(lhs: Expr, rhs: Expr) -> Expr {
    Expr::LogicalAnd { lhs: Box::new(lhs), rhs: Box::new(rhs), span: s() }
}
fn lor(lhs: Expr, rhs: Expr) -> Expr {
    Expr::LogicalOr { lhs: Box::new(lhs), rhs: Box::new(rhs), span: s() }
}
fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn puts(arg: Expr) -> Stmt {
    Stmt::ExprStmt { expr: bc("puts", vec![arg]), span: s() }
}
/// A `main` module declaring only `ShortCircuit`.
fn sc_module(stmts: Vec<Stmt>) -> Module {
    Module {
        name: "scprog".into(),
        manifest: FeatureManifest::from_features(&[Feature::ShortCircuit]),
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
fn logical_and_returns_the_deciding_operand() {
    // `a && b` is the OPERAND: `1 && 2` → `2` (lhs truthy → rhs); `false && 2` →
    // `false` (lhs falsy → lhs); `nil && 2` → `nil`.
    match run(&sc_module(vec![
        puts(land(ilit(1), ilit(2))),     // 2
        puts(land(blit(false), ilit(2))), // #f
        puts(land(nil_lit(), ilit(2))),   // nil
    ])) {
        Some(out) => assert_eq!(out, "2\n#f\nnil\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn logical_or_returns_the_deciding_operand() {
    // `a || b`: `1 || 2` → `1` (lhs truthy → lhs); `false || 5` → `5` (lhs
    // falsy → rhs); `nil || 7` → `7`.
    match run(&sc_module(vec![
        puts(lor(ilit(1), ilit(2))),     // 1
        puts(lor(blit(false), ilit(5))), // 5
        puts(lor(nil_lit(), ilit(7))),   // 7
    ])) {
        Some(out) => assert_eq!(out, "1\n5\n7\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn short_circuit_does_not_evaluate_the_dead_operand() {
    // The RHS must NOT run when the LHS decides. The dead operand is `1 / 0`,
    // which TRAPS (`_sir_ifloordiv` → `stderr` + `exit(1)`) if evaluated — so a
    // broken eager lowering exits non-zero and `run` (which asserts success)
    // fails. A correct short-circuit skips it: `false && (1/0)` → `false`,
    // `true || (1/0)` → `true`, both exit 0.
    let div_by_zero = || bc("/", vec![ilit(1), ilit(0)]);
    match run(&sc_module(vec![
        puts(land(blit(false), div_by_zero())), // #f, no trap
        puts(lor(blit(true), div_by_zero())),   // #t, no trap
    ])) {
        Some(out) => assert_eq!(out, "#f\n#t\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn logical_and_in_tail_position() {
    // A short-circuit in a function's RETURN position exercises `emit_tail`
    // (which routes a compound value into a temp, then returns it) rather than
    // `emit_assign` — it must stay total there too. `sc()` returns `1 && 2` (→
    // `2`), and `main` prints the call result.
    let m = Module {
        name: "sctail".into(),
        manifest: FeatureManifest::from_features(&[Feature::ShortCircuit]),
        imports: vec![],
        exports: vec![],
        functions: vec![
            Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body: Block {
                    stmts: vec![puts(Expr::DirectCall {
                        fn_name: "sc".into(),
                        args: vec![],
                        effects: EffectSet::PURE,
                        span: s(),
                    })],
                    value: Expr::NilLit { span: s() },
                    span: s(),
                },
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            },
            Function {
                name: "sc".into(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body: Block {
                    stmts: vec![],
                    value: land(ilit(1), ilit(2)),
                    span: s(),
                },
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            },
        ],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s(),
    };
    match run(&m) {
        Some(out) => assert_eq!(out, "2\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn short_circuit_emits_a_truthiness_branch() {
    // Emit-shape: the node lowers to an `if (_sir_truthy(...))` overwrite, not a
    // bare C `&&` (which would yield an int 0/1, not the operand).
    let src = semantic_ir_to_c::compile(&sc_module(vec![puts(land(ilit(1), ilit(2)))]))
        .expect("compile")
        .source;
    assert!(
        src.contains("if (_sir_truthy("),
        "LogicalAnd lowers to a truthiness branch:\n{src}"
    );
}
