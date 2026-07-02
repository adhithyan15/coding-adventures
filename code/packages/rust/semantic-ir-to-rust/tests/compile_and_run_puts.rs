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
    Block, Effect, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope,
    Span, Stmt,
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

fn var(name: &str) -> Expr {
    Expr::VarRef { name: name.into(), scope: Scope::Local, span: s() }
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

/// Build a module equivalent to the Ruby program
///
///     a = [nil]
///     a[0] = a
///     puts a
///
/// i.e. a *self-referential* array.  Real Ruby is cycle-aware: `puts a`
/// prints `[...]` and terminates.  Before the cycle guard the emitted
/// `puts_one` recursed per-element with no bound, so this program overflowed
/// the native stack and aborted (CWE-674, uncontrolled recursion — a DoS).
fn cyclic_module() -> Module {
    let stmts = vec![
        Stmt::LetBinding {
            name: "a".into(),
            sir_type: None,
            value: Expr::SeqLit { items: vec![Expr::NilLit { span: s() }], span: s() },
            span: s(),
        },
        Stmt::SeqSet { seq: var("a"), index: ilit(0), value: var("a"), span: s() },
        puts_stmt(vec![var("a")]),
    ];

    Module {
        name: "puts_cyclic".into(),
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

/// Compile `module` to Rust, build with `rustc`, run the binary, and return
/// its normalised (CRLF→LF) stdout.  Returns `None` when the toolchain or a
/// usable linker is unavailable (the caller then skips).  Panics on a genuine
/// compile/run failure so a real regression is loud.
fn compile_run(module: &Module, tag: &str) -> Option<String> {
    let artifact = compile(module).expect("module should compile to Rust source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_puts_{tag}_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_puts_{tag}_{nonce}{}",
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
            return None;
        }
        panic!(
            "emitted Rust failed to compile:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }

    let run_out = Command::new(&bin_path).output().expect("run compiled binary");
    let stderr = String::from_utf8_lossy(&run_out.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&run_out.stdout).into_owned();
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);

    // A stack-overflow abort (the pre-fix cyclic behaviour) exits non-zero;
    // surface it as a test failure with the captured stderr.
    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero (should terminate cleanly):\n{stderr}",
    );
    Some(stdout.replace("\r\n", "\n"))
}

#[test]
fn puts_compiles_and_matches_ruby_output() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let Some(out) = compile_run(&demo_module(), "demo") else { return };
    // Exact byte-for-byte match against the Ruby reference output.
    assert_eq!(
        out, "hello\n\n1\n2\n3\n",
        "unexpected puts output; full stdout (escaped): {out:?}"
    );
}

/// Regression (security, CWE-674): `puts` on a self-referential array must
/// TERMINATE — printing a `[...]` cycle placeholder like Ruby — rather than
/// recursing until the native stack overflows and the process aborts.
#[test]
fn puts_cyclic_array_terminates() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let Some(out) = compile_run(&cyclic_module(), "cyclic") else { return };
    assert_eq!(
        out, "[...]\n",
        "unexpected cyclic puts output; full stdout (escaped): {out:?}"
    );
}
