//! End-to-end proof for the `puts` builtin (Ruby semantics) in the Rust
//! backend.
//!
//! Ruby's `puts` is the most common output method, and its semantics are
//! subtle enough that a *shape* assertion in a unit test is not enough — we
//! must prove the emitted Rust actually produces the exact byte stream Ruby
//! would.  This test hand-builds a SIR module equivalent to the Ruby program
//!
//!     puts "hello"
//!     puts
//!     puts [1, 2, 3]
//!
//! emits Rust, compiles it with `rustc`, runs the binary, and asserts stdout
//! is **exactly** `hello\n\n1\n2\n3\n` — the Ruby reference output.
//!
//! Gates on `rustc` being available and degrades gracefully when the host
//! linker is missing (mirrors `compile_and_run_loops.rs`).

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
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

/// `puts(args…)` as an effectful statement (MayPrint, matching the frontend).
fn puts_stmt(args: Vec<Expr>) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: "puts".into(),
            args,
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// Build a module whose `main` runs `puts "hello"; puts; puts [1,2,3]`.
fn demo_module() -> Module {
    let stmts = vec![
        puts_stmt(vec![slit("hello")]),
        puts_stmt(vec![]),
        puts_stmt(vec![Expr::SeqLit { items: vec![ilit(1), ilit(2), ilit(3)], span: s() }]),
    ];

    Module {
        name: "puts_demo".into(),
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

fn rustc_available() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn puts_compiles_and_matches_ruby_output() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    let artifact = compile(&demo_module()).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_puts_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_puts_{nonce}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    // `--edition 2021` matches the emitted runtime; an optional linker
    // override lets hosts without the default linker point at `rust-lld`.
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

    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero:\n{}",
        String::from_utf8_lossy(&run_out.stderr),
    );

    // Exact byte-for-byte match against the Ruby reference output.
    let stdout = String::from_utf8_lossy(&run_out.stdout);
    let normalised = stdout.replace("\r\n", "\n");
    assert_eq!(
        normalised, "hello\n\n1\n2\n3\n",
        "unexpected puts output; full stdout (escaped): {stdout:?}"
    );

    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
}
