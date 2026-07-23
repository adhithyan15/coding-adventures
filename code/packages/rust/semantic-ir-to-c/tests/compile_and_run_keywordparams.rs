//! Execution proof for SIR19 keyword parameters on the C backend — hand-build
//! modules (producer-agnostic), emit C, compile with a real gcc/clang-style
//! compiler, run, assert stdout. Skips gracefully when no `cc` is present.
//!
//! C has no native keyword calls, so — like the Go backend's KW6 — a
//! `KeywordArg` is resolved to its callee's parameter SLOT BY NAME at emit time,
//! producing a plain positional C call (omitted optional keywords filled with
//! `_sir_missing()` and substituted by the default prologue). Hand-built to
//! bypass the frontend and prove the resolution end to end.

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
    let stem = format!("sirc_kw_{}_{}", std::process::id(), n);
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
fn param(name: &str, kind: ParamKind, default: Option<Expr>) -> Param {
    Param {
        name: name.into(),
        sir_type: None,
        kind,
        default: default.map(Box::new),
        span: s(),
    }
}
fn kwparam(name: &str, default: Option<Expr>) -> Param {
    param(name, ParamKind::Keyword, default)
}
fn kwarg(name: &str, value: Expr) -> Expr {
    Expr::KeywordArg { name: name.into(), value: Box::new(value), span: s() }
}
/// A module with a helper function `f(<params>) = <body_value>` and a `main`
/// running `main_stmts`, declaring `KeywordParams` (+ `DynamicTyping`).
fn kw_module(params: Vec<Param>, body_value: Expr, main_stmts: Vec<Stmt>) -> Module {
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
        name: "kwprog".into(),
        manifest: FeatureManifest::from_features(&[Feature::KeywordParams, Feature::DynamicTyping]),
        imports: vec![],
        exports: vec![],
        functions: vec![f, main],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s(),
    }
}

#[test]
fn keyword_argument_binds_to_the_keyword_parameter() {
    // `def f(x:); x; end` called `f(x: 5)` → `5`.
    match run(&kw_module(
        vec![kwparam("x", None)],
        pref("x"),
        vec![puts(directcall("f", vec![kwarg("x", ilit(5))]))],
    )) {
        Some(out) => assert_eq!(out, "5\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn keyword_arguments_resolve_by_name_regardless_of_order() {
    // `f(a:, b:) = a - b` called `f(b: 2, a: 10)` → `8` — resolved BY NAME, so
    // the reversed call order still binds `a`/`b` to their slots.
    match run(&kw_module(
        vec![kwparam("a", None), kwparam("b", None)],
        bc("-", vec![pref("a"), pref("b")]),
        vec![puts(directcall("f", vec![kwarg("b", ilit(2)), kwarg("a", ilit(10))]))],
    )) {
        Some(out) => assert_eq!(out, "8\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn optional_keyword_uses_its_default_when_omitted() {
    // `f(x: 7) = x` — `f()` fills the default `7` (via `_sir_missing()` + the
    // prologue), `f(x: 9)` overrides it.
    match run(&kw_module(
        vec![kwparam("x", Some(ilit(7)))],
        pref("x"),
        vec![
            puts(directcall("f", vec![])),                    // 7
            puts(directcall("f", vec![kwarg("x", ilit(9))])), // 9
        ],
    )) {
        Some(out) => assert_eq!(out, "7\n9\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn positional_and_keyword_arguments_mix() {
    // `f(a, b:) = a - b` called `f(10, b: 2)` → `8` — the leading positional
    // fills slot 0, the keyword fills slot 1 by name.
    match run(&kw_module(
        vec![param("a", ParamKind::Required, None), kwparam("b", None)],
        bc("-", vec![pref("a"), pref("b")]),
        vec![puts(directcall("f", vec![ilit(10), kwarg("b", ilit(2))]))],
    )) {
        Some(out) => assert_eq!(out, "8\n"),
        None => eprintln!("skip: no cc"),
    }
}

#[test]
fn compound_keyword_value_is_evaluated_once() {
    // A keyword argument whose value is a compound expression (a nested call)
    // hoists into a temp — resolved by name into the right slot. `f(a:, b:) =
    // a - b` called `f(b: g(), a: 10)` where `g() = 2` → `8`.
    let g = Function {
        name: "g".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: ilit(2), span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    };
    let mut m = kw_module(
        vec![kwparam("a", None), kwparam("b", None)],
        bc("-", vec![pref("a"), pref("b")]),
        vec![puts(directcall(
            "f",
            vec![kwarg("b", directcall("g", vec![])), kwarg("a", ilit(10))],
        ))],
    );
    m.functions.push(g);
    match run(&m) {
        Some(out) => assert_eq!(out, "8\n"),
        None => eprintln!("skip: no cc"),
    }
}
