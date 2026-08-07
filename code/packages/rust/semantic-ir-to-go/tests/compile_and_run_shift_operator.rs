//! End-to-end proof for the `<<` builtin (Ruby's shift operator) as a
//! top-level `BuiltinCall("<<", [lhs, rhs, ...])` on the Go backend — part of
//! "Python/JS/Go/Rust/Ruby backends: implement `<<` runtime dispatch".
//!
//! `ruby-to-semantic-ir` lowers `<<` to this envelope (distinct from the
//! `__method__("<<", recv, arg)` Collections-dispatch protocol Array#push
//! already used on this backend). Before this fix, the top-level operator
//! reached `_sir_call_builtin_by_name`'s floor and panicked with
//! `unknown builtin: <<` — every Ruby program using `<<` as an operator
//! (`a << 1`, `arr << x`, `"a" << "b"`) failed at runtime on Go.
//!
//! This test hand-builds SIR modules exercising each receiver type, emits
//! Go, runs with `go run`, and asserts stdout:
//!
//!   * `5 << 2`                → `20`   (Integer left shift)
//!   * `5 << -1`               → `2`    (negative amount reverses direction)
//!   * `1 << 63`               → saturates to `math.MaxInt64` (no bignum growth)
//!   * `[1, 2] << 3`           → `[1, 2, 3]` (Array push, mutates receiver)
//!   * `"ab" << "cd"`          → `abcd` (String concat, new string)
//!
//! Gates on `go`; logs a skip when absent.

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope,
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

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn shift(args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: "<<".into(), args, effects: EffectSet::PURE, span: s() }
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

fn demo_module(name: &str, stmts: Vec<Stmt>) -> Module {
    Module {
        // A distinct name per test -- these run as parallel threads in the
        // same process (cargo test's default), so a shared name collides on
        // the same temp file path.
        name: name.into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Strings,
            Feature::Sequences,
            Feature::MutableBindings,
        ]),
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

fn run(module: &Module) -> Option<String> {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return None;
    }
    let artifact = compile(module).expect("module should compile to Go source");
    let dir = std::env::temp_dir();
    let src_path = dir.join(format!("sir_go_shift_{}_{}.go", std::process::id(), module.name));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let run_out = Command::new("go").arg("run").arg(&src_path).output().expect("invoke go run");
    let _ = std::fs::remove_file(&src_path);
    if !run_out.status.success() {
        panic!(
            "emitted Go failed to compile/run:\n--- stderr ---\n{}\n--- source ---\n{}",
            String::from_utf8_lossy(&run_out.stderr),
            artifact.source,
        );
    }
    Some(String::from_utf8_lossy(&run_out.stdout).replace("\r\n", "\n"))
}

#[test]
fn integer_shift_left() {
    let m = demo_module("shift_int", vec![puts_stmt(shift(vec![ilit(5), ilit(2)]))]);
    if let Some(out) = run(&m) {
        assert_eq!(out, "20\n");
    }
}

#[test]
fn negative_amount_reverses_direction() {
    let m = demo_module("shift_neg", vec![puts_stmt(shift(vec![ilit(5), ilit(-1)]))]);
    if let Some(out) = run(&m) {
        assert_eq!(out, "2\n");
    }
}

#[test]
fn overflow_saturates_instead_of_wrapping() {
    // `1 << 63` is one past int64's positive range (no bignum growth here,
    // unlike real Ruby) -- must saturate to MaxInt64, not silently wrap to
    // a negative number.
    let m = demo_module("shift_overflow", vec![puts_stmt(shift(vec![ilit(1), ilit(63)]))]);
    if let Some(out) = run(&m) {
        assert_eq!(out, "9223372036854775807\n");
    }
}

#[test]
fn array_receiver_pushes_in_place() {
    // `puts` on an Array unpacks one element per line (real Ruby's
    // `Kernel#puts` rule, already implemented on this backend) -- so the
    // mutated `[1, 2, 3]` receiver prints as three lines, not a bracketed
    // display, proving the push landed without depending on this
    // backend's bracket-display convention.
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
            puts_stmt(Expr::VarRef { name: "a".into(), scope: Scope::Local, span: s() }),
        ],
    );
    if let Some(out) = run(&m) {
        assert_eq!(out, "1\n2\n3\n");
    }
}

#[test]
fn string_receiver_concatenates_to_a_new_string() {
    let m = demo_module("shift_string", vec![puts_stmt(shift(vec![slit("ab"), slit("cd")]))]);
    if let Some(out) = run(&m) {
        assert_eq!(out, "abcd\n");
    }
}
