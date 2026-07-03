//! End-to-end proof for SIR19 **DefaultParams** in the Go backend (P2f).
//!
//! Go has no native optional/default parameters, so the backend uses a
//! RUNTIME-MIMIC strategy: a package-level MISSING sentinel
//! (`_sir_missing`) flows through the ordinary `Value` channel.  A
//! `DirectCall` that omits trailing defaulted arguments pads up to the
//! callee's full (fixed) arity with the sentinel; the callee's body
//! prologue swaps each sentinel for that param's default expression,
//! evaluated where the *earlier* params are already bound (call-time +
//! param-scope).
//!
//! Unit tests assert the emitted *shape*; this test goes the whole way:
//! it hand-builds the discriminating module, emits Go, writes it to a temp
//! `.go` file, runs it with `go run`, and checks stdout.  Only a real Go
//! toolchain can confirm the emitted source compiles under Go's strict
//! unused-var / unused-import rules and behaves — which is the entire
//! reason this feature is delicate.
//!
//! DISCRIMINATING MODULE: `f(a, b)` where `b`'s default is `(+ a 1)` (it
//! references the *earlier* param `a`).  `main` prints `f(5)` then
//! `f(5, 10)`.  Expected stdout: `6` then `10`.  The first call proves the
//! default ran *and* saw the supplied `a = 5` (param-scope, call-time); the
//! second proves a supplied argument suppresses the default.
//!
//! The test gates on `go` being available (`go version`); a missing
//! toolchain logs a skip rather than reddening an unrelated build (mirrors
//! `compile_and_run_loops.rs`).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Param,
    ParamKind, Scope, Span, Stmt,
};
use semantic_ir_to_go::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn param_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall {
        name: name.into(),
        args,
        effects: EffectSet::PURE,
        span: s(),
    }
}

/// `print(expr)` as an effectful statement.
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

/// `f(a, b = (+ a 1))` returning `(+ a b)` … no, simpler: returns `b`.
///
/// We make `f` return `b` so the printed value isolates exactly what the
/// default mechanism produced:
///   * `f(5)`     → `b` defaulted to `5 + 1` = `6`
///   * `f(5, 10)` → `b` supplied as `10`
fn defaulted_fn() -> Function {
    Function {
        name: "f".into(),
        params: vec![
            Param { name: "a".into(), kind: ParamKind::Required, sir_type: None, default: None, span: s() },
            Param {
                name: "b".into(),
                kind: ParamKind::Required,
                sir_type: None,
                default: Some(Box::new(call("+", vec![param_ref("a"), ilit(1)]))),
                span: s(),
            },
        ],
        return_type: None,
        captures: vec![],
        body: Block { stmts: vec![], value: param_ref("b"), span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    }
}

/// `main`: print `f(5)` (→ 6), then print `f(5, 10)` (→ 10).
fn demo_module() -> Module {
    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![
                print_stmt(Expr::DirectCall {
                    fn_name: "f".into(),
                    args: vec![ilit(5)],
                    effects: EffectSet::PURE,
                    span: s(),
                }),
                print_stmt(Expr::DirectCall {
                    fn_name: "f".into(),
                    args: vec![ilit(5), ilit(10)],
                    effects: EffectSet::PURE,
                    span: s(),
                }),
            ],
            value: Expr::NilLit { span: s() },
            span: s(),
        },
        effects: EffectSet::PURE.with(Effect::MayPrint),
        metadata: Metadata::new(),
        span: s(),
    };

    Module {
        name: "default_params_demo".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::DefaultParams,
            // Untyped params observe `DynamicTyping`; the validator requires
            // every observed feature to be declared.
            Feature::DynamicTyping,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![defaulted_fn(), main],
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
fn default_params_compile_and_run() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }

    // 1. Emit.
    let artifact = compile(&demo_module()).expect("module should compile to Go source");

    // 2. Write to a unique temp file (`go run` requires a `.go` extension).
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_default_params_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    // 3. Compile + run with `go run` (arg vector — no shell).
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

    // 4. Assert observable behaviour: the default ran and saw `a = 5`
    //    (→ 6); a supplied argument suppressed it (→ 10).
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines,
        vec!["6", "10"],
        "unexpected program output; full stdout:\n{stdout}"
    );

    // 5. Best-effort cleanup.
    let _ = std::fs::remove_file(&src_path);
}
