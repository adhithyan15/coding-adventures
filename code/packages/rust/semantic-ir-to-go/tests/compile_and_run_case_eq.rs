//! End-to-end proof for the `case_eq` builtin (Ruby case-equality, `===`) in
//! the Go backend.
//!
//! Ruby's `case`/`when` lowers (in the frontend) to a chain of `if`s whose
//! conditions are `BuiltinCall("case_eq", [pattern, scrutinee])`.  Before this
//! fix the Go runtime had no `case_eq`, so **every** `case`/`when` program hit
//! `_sir_call_builtin_by_name`'s floor and panicked with
//! `unknown builtin: case_eq` at runtime — `case` was unusable on this backend.
//!
//! This test hand-builds the SIR equivalent of a `when`-style dispatch (a
//! `case_eq` used as an `if` condition — exactly how the frontend emits it):
//!
//!     if 5 === 5     then puts "A" else puts "Z"   # A  (match)
//!     if 5 === 6     then puts "B" else puts "Y"   # Y  (no match)
//!     if "a" === "a" then puts "C" else puts "X"   # C  (string match)
//!     if :x === :x   then puts "D" else puts "W"   # D  (structural symbol match)
//!
//! emits Go, runs it with `go run`, and asserts stdout is exactly
//! `A\nY\nC\nD\n`.  Driving an `if` off `case_eq` tests the boolean it returns
//! without depending on how this backend renders a bare boolean (Go/Rust format
//! `true` as the Lisp-style `#t`, a separate pre-existing convention).  Gates on
//! `go`; logs a skip when absent.

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module,
    Span, Stmt,
};
use semantic_ir_to_go::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn sym(name: &str) -> Expr {
    Expr::SymLit { name: name.into(), span: s() }
}

/// `case_eq(pattern, value)` — the narrow-waist envelope a `when` arm runs.
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
        when_stmt(sym("x"), sym("x"), "D", "W"),
    ];

    Module {
        name: "case_eq_demo".into(),
        manifest: FeatureManifest::from_features(&[Feature::Strings, Feature::Symbols]),
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

#[test]
fn case_eq_compiles_and_matches_ruby_output() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }

    let artifact = compile(&demo_module()).expect("module should compile to Go source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_case_eq_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let run_out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");

    if !run_out.status.success() {
        let stderr = String::from_utf8_lossy(&run_out.stderr);
        let _ = std::fs::remove_file(&src_path);
        panic!(
            "emitted Go failed to compile/run:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    let stdout = String::from_utf8_lossy(&run_out.stdout).replace("\r\n", "\n");
    let _ = std::fs::remove_file(&src_path);
    assert_eq!(
        stdout, "A\nY\nC\nD\n",
        "case_eq did not match Ruby === semantics"
    );
}
