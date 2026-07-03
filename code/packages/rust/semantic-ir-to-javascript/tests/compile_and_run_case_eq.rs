//! End-to-end proof for the `case_eq` builtin (Ruby case-equality, `===`) in
//! the JavaScript backend.
//!
//! Ruby's `case`/`when` lowers (in the frontend) to a chain of `if`s whose
//! conditions are `BuiltinCall("case_eq", [pattern, scrutinee])`.  Before this
//! fix the JS runtime's builtin table had no `case_eq`, so **every**
//! `case`/`when` program threw `TypeError: unknown builtin: case_eq` at runtime
//! — `case` was unusable on this backend.
//!
//! This test hand-builds the SIR equivalent of a `when`-style dispatch (a
//! `case_eq` used as an `if` condition — exactly how the frontend emits it):
//!
//!     if 5 === 5     then puts "A" else puts "Z"   # A
//!     if 5 === 6     then puts "B" else puts "Y"   # Y
//!     if "a" === "a" then puts "C" else puts "X"   # C
//!
//! emits self-contained JavaScript, runs it with `node`, and asserts stdout is
//! exactly `A\nY\nC`.  Driving an `if` off `case_eq` tests the boolean it
//! returns directly.  (Only primitive patterns are used — this backend's
//! `case_eq`, like its `=` builtin, is native `===`.)  Node is optional: skip.

use std::path::PathBuf;
use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    Stmt,
};
use semantic_ir_to_javascript::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn case_eq(pattern: Expr, value: Expr) -> Expr {
    Expr::BuiltinCall {
        name: "case_eq".into(),
        args: vec![pattern, value],
        effects: EffectSet::PURE,
        span: s(),
    }
}

fn puts_stmt(arg: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "puts".into(),
            args: vec![arg],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// A one-statement `Block` that `puts` a literal string.
fn puts_block(msg: &str) -> Block {
    Block {
        stmts: vec![puts_stmt(slit(msg))],
        value: Expr::NilLit { span: s() },
        span: s(),
    }
}

/// `if case_eq(pattern, value) then puts(yes) else puts(no)` — a `when` arm.
fn when_stmt(pattern: Expr, value: Expr, yes: &str, no: &str) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::If {
            cond: Box::new(case_eq(pattern, value)),
            then_branch: Box::new(puts_block(yes)),
            else_branch: Box::new(puts_block(no)),
            span: s(),
        },
        span: s(),
    }
}

fn demo_module() -> Module {
    let stmts = vec![
        when_stmt(ilit(5), ilit(5), "A", "Z"),
        when_stmt(ilit(5), ilit(6), "B", "Y"),
        when_stmt(slit("a"), slit("a"), "C", "X"),
    ];

    Module {
        name: "case_eq_demo".into(),
        manifest: FeatureManifest::from_features(&[Feature::Strings]),
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

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn case_eq_compiles_and_matches_ruby_output() {
    let artifact = compile(&demo_module()).expect("compile to javascript");
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution");
        return;
    }
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_js_case_eq_{}.js", std::process::id()));
    std::fs::write(&path, &artifact.source).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero:\nstdout: {}\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        artifact.source,
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert_eq!(
        stdout.trim_end_matches(['\n', '\r']),
        "A\nY\nC",
        "case_eq did not match Ruby === semantics"
    );
}
