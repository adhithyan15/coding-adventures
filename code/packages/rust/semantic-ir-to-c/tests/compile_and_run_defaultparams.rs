//! Execution proof for SIR19 default parameters on the C backend — hand-build
//! modules (producer-agnostic), emit C, compile with a real gcc/clang-style
//! compiler, run, assert stdout. Skips gracefully when no `cc` is present.
//!
//! C has no native default parameters, so this uses the `_sir_missing` sentinel:
//! a `DirectCall` that omits a trailing defaulted argument pads the call with
//! `_sir_missing()`, and each function opens with a prologue `if
//! (_sir_is_missing(p)) { p = <default>; }` in declaration order. Hand-built to
//! bypass the frontend and prove the whole path (padding + prologue) end to end.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Param, ParamKind,
    Scope, Span, Stmt, CURRENT_SIR_VERSION,
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
    let stem = format!("sirc_def_{}_{}", std::process::id(), n);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));
    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");
    let out = Command::new(&cc)
        .args(["-std=c99", "-Wall", "-Werror=unused-variable", "-o"])
        .arg(&exe)
        .arg(&cpath)
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
fn pref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}
fn bc(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn puts(arg: Expr) -> Stmt {
    Stmt::ExprStmt { expr: bc("puts", vec![arg]), span: s() }
}
fn directcall(fn_name: &str, args: Vec<Expr>) -> Expr {
    Expr::DirectCall { fn_name: fn_name.into(), args, effects: EffectSet::PURE, span: s() }
}
fn param(name: &str, default: Option<Expr>) -> Param {
    Param {
        name: name.into(),
        sir_type: None,
        kind: ParamKind::Required,
        default: default.map(Box::new),
        span: s(),
    }
}
/// A module with a helper function `f(<params>) = <body_value>` and a `main`
/// running `main_stmts`, declaring `DefaultParams` (+ `DynamicTyping`, which an
/// untyped parameter makes the validator observe).
fn defparam_module(params: Vec<Param>, body_value: Expr, main_stmts: Vec<Stmt>) -> Module {
    let f = Function {
        name: "f".into(),
        params,
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: body_value, span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    };
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts: main_stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    };
    Module {
        name: "defprog".into(),
        manifest: FeatureManifest::from_features(&[Feature::DefaultParams, Feature::DynamicTyping]),
        imports: vec![],
        exports: vec![],
        functions: vec![f, main],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s(),
    }
}

#[test]
fn default_param_is_used_when_the_argument_is_omitted() {
    // `f(a, b = 5) = a + b` — `f(1)` pads the missing `b` and the prologue fills
    // `5` (`1 + 5 = 6`); `f(1, 2)` supplies `b` (`1 + 2 = 3`).
    let body = bc("+", vec![pref("a"), pref("b")]);
    match run(&defparam_module(
        vec![param("a", None), param("b", Some(ilit(5)))],
        body,
        vec![
            puts(directcall("f", vec![ilit(1)])),          // 6
            puts(directcall("f", vec![ilit(1), ilit(2)])), // 3
        ],
    )) {
        Some(out) => assert_eq!(out, "6\n3\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn default_may_reference_an_earlier_parameter() {
    // `f(a, b = a) = b` — a default sees the parameters declared before it (the
    // prologue runs in declaration order), so `f(7)` yields `7`.
    match run(&defparam_module(
        vec![param("a", None), param("b", Some(pref("a")))],
        pref("b"),
        vec![puts(directcall("f", vec![ilit(7)]))], // 7
    )) {
        Some(out) => assert_eq!(out, "7\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn two_defaults_both_fill_when_omitted() {
    // `f(a, b = 10, c = 20) = a + b + c` — `f(1)` fills both (`31`), `f(1, 2)`
    // fills only `c` (`23`), `f(1, 2, 3)` supplies all (`6`). Proves multiple
    // trailing defaults pad and fill correctly.
    let body = bc("+", vec![bc("+", vec![pref("a"), pref("b")]), pref("c")]);
    match run(&defparam_module(
        vec![
            param("a", None),
            param("b", Some(ilit(10))),
            param("c", Some(ilit(20))),
        ],
        body,
        vec![
            puts(directcall("f", vec![ilit(1)])),                    // 31
            puts(directcall("f", vec![ilit(1), ilit(2)])),           // 23
            puts(directcall("f", vec![ilit(1), ilit(2), ilit(3)])),  // 6
        ],
    )) {
        Some(out) => assert_eq!(out, "31\n23\n6\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn prologue_and_padding_emit_the_sentinel() {
    // Emit-shape: the callee opens with a `_sir_is_missing` guard, and a call
    // that omits the defaulted argument pads with `_sir_missing()`.
    let src = semantic_ir_to_c::compile(&defparam_module(
        vec![param("a", None), param("b", Some(ilit(5)))],
        bc("+", vec![pref("a"), pref("b")]),
        vec![puts(directcall("f", vec![ilit(1)]))],
    ))
    .expect("compile")
    .source;
    assert!(src.contains("if (_sir_is_missing(b))"), "prologue guard:\n{src}");
    assert!(src.contains("_sir_missing()"), "call-site padding:\n{src}");
}
