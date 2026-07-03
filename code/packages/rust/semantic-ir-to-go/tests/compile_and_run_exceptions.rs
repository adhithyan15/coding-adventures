//! Execution proof for E3 exception support: emit Go, run it with `go run`,
//! and assert the observable behaviour matches Ruby's `begin/rescue/ensure`
//! + `raise` semantics.
//!
//! Go has NO native try/catch — the backend models unwinding with `panic`
//! and a deferred `recover` (see `emit::emit_try_catch`).  These five cases
//! prove the mapping end-to-end:
//!
//!   (a) built-in ancestry — `raise ArgumentError; rescue StandardError => e`
//!       catches (ArgumentError descends from StandardError);
//!   (b) bare `rescue` catches anything;
//!   (c) an UNMATCHED rescue type re-panics — the program exits non-zero and
//!       the inner "caught" marker is NOT printed;
//!   (d) `ensure` runs on BOTH the caught and the uncaught (propagating) path;
//!   (e) USER ancestry — `class MyErr < StandardError; raise MyErr; rescue
//!       StandardError` catches via the registered edge.
//!
//! To avoid StrLit interpolation the Go backend rejects, every program prints
//! simple ASCII markers (no `#{}`).
//!
//! A missing `go` toolchain logs a skip rather than reddening the build
//! (mirrors `compile_and_run_cyclic.rs`).

use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, RescueClause,
    Scope, Span, Stmt,
};
use semantic_ir_to_go::compile;

fn s() -> Span {
    Span::synthetic()
}

fn str_lit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn print_stmt(v: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "print".into(),
            args: vec![v],
            effects: EffectSet::PURE,
            span: s(),
        },
        span: s(),
    }
}

fn print_marker(m: &str) -> Stmt {
    print_stmt(str_lit(m))
}

/// `raise ClassName` (const first arg), optionally with a message.
fn raise_class(class: &str) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "raise".into(),
            args: vec![Expr::VarRef { name: class.into(), scope: Scope::Const, span: s() }],
            effects: EffectSet::PURE,
            span: s(),
        },
        span: s(),
    }
}

/// Wrap `main`'s body statements into a runnable, validated module.
fn module_from(stmts: Vec<Stmt>, extra_features: &[Feature]) -> Module {
    let mut features = vec![Feature::Exceptions, Feature::Constants, Feature::Strings];
    features.extend_from_slice(extra_features);
    Module {
        name: "exc_demo".into(),
        manifest: FeatureManifest::from_features(&features),
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

/// Emit `m` to Go, run it, and return `(success, stdout)`.
fn emit_and_run(m: &Module, nonce: &str) -> (bool, String) {
    let artifact = compile(m).expect("module should compile to Go source");
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    let src_path = dir.join(format!("sir_go_exc_{pid}_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");
    let run_out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");
    if !run_out.status.success() {
        // A compile error (bad emit) is different from a *runtime* non-zero
        // exit (an uncaught panic, which cases (c)/(d) rely on).  Surface
        // compile errors loudly; return the flag for runtime failures.
        let stderr = String::from_utf8_lossy(&run_out.stderr);
        if stderr.contains("cannot") || stderr.contains("syntax error") || stderr.contains("undefined:") {
            let _ = std::fs::remove_file(&src_path);
            panic!("emitted Go failed to COMPILE:\n--- stderr ---\n{stderr}\n--- source ---\n{}", artifact.source);
        }
    }
    let stdout = String::from_utf8_lossy(&run_out.stdout).into_owned();
    let ok = run_out.status.success();
    let _ = std::fs::remove_file(&src_path);
    (ok, stdout)
}

/// (a) Built-in ancestry: `begin; raise ArgumentError; rescue StandardError
/// => e; print("caught"); end` → prints "caught" and exits 0.
#[test]
fn builtin_ancestry_rescue_catches() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let tc = Stmt::TryCatch {
        body: vec![raise_class("ArgumentError")],
        rescues: vec![RescueClause {
            exception_types: vec!["StandardError".into()],
            binding: Some("e".into()),
            body: vec![print_marker("caught")],
            span: s(),
        }],
        ensure_body: None,
        span: s(),
    };
    let (ok, out) = emit_and_run(&module_from(vec![tc], &[]), "a");
    assert!(ok, "should exit 0 (rescue matched); stdout:\n{out}");
    assert_eq!(out.trim(), "caught", "ArgumentError must be caught by rescue StandardError");
}

/// (b) Bare `rescue` catches anything.
#[test]
fn bare_rescue_catches() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let tc = Stmt::TryCatch {
        body: vec![raise_class("TypeError")],
        rescues: vec![RescueClause {
            exception_types: vec![],
            binding: None,
            body: vec![print_marker("bare-caught")],
            span: s(),
        }],
        ensure_body: None,
        span: s(),
    };
    let (ok, out) = emit_and_run(&module_from(vec![tc], &[]), "b");
    assert!(ok, "bare rescue should catch and exit 0; stdout:\n{out}");
    assert_eq!(out.trim(), "bare-caught");
}

/// (c) An unmatched rescue TYPE re-panics: the program exits non-zero and
/// the inner "should-not-print" marker never appears.
#[test]
fn unmatched_rescue_type_repanics() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    // raise TypeError, rescue only ZeroDivisionError (which does NOT match).
    let tc = Stmt::TryCatch {
        body: vec![raise_class("TypeError")],
        rescues: vec![RescueClause {
            exception_types: vec!["ZeroDivisionError".into()],
            binding: None,
            body: vec![print_marker("should-not-print")],
            span: s(),
        }],
        ensure_body: None,
        span: s(),
    };
    let (ok, out) = emit_and_run(&module_from(vec![tc], &[]), "c");
    assert!(!ok, "unmatched rescue must re-panic (non-zero exit); stdout:\n{out}");
    assert!(
        !out.contains("should-not-print"),
        "the non-matching handler body must NOT run; stdout:\n{out}"
    );
}

/// (d) `ensure` runs on BOTH the caught and the uncaught path.
#[test]
fn ensure_runs_on_both_paths() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }

    // (d1) Caught path: raise + matching rescue + ensure.  Expect both the
    // rescue marker and the ensure marker, in order, exit 0.
    let caught = Stmt::TryCatch {
        body: vec![raise_class("ArgumentError")],
        rescues: vec![RescueClause {
            exception_types: vec!["StandardError".into()],
            binding: None,
            body: vec![print_marker("rescued")],
            span: s(),
        }],
        ensure_body: Some(vec![print_marker("ensured")]),
        span: s(),
    };
    let (ok, out) = emit_and_run(&module_from(vec![caught], &[]), "d1");
    assert!(ok, "caught path should exit 0; stdout:\n{out}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines, vec!["rescued", "ensured"], "ensure must run AFTER the rescue body");

    // (d2) Uncaught path: raise + NON-matching rescue + ensure.  The rescue
    // re-panics, but `ensure` must STILL run before propagation, so "ensured"
    // is printed and the program exits non-zero.
    let uncaught = Stmt::TryCatch {
        body: vec![raise_class("TypeError")],
        rescues: vec![RescueClause {
            exception_types: vec!["ZeroDivisionError".into()],
            binding: None,
            body: vec![print_marker("should-not-print")],
            span: s(),
        }],
        ensure_body: Some(vec![print_marker("ensured")]),
        span: s(),
    };
    let (ok2, out2) = emit_and_run(&module_from(vec![uncaught], &[]), "d2");
    assert!(!ok2, "uncaught path must exit non-zero; stdout:\n{out2}");
    assert!(
        out2.contains("ensured"),
        "ensure must run even when the exception propagates; stdout:\n{out2}"
    );
    assert!(!out2.contains("should-not-print"), "non-matching handler must not run");
}

/// (e) USER ancestry: `class MyErr < StandardError; end; begin; raise MyErr;
/// rescue StandardError; print("user-caught"); end` → caught via the
/// registered edge.
#[test]
fn user_ancestry_rescue_catches() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let class = Stmt::ClassDef {
        name: "MyErr".into(),
        superclass: Some("StandardError".into()),
        body: vec![],
        span: s(),
    };
    let tc = Stmt::TryCatch {
        body: vec![raise_class("MyErr")],
        rescues: vec![RescueClause {
            exception_types: vec!["StandardError".into()],
            binding: None,
            body: vec![print_marker("user-caught")],
            span: s(),
        }],
        ensure_body: None,
        span: s(),
    };
    let (ok, out) = emit_and_run(&module_from(vec![class, tc], &[Feature::Classes]), "e");
    assert!(ok, "user MyErr must be caught by rescue StandardError; stdout:\n{out}");
    assert_eq!(out.trim(), "user-caught");
}
