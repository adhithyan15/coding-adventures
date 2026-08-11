//! End-to-end proof for `Array#[]=` (the OO-surface bracket-index write,
//! `arr[i] = v`) on the Rust backend — the gap in "Python/JS/Go/Rust
//! backends: implement `[]`/`[]=` bracket-index runtime dispatch".
//!
//! `arr[i] = v` lowers through the SAME `__method__("[]=", recv, i, v)`
//! envelope every Collections method uses (PR #9686), NOT the SIR-native
//! `SeqSet` statement — those are two separate paths that happen to need the
//! same semantics. Before this fix, `array_method`'s `"[]="` name was simply
//! absent, so any bracket-index write reached the `unknown_method` floor
//! (`NoMethodError`) at runtime — `Array#[]` (read) and `Hash#[]`/`Hash#[]=`
//! already worked; only `Array#[]=` was missing.
//!
//! The fix delegates to the pre-existing `seq_set` primitive (the same one
//! `SeqSet` already lowers to), so the two paths share one strictness rule:
//! `0 <= i < len`, panics outside — no auto-grow, matching the Go/C/Rust
//! `SeqSet` reference.

use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span, Stmt,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

/// `recv.meth(args…)` — the `__method__` dispatch envelope.
fn method(recv: Expr, name: &str, mut args: Vec<Expr>) -> Expr {
    let mut all = vec![recv, Expr::StrLit { value: name.into(), span: s() }];
    all.append(&mut args);
    Expr::BuiltinCall { name: "__method__".into(), args: all, effects: EffectSet::PURE, span: s() }
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
            effects: EffectSet::PURE,
            span: s(),
        },
        span: s(),
    }
}

fn demo_module(name: &str, main_stmts: Vec<Stmt>) -> Module {
    Module {
        // A distinct name per test -- these run as parallel threads in the
        // same process (cargo test's default), so a shared name collides on
        // the same temp file path and one test's compile/run clobbers
        // another's (the exact hazard `compile_and_run_c` hit; see that
        // crate's CHANGELOG).
        name: name.into(),
        manifest: FeatureManifest::from_features(&[Feature::ConsoleIO, 
            Feature::Sequences,
            Feature::Strings,
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts: main_stmts, value: Expr::NilLit { span: s() }, span: s() },
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

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn compile_and_run(module: &Module) -> Option<std::process::Output> {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return None;
    }
    let artifact = compile(module).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = format!("{}_{}", std::process::id(), module.name);
    let src_path = dir.join(format!("sir_bracket_write_{nonce}.rs"));
    let bin_path = dir.join(format!("sir_bracket_write_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compile_out = cmd.arg(&src_path).arg("-o").arg(&bin_path).output().expect("invoke rustc");
    if !compile_out.status.success() {
        let stderr = String::from_utf8_lossy(&compile_out.stderr);
        if stderr.contains("linker") && (stderr.contains("not found") || stderr.contains("No such file")) {
            eprintln!("skipping: no usable linker on host\n{stderr}");
            let _ = std::fs::remove_file(&src_path);
            return None;
        }
        panic!(
            "emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }
    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
    Some(run_out)
}

#[test]
fn array_bracket_write_is_visible_on_a_later_read() {
    let m = demo_module("bracket_write_visible_on_read", vec![
        Stmt::LetBinding {
            name: "a".into(),
            sir_type: None,
            value: seq(vec![ilit(10), ilit(20), ilit(30)]),
            span: s(),
        },
        Stmt::ExprStmt {
            expr: method(
                Expr::VarRef { name: "a".into(), scope: semantic_ir::Scope::Local, span: s() },
                "[]=",
                vec![ilit(1), ilit(99)],
            ),
            span: s(),
        },
        print_stmt(method(
            Expr::VarRef { name: "a".into(), scope: semantic_ir::Scope::Local, span: s() },
            "[]",
            vec![ilit(1)],
        )),
    ]);
    if let Some(out) = compile_and_run(&m) {
        assert!(out.status.success(), "compiled binary exited non-zero:\n{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), "99\n");
    }
}

#[test]
fn array_bracket_write_returns_the_assigned_value() {
    // Ruby: `arr[i] = v` evaluates to `v` (the RHS), not the receiver.
    let m = demo_module("bracket_write_returns_rhs", vec![
        Stmt::LetBinding {
            name: "a".into(),
            sir_type: None,
            value: seq(vec![ilit(1), ilit(2)]),
            span: s(),
        },
        print_stmt(method(
            Expr::VarRef { name: "a".into(), scope: semantic_ir::Scope::Local, span: s() },
            "[]=",
            vec![ilit(0), ilit(42)],
        )),
    ]);
    if let Some(out) = compile_and_run(&m) {
        assert!(out.status.success(), "compiled binary exited non-zero:\n{}", String::from_utf8_lossy(&out.stderr));
        assert_eq!(String::from_utf8_lossy(&out.stdout), "42\n");
    }
}

#[test]
fn array_bracket_write_out_of_range_panics_not_silently_no_ops() {
    // Matches SeqSet's existing strictness (0 <= i < len, no auto-grow) --
    // this test proves the OO-surface path shares that trap rather than
    // silently doing nothing or corrupting memory.
    let m = demo_module("bracket_write_out_of_range", vec![
        Stmt::LetBinding {
            name: "a".into(),
            sir_type: None,
            value: seq(vec![ilit(1)]),
            span: s(),
        },
        Stmt::ExprStmt {
            expr: method(
                Expr::VarRef { name: "a".into(), scope: semantic_ir::Scope::Local, span: s() },
                "[]=",
                vec![ilit(5), ilit(1)],
            ),
            span: s(),
        },
    ]);
    if let Some(out) = compile_and_run(&m) {
        assert!(!out.status.success(), "expected a panic (non-zero exit) on out-of-range []=");
    }
}
