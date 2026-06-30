//! End-to-end proof for **DefaultParams** (P2e) in the Rust backend.
//!
//! Unit tests assert the *shape* of the emitted source (the body-top
//! prologue + the padded full-arity call); this test closes the loop:
//! it hand-builds a SIR module that exercises a default referencing an
//! *earlier* parameter, emits Rust, compiles it with `rustc`, runs the
//! binary, and checks stdout.  That proves the missing-sentinel
//! runtime-mimic strategy actually compiles and behaves with call-time,
//! param-scope semantics.
//!
//! Discriminating program (`f` returns `b`, so the printed value is the
//! resolved parameter — exposing whether the default actually ran):
//!
//! ```text
//!   f(a, b = a + 1) -> b
//!   print(f(5))        // b omitted ⇒ default a + 1 = 6   → "6"
//!   print(f(5, 10))    // b supplied = 10                 → "10"
//! ```
//!
//! `rustc` ships with every Rust toolchain, so this runs in CI.  A
//! missing `rustc` or linker logs a skip rather than reddening the
//! build (matching the sibling `compile_and_run_*` tests).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module,
    Param, ParamKind, Scope, Span,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn ilit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn param_ref(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Param, span: s() }
}

/// `(name arg0 arg1 ...)` builtin call, pure.
fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::BuiltinCall {
        name: name.into(),
        args,
        effects: EffectSet::PURE,
        span: s(),
    }
}

/// `(f arg0 arg1 ...)` direct call.
fn direct(fn_name: &str, args: Vec<Expr>) -> Expr {
    Expr::DirectCall {
        fn_name: fn_name.into(),
        args,
        effects: EffectSet::PURE,
        span: s(),
    }
}

/// `print(expr)` as an effectful statement.
fn print_stmt(expr: Expr) -> semantic_ir::Stmt {
    semantic_ir::Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "print".into(),
            args: vec![expr],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// Build the discriminating module:
///   f(a, b = a + 1) -> b
///   main: print(f(5)); print(f(5, 10))
fn demo_module() -> Module {
    // b's default = (+ a 1), referencing the EARLIER param `a`.
    let default_b = call("+", vec![param_ref("a"), ilit(1)]);

    let f = Function {
        name: "f".into(),
        params: vec![
            Param { name: "a".into(), kind: ParamKind::Required, sir_type: None, default: None, span: s() },
            Param {
                name: "b".into(),
                kind: ParamKind::Required,
                sir_type: None,
                default: Some(Box::new(default_b)),
                span: s(),
            },
        ],
        return_type: None,
        captures: vec![],
        // f returns b — so the printed value IS the resolved default.
        body: Block { stmts: vec![], value: param_ref("b"), span: s() },
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: s(),
    };

    let main = Function {
        name: "main".into(),
        params: vec![],
        return_type: None,
        captures: vec![],
        body: Block {
            stmts: vec![
                // f(5)      → b defaulted to a + 1 = 6   → "6"
                print_stmt(direct("f", vec![ilit(5)])),
                // f(5, 10)  → b supplied = 10            → "10"
                print_stmt(direct("f", vec![ilit(5), ilit(10)])),
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
            Feature::DynamicTyping,
            Feature::DefaultParams,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![f, main],
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

#[test]
fn default_params_compile_and_run() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    // 1. Emit.
    let artifact = compile(&demo_module()).expect("module should compile to Rust source");

    // Sanity: the emitted source carries the prologue + padded call.
    assert!(
        artifact.source.contains("let b = if __sir::is_missing(&b)"),
        "expected default-param prologue in emitted source:\n{}",
        artifact.source
    );
    assert!(
        artifact.source.contains("f(__sir::Value::Int(5i64), __sir::missing())"),
        "expected padded full-arity call in emitted source:\n{}",
        artifact.source
    );

    // 2. Write the source to a unique temp file.
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_defparams_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_defparams_{nonce}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    // 3. Compile with rustc.  `--edition 2021` is required (raw idents +
    //    2018+ closure capture).  An absent default linker can be pointed
    //    at a working one via `SIR_TEST_RUSTC_LINKER` (e.g. `rust-lld`).
    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compile_out = cmd
        .arg(&src_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("invoke rustc");
    if !compile_out.status.success() {
        let stderr = String::from_utf8_lossy(&compile_out.stderr);
        // A missing linker is a host issue, not a codegen defect — skip.
        if stderr.contains("linker")
            && (stderr.contains("not found") || stderr.contains("No such file"))
        {
            eprintln!("skipping: no usable linker on host\n{stderr}");
            let _ = std::fs::remove_file(&src_path);
            return;
        }
        panic!(
            "emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    // 4. Run the binary and capture stdout.
    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero:\n{}",
        String::from_utf8_lossy(&run_out.stderr),
    );
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    // 5. Assert the program's observable behaviour:
    //    f(5)     → default b = a + 1 = 6
    //    f(5, 10) → supplied b = 10
    assert_eq!(
        lines,
        vec!["6", "10"],
        "unexpected program output; full stdout:\n{stdout}"
    );

    // 6. Best-effort cleanup.
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
