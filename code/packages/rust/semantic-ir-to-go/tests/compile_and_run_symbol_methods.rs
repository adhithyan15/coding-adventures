//! End-to-end proof for the **Ruby Symbol method catalog** in the Go backend
//! (`_sir_symbol_method`) — parity-fill with the Python + TypeScript
//! `sir-runtime-oop` Symbol catalogs plus the task-mandated `capitalize` /
//! `to_proc`.
//!
//! Unit tests assert the emitted *shape*; this test goes the whole way: it
//! hand-builds a SIR module that exercises the Symbol methods on a symbol
//! literal receiver, emits Go, writes it to a temp `.go` file, runs it with
//! `go run`, and checks stdout — proving the emitted Go actually compiles and
//! behaves under a real Go toolchain (mirrors `compile_and_run_seq_maps.rs`).
//!
//! Note on formatting: a `*Symbol` is printed by its BARE name (see
//! `_sir_format_d`), so `:abc.upcase` (a `*Symbol` `:ABC`) renders as `ABC`,
//! and `:x.inspect` (a String `":x"`) renders as `:x`.
//!
//! Gates on `go` being available; logs a skip instead of failing when the Go
//! toolchain is absent (a missing tool must never redden an unrelated build).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    Stmt,
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

fn symlit(name: &str) -> Expr {
    Expr::SymLit { name: name.into(), span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn builtin(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall { name: name.into(), args, effects: EffectSet::PURE, span: s() }
}

/// `recv.meth(extra…)` → `BuiltinCall("__method__", [recv, "meth", …extra])`.
fn method(recv: Expr, name: &str, extra: Vec<Expr>) -> Expr {
    let mut args = vec![recv, slit(name)];
    args.extend(extra);
    builtin("__method__", args)
}

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

/// Build a module whose `main` prints, one per line:
///   1. `:hello.to_s`                 → "hello"  (String)
///   2. `:hi.length`                  → "2"
///   3. `:abc.upcase`                 → "ABC"    (Symbol, bare-name print)
///   4. `:ABC.downcase`               → "abc"    (Symbol)
///   5. `:hELLO.capitalize`           → "Hello"  (Symbol)
///   6. `:x.inspect`                  → ":x"     (String)
///   7. `[1,2,3].map(&:to_s).join`    → "123"    (Symbol#to_proc via block-pass)
///   8. `[10,20].map(&(:to_s.to_proc)).join` — but simpler: prove explicit
///      `.to_proc` yields a usable proc by mapping with it. We instead exercise
///      the explicit-catalog `to_proc` through `[4,5].map(sym.to_proc)` where
///      the proc is passed as a trailing block arg (a `*Closure`).
fn demo_module() -> Module {
    let stmts = vec![
        print_stmt(method(symlit("hello"), "to_s", vec![])),
        print_stmt(method(symlit("hi"), "length", vec![])),
        print_stmt(method(symlit("abc"), "upcase", vec![])),
        print_stmt(method(symlit("ABC"), "downcase", vec![])),
        print_stmt(method(symlit("hELLO"), "capitalize", vec![])),
        print_stmt(method(symlit("x"), "inspect", vec![])),
        // `&:to_s` block-pass form (frontend-lowered to `_sir_sym_to_proc`).
        print_stmt(method(
            method(
                seq(vec![ilit(1), ilit(2), ilit(3)]),
                "map",
                vec![builtin("block_pass", vec![symlit("to_s")])],
            ),
            "join",
            vec![],
        )),
        // Explicit `Symbol#to_proc` from the catalog: `:to_s.to_proc` returns a
        // `*Closure` that, passed as the trailing block arg to `map`, drives the
        // same per-element dispatch. Proves the catalog `to_proc` arm works.
        print_stmt(method(
            method(
                seq(vec![ilit(4), ilit(5), ilit(6)]),
                "map",
                vec![method(symlit("to_s"), "to_proc", vec![])],
            ),
            "join",
            vec![],
        )),
    ];

    Module {
        name: "symbol_methods_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::Sequences,
            Feature::Strings,
            Feature::Symbols,
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

#[test]
fn symbol_methods_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }

    let artifact = compile(&demo_module()).expect("module should compile to Go source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_symbol_methods_{nonce}.go"));
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

    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec!["hello", "2", "ABC", "abc", "Hello", ":x", "123", "456"],
        "unexpected program output; full stdout:\n{stdout}"
    );

    let _ = std::fs::remove_file(&src_path);
}
