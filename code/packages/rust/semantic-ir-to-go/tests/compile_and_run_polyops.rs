//! End-to-end proof for the **polymorphic `+` / `*`** operators
//! (sir-polymorphic-operators, PO4) in the Go backend.
//!
//! Ruby overloads `+` and `*` by receiver type, and every case lowers to
//! the same SIR builtins (`_sir_plus` / `_sir_times`).  Before PO4 the Go
//! runtime helpers were NUMERIC-ONLY (`_sir_as_int`/`_sir_as_float` on
//! every operand), so `"a" + "b"` and `[1] + [2]` produced garbage or
//! panicked.  A *shape* assertion cannot catch that — we must prove the
//! emitted Go actually produces the byte stream Ruby would.
//!
//! This test hand-builds a SIR module equivalent to the Ruby program
//!
//!     print "a" + "b"      # => ab
//!     print "ab" * 3       # => ababab
//!     print [1] + [2]      # => [1, 2]  (Go backend array display)
//!     print [0] * 3        # => [0, 0, 0]
//!     print [1, 2] * ", "  # => 1, 2    (Array join via *)
//!     print 1 + 2          # => 3       (numeric regression)
//!     print 2 * 3          # => 6       (numeric regression)
//!
//! emits Go, runs it with `go run`, and asserts stdout is EXACTLY the
//! concatenation of each `print`'s output (one line each, `print` uses
//! `_sir_format` + a trailing newline).
//!
//! Gates on `go` being available; logs a skip rather than failing when the
//! toolchain is absent (mirrors `compile_and_run_puts.rs`).

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

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

/// `(name arg0 arg1 ...)` builtin call, pure.
fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

/// `print(expr)` as an effectful statement — `_sir_print` renders the
/// value via `_sir_format` and appends a newline, so each call emits
/// exactly one line and the results are trivially assertable.
fn print_stmt(expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "print".into(),
            args: vec![expr],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// Build a module whose `main` prints each polymorphic-operator result on
/// its own line, in the order documented in the module comment above.
fn demo_module() -> Module {
    let stmts = vec![
        // "a" + "b" → ab
        print_stmt(call("+", vec![slit("a"), slit("b")])),
        // "ab" * 3 → ababab
        print_stmt(call("*", vec![slit("ab"), ilit(3)])),
        // [1] + [2] → [1, 2]
        print_stmt(call("+", vec![seq(vec![ilit(1)]), seq(vec![ilit(2)])])),
        // [0] * 3 → [0, 0, 0]
        print_stmt(call("*", vec![seq(vec![ilit(0)]), ilit(3)])),
        // [1, 2] * ", " → 1, 2
        print_stmt(call("*", vec![seq(vec![ilit(1), ilit(2)]), slit(", ")])),
        // 1 + 2 → 3 (numeric regression)
        print_stmt(call("+", vec![ilit(1), ilit(2)])),
        // 2 * 3 → 6 (numeric regression)
        print_stmt(call("*", vec![ilit(2), ilit(3)])),
    ];

    Module {
        name: "polyops_demo".into(),
        manifest: FeatureManifest::from_features(&[Feature::Sequences, Feature::Strings]),
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
fn polymorphic_ops_compile_and_match_ruby_output() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }

    let artifact = compile(&demo_module()).expect("module should compile to Go source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_polyops_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let run_out = Command::new("go").arg("run").arg(&src_path).output().expect("invoke go run");

    if !run_out.status.success() {
        let stderr = String::from_utf8_lossy(&run_out.stderr);
        let _ = std::fs::remove_file(&src_path);
        panic!(
            "emitted Go failed to compile/run:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    let stdout = String::from_utf8_lossy(&run_out.stdout);
    // Normalise CRLF (Go's fmt may emit CRLF on Windows) so the assertion
    // tests semantics, not the platform newline convention.
    let normalised = stdout.replace("\r\n", "\n");
    assert_eq!(
        normalised,
        // "a"+"b"  "ab"*3   [1]+[2]   [0]*3        [1,2]*", "  1+2  2*3
        "ab\nababab\n[1, 2]\n[0, 0, 0]\n1, 2\n3\n6\n",
        "unexpected polymorphic-op output; full stdout (escaped): {stdout:?}"
    );

    let _ = std::fs::remove_file(&src_path);
}
