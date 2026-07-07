//! End-to-end proof for the Go backend's **String-method catalog** — the Go
//! analogue of the Python/TS `sir-runtime-oop` `_string_method` dispatch.
//!
//! It hand-builds a SIR module exercising the newly-added String methods
//! (`capitalize`, `chomp`, `bytes`, `index`, `replace`, `sub`, `gsub`) plus a
//! few pre-existing ones for regression coverage (`chars`, `split`,
//! `start_with?`, `reverse`), emits Go, runs it under a real `go run`, and
//! diffs stdout against the values the Python/TS reference backends yield for
//! the SAME operations.  `sub`/`gsub` are LITERAL (no regex / no `$&`), matching
//! the reference runtimes exactly.
//!
//! Gated on `go version`: a missing toolchain logs a skip rather than
//! reddening the build (mirrors `compile_and_run_coll_methods.rs`).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    Stmt,
};
use semantic_ir_to_go::compile;

fn s() -> Span {
    Span::synthetic()
}

fn slit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
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

fn manifest() -> FeatureManifest {
    FeatureManifest::from_features(&[
        Feature::Strings,
        Feature::Symbols,
        Feature::Sequences,
        Feature::DynamicTyping,
    ])
}

fn program(functions: Vec<Function>) -> Module {
    Module {
        name: "string_methods_demo".into(),
        manifest: manifest(),
        imports: vec![],
        exports: vec![],
        functions,
        globals: vec![],
        metadata: Metadata::new()
            .with_source_language("test")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
        span: s(),
    }
}

/// Each printed line's expected value is what the Python/TS reference runtime
/// yields for the identical operation.
fn catalog_module() -> Module {
    let stmts = vec![
        // "hello".capitalize → "Hello"
        print_stmt(method(slit("hello"), "capitalize", vec![])),
        // "hELLO".capitalize → "Hello"  (first up, rest DOWN)
        print_stmt(method(slit("hELLO"), "capitalize", vec![])),
        // "hi\n".chomp → "hi"
        print_stmt(method(slit("hi\n"), "chomp", vec![])),
        // "hi\r\n".chomp → "hi"  (one CRLF)
        print_stmt(method(slit("hi\r\n"), "chomp", vec![])),
        // "hello!".chomp("!") → "hello"  (explicit separator)
        print_stmt(method(slit("hello!"), "chomp", vec![slit("!")])),
        // "abc".chars → [a, b, c]
        print_stmt(method(slit("abc"), "chars", vec![])),
        // "hi".bytes → [104, 105]
        print_stmt(method(slit("hi"), "bytes", vec![])),
        // "a-b-c".split("-") → [a, b, c]
        print_stmt(method(slit("a-b-c"), "split", vec![slit("-")])),
        // "foobar".sub("o","0") → "f0obar"  (first only)
        print_stmt(method(slit("foobar"), "sub", vec![slit("o"), slit("0")])),
        // "foobar".gsub("o","0") → "f00bar"  (all)
        print_stmt(method(slit("foobar"), "gsub", vec![slit("o"), slit("0")])),
        // "hello".index("l") → 2
        print_stmt(method(slit("hello"), "index", vec![slit("l")])),
        // "hello".index("z") → nil
        print_stmt(method(slit("hello"), "index", vec![slit("z")])),
        // "old".replace("new") → "new"
        print_stmt(method(slit("old"), "replace", vec![slit("new")])),
        // "hi".start_with?("h") → #t
        print_stmt(method(slit("hi"), "start_with?", vec![slit("h")])),
        // "abc".reverse → "cba"
        print_stmt(method(slit("abc"), "reverse", vec![])),
    ];

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block { stmts, value: Expr::NilLit { span: s() }, span: s() },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };

    program(vec![main])
}

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_go(source: &str, tag: &str) -> std::process::Output {
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_strm_{tag}_{nonce}.go"));
    std::fs::write(&src_path, source).expect("write temp source");
    let out = Command::new("go")
        .arg("run")
        .arg(&src_path)
        .output()
        .expect("invoke go run");
    let _ = std::fs::remove_file(&src_path);
    out
}

#[test]
fn string_methods_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let artifact = compile(&catalog_module()).expect("module should compile to Go source");
    let run_out = run_go(&artifact.source, "catalog");
    if !run_out.status.success() {
        panic!(
            "emitted Go failed:\n--- stderr ---\n{}\n--- source ---\n{}",
            String::from_utf8_lossy(&run_out.stderr),
            artifact.source,
        );
    }
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec![
            "Hello",      // "hello".capitalize
            "Hello",      // "hELLO".capitalize
            "hi",         // "hi\n".chomp
            "hi",         // "hi\r\n".chomp
            "hello",      // "hello!".chomp("!")
            "[a, b, c]",  // "abc".chars
            "[104, 105]", // "hi".bytes
            "[a, b, c]",  // "a-b-c".split("-")
            "f0obar",     // "foobar".sub("o","0")
            "f00bar",     // "foobar".gsub("o","0")
            "2",          // "hello".index("l")
            "nil",        // "hello".index("z")
            "new",        // "old".replace("new")
            "#t",         // "hi".start_with?("h")
            "cba",        // "abc".reverse
        ],
        "unexpected stdout:\n{stdout}"
    );
}
