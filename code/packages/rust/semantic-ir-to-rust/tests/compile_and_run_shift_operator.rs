//! End-to-end proof for the `<<` builtin (Ruby's shift operator) as a
//! top-level `BuiltinCall("<<", [lhs, rhs, ...])` on the Rust backend — part
//! of "Python/JS/Rust/Ruby backends: implement shift-operator runtime
//! dispatch".
//!
//! `ruby-to-semantic-ir` lowers `<<` to this envelope (distinct from the
//! `__method__("<<", recv, arg)` Collections-dispatch protocol). Before this
//! fix, the top-level operator reached `call_builtin_by_name`'s floor and
//! panicked `unknown builtin: <<` — every Ruby program using `<<` as an
//! operator failed at runtime on Rust.
//!
//! This test hand-builds SIR modules exercising each receiver type, emits
//! Rust, compiles with `rustc`, runs the binary, and asserts stdout:
//!
//!   * `5 << 2`       → `20`   (Integer left shift)
//!   * `5 << -1`      → `2`    (negative amount reverses direction)
//!   * `1 << 63`      → saturates to `i64::MAX` (no bignum growth)
//!   * `[1, 2] << 3`  → `[1, 2, 3]` (Array push, mutates receiver)
//!   * `"ab" << "cd"` → `abcd` (String concat, new string)
//!
//! Gates on `rustc`; logs a skip when absent.

use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope, Span,
    Stmt,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn shift(args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: "<<".into(), args, effects: EffectSet::PURE, span: s() }
}

fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "print".into(),
            args: vec![expr],
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
        // the same temp file path.
        name: name.into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Strings,
            Feature::Closures,
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

fn compile_and_run(module: &Module) -> Option<String> {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return None;
    }
    let artifact = compile(module).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = format!("{}_{}", std::process::id(), module.name);
    let src_path = dir.join(format!("sir_shift_{nonce}.rs"));
    let bin_path = dir.join(format!("sir_shift_{nonce}{}", if cfg!(windows) { ".exe" } else { "" }));
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
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero:\n{}",
        String::from_utf8_lossy(&run_out.stderr)
    );
    Some(String::from_utf8_lossy(&run_out.stdout).replace("\r\n", "\n"))
}

#[test]
fn empty_args_does_not_panic() {
    // A hand-built `BuiltinCall("<<", [])` (no receiver at all) must not
    // panic on an empty-slice index -- caught by security review: the
    // integer fallback arm indexed `args[1..]` before checking `args` was
    // non-empty.
    let m = demo_module("shift_empty", vec![print_stmt(shift(vec![]))]);
    if let Some(out) = compile_and_run(&m) {
        assert_eq!(out, "0\n");
    }
}

#[test]
fn integer_shift_left() {
    let m = demo_module("shift_int", vec![print_stmt(shift(vec![ilit(5), ilit(2)]))]);
    if let Some(out) = compile_and_run(&m) {
        assert_eq!(out, "20\n");
    }
}

#[test]
fn negative_amount_reverses_direction() {
    let m = demo_module("shift_neg", vec![print_stmt(shift(vec![ilit(5), ilit(-1)]))]);
    if let Some(out) = compile_and_run(&m) {
        assert_eq!(out, "2\n");
    }
}

#[test]
fn overflow_saturates_instead_of_wrapping() {
    let m = demo_module("shift_overflow", vec![print_stmt(shift(vec![ilit(1), ilit(63)]))]);
    if let Some(out) = compile_and_run(&m) {
        assert_eq!(out, "9223372036854775807\n");
    }
}

#[test]
fn array_receiver_pushes_in_place() {
    let m = demo_module(
        "shift_array",
        vec![
            Stmt::LetBinding {
                name: "a".into(),
                sir_type: None,
                value: seq(vec![ilit(1), ilit(2)]),
                span: s(),
            },
            Stmt::ExprStmt {
                expr: shift(vec![
                    Expr::VarRef { name: "a".into(), scope: Scope::Local, span: s() },
                    ilit(3),
                ]),
                span: s(),
            },
            print_stmt(Expr::VarRef { name: "a".into(), scope: Scope::Local, span: s() }),
        ],
    );
    if let Some(out) = compile_and_run(&m) {
        assert_eq!(out, "[1, 2, 3]\n");
    }
}

#[test]
fn string_receiver_concatenates_to_a_new_string() {
    let m = demo_module("shift_string", vec![print_stmt(shift(vec![slit("ab"), slit("cd")]))]);
    if let Some(out) = compile_and_run(&m) {
        assert_eq!(out, "abcd\n");
    }
}
