//! End-to-end proof for **M6 universal Object metaprogramming** in the Go
//! backend — the Go analogue of the Python/TS `sir-runtime-oop` M6 surface
//! (`send`/`__send__`/`public_send`, `tap`, `then`/`yield_self`,
//! `respond_to?`, and boolean `&`/`|`/`^`).
//!
//! Unit tests (`src/emit.rs` / `src/runtime.rs`) assert the runtime *carries*
//! the M6 helpers.  This test goes the whole way: it hand-builds SIR modules
//! that exercise the surface, emits Go, runs it under a real `go run`, and
//! diffs stdout against the values the Ruby/Python/TS reference produce for
//! the SAME operation:
//!
//!   * `"hello".send(:upcase)` re-enters dispatch → `"HELLO"`.
//!   * `[1,2,3].send(:map, &block)` forwards the trailing block → `[2,4,6]`.
//!   * `5.tap { … }` runs the block and returns the RECEIVER → `5`.
//!   * `5.then { |x| x*2 }` returns the BLOCK RESULT → `10`.
//!   * `"x".respond_to?(:upcase)` → true ; `:bogus` → false.
//!   * `true & false` / `true | false` / `true ^ true` — eager boolean logic.
//!
//! A separate module proves an **unknown `send`** (`[1].send(:bogus_xyz)`)
//! fails CLEANLY — a non-zero `go run` exit carrying the controlled
//! "undefined method" message routed through the SAME catalog a direct call
//! walks (the C3 dynamic-dispatch discipline: no reflection on the name).
//!
//! Gated on `go version`: a missing toolchain logs a skip rather than
//! reddening the build (mirrors `compile_and_run_coll_methods.rs`).

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

fn blit(v: bool) -> Expr {
    Expr::BoolLit { value: v, span: s() }
}

fn sym(name: &str) -> Expr {
    Expr::SymLit { name: name.into(), span: s() }
}

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn var_p(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
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

/// One-parameter lambda `fn_name(x) -> body`, registered top-level; the caller
/// builds a `MakeClosure` referencing it.
fn lambda_fn(fn_name: &str, param: &str, body: Expr) -> Function {
    Function {
        name: fn_name.into(),
        params: vec![semantic_ir::Param {
            name: param.into(),
            kind: semantic_ir::ParamKind::Required,
            sir_type: None,
            default: None,
            span: s(),
        }],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: body, span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    }
}

fn closure(fn_name: &str) -> Expr {
    Expr::MakeClosure { fn_name: fn_name.into(), captures: vec![], span: s() }
}

fn manifest() -> FeatureManifest {
    FeatureManifest::from_features(&[
        Feature::Closures,
        Feature::Sequences,
        Feature::Maps,
        Feature::Strings,
        Feature::Symbols,
        Feature::MutableBindings,
        Feature::DynamicTyping,
    ])
}

fn program(functions: Vec<Function>) -> Module {
    Module {
        name: "m6_meta_demo".into(),
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

/// The M6 demo `main`.  Each printed line's expected value is what the
/// Ruby/Python/TS reference runtime yields for the identical operation.
fn m6_module() -> Module {
    // Blocks the meta methods drive.
    let double = lambda_fn("__lam_double", "x", builtin("*", vec![var_p("x"), ilit(2)]));
    // `tap`'s block has a side effect but its result is DISCARDED (tap returns
    // the receiver); we make it a pure identity so it is observable only via
    // the returned receiver.
    let ident = lambda_fn("__lam_ident", "x", var_p("x"));

    let stmts = vec![
        // "hello".send(:upcase) → "HELLO"  (send re-enters dispatch)
        print_stmt(method(slit("hello"), "send", vec![sym("upcase")])),
        // "hi".__send__(:upcase) → "HI"
        print_stmt(method(slit("hi"), "__send__", vec![sym("upcase")])),
        // "yo".public_send(:upcase) → "YO"
        print_stmt(method(slit("yo"), "public_send", vec![sym("upcase")])),
        // [1,2,3].send(:map, &double) → [2, 4, 6]  (trailing block forwarded)
        print_stmt(method(
            seq(vec![ilit(1), ilit(2), ilit(3)]),
            "send",
            vec![sym("map"), closure("__lam_double")],
        )),
        // "abc".send(:length) → 3  (send with a numeric result)
        print_stmt(method(slit("abc"), "send", vec![sym("length")])),
        // 5.tap { |x| x } → 5  (tap returns the RECEIVER)
        print_stmt(method(ilit(5), "tap", vec![closure("__lam_ident")])),
        // 5.then { |x| x*2 } → 10  (then returns the BLOCK RESULT)
        print_stmt(method(ilit(5), "then", vec![closure("__lam_double")])),
        // 5.yield_self { |x| x*2 } → 10  (alias of then)
        print_stmt(method(ilit(5), "yield_self", vec![closure("__lam_double")])),
        // 7.tap (block-less) → 7  (v0 floor: returns receiver)
        print_stmt(method(ilit(7), "tap", vec![])),
        // "x".respond_to?(:upcase) → #t  (catalog method)
        print_stmt(method(slit("x"), "respond_to?", vec![sym("upcase")])),
        // "x".respond_to?(:bogus_xyz) → #f  (out-of-catalog → honest false)
        print_stmt(method(slit("x"), "respond_to?", vec![sym("bogus_xyz")])),
        // [1].respond_to?(:map) → #t  (block method reported)
        print_stmt(method(seq(vec![ilit(1)]), "respond_to?", vec![sym("map")])),
        // 5.respond_to?(:send) → #t  (universal M6 method on every receiver)
        print_stmt(method(ilit(5), "respond_to?", vec![sym("send")])),
        // true & false → #f
        print_stmt(method(blit(true), "&", vec![blit(false)])),
        // true | false → #t
        print_stmt(method(blit(true), "|", vec![blit(false)])),
        // false | true → #t
        print_stmt(method(blit(false), "|", vec![blit(true)])),
        // true ^ true → #f
        print_stmt(method(blit(true), "^", vec![blit(true)])),
        // true ^ false → #t
        print_stmt(method(blit(true), "^", vec![blit(false)])),
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

    program(vec![double, ident, main])
}

/// A module that `send`s an out-of-catalog method — must fail cleanly through
/// the SAME NoMethodError floor a direct call hits (no reflection).
fn unknown_send_module() -> Module {
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![print_stmt(method(
                seq(vec![ilit(1)]),
                "send",
                vec![sym("bogus_xyz")],
            ))],
            value: Expr::NilLit { span: s() },
            span: s(),
        },
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
    let src_path = dir.join(format!("sir_go_m6_{tag}_{nonce}.go"));
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
fn m6_metaprogramming_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let artifact = compile(&m6_module()).expect("module should compile to Go source");
    let run_out = run_go(&artifact.source, "meta");
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
            "HELLO",     // "hello".send(:upcase)
            "HI",        // "hi".__send__(:upcase)
            "YO",        // "yo".public_send(:upcase)
            "[2, 4, 6]", // [1,2,3].send(:map, &double)
            "3",         // "abc".send(:length)
            "5",         // 5.tap { x } → receiver
            "10",        // 5.then { x*2 } → block result
            "10",        // 5.yield_self { x*2 }
            "7",         // 7.tap (block-less) → receiver
            "#t",        // "x".respond_to?(:upcase)
            "#f",        // "x".respond_to?(:bogus_xyz)
            "#t",        // [1].respond_to?(:map)
            "#t",        // 5.respond_to?(:send)
            "#f",        // true & false
            "#t",        // true | false
            "#t",        // false | true
            "#f",        // true ^ true
            "#t",        // true ^ false
        ],
        "unexpected stdout:\n{stdout}"
    );
}

#[test]
fn unknown_send_fails_cleanly() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let artifact = compile(&unknown_send_module()).expect("compiles to Go");
    let run_out = run_go(&artifact.source, "unknown_send");
    // A controlled failure: non-zero exit AND the "undefined method" message,
    // routed through the SAME catalog a direct `.bogus_xyz` call walks — NOT a
    // reflective dispatch, NOT a silent nil.
    assert!(
        !run_out.status.success(),
        "unknown send must fail, not succeed; stdout: {}",
        String::from_utf8_lossy(&run_out.stdout)
    );
    let stderr = String::from_utf8_lossy(&run_out.stderr);
    assert!(
        stderr.contains("undefined method") && stderr.contains("bogus_xyz"),
        "expected a controlled undefined-method panic; stderr:\n{stderr}"
    );
}
