//! End-to-end proof for the source-language display convention (SIR
//! display-convention spec) in the Rust backend.
//!
//! A translated **Ruby** program's `puts true` must print `true` — not the
//! Twig/Lisp `#t`.  The emitter selects the convention from the module's
//! `source_language` metadata and substitutes it into the runtime's
//! `SIR_DISPLAY_RUBY` constant; `format` then renders booleans accordingly.
//!
//! This test hand-builds the program
//!
//!     puts true
//!     puts false
//!
//! twice — once tagged `source_language = "ruby"`, once `"twig"` — emits Rust,
//! compiles it with `rustc`, runs it, and asserts:
//!   * Ruby  → `true\nfalse\n`   (Ruby-faithful)
//!   * Twig  → `#t\n#f\n`        (Lisp default, unchanged)
//!
//! Gates on `rustc` and degrades gracefully when the host linker is missing.

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, Function, FeatureManifest, Metadata, Module, Span, Stmt,
};
use semantic_ir_to_rust::compile;

fn s() -> Span {
    Span::synthetic()
}

fn blit(v: bool) -> Expr {
    Expr::BoolLit { value: v, span: s() }
}

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

/// `puts true; puts false`, tagged with the given `source_language`.
fn bool_module(source_language: &str) -> Module {
    let stmts = vec![puts_stmt(vec![blit(true)]), puts_stmt(vec![blit(false)])];
    Module {
        name: "display_bool".into(),
        manifest: FeatureManifest::from_features(&[]),
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
            .with_source_language(source_language)
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

fn compile_run(module: &Module, tag: &str) -> Option<String> {
    let artifact = compile(module).expect("module should compile to Rust source");
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_disp_{tag}_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_disp_{tag}_{nonce}{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

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
    let stdout = String::from_utf8_lossy(&run_out.stdout).into_owned();
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);
    assert!(run_out.status.success(), "compiled binary exited non-zero");
    Some(stdout.replace("\r\n", "\n"))
}

#[test]
fn ruby_source_prints_true_false() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let Some(out) = compile_run(&bool_module("ruby"), "ruby") else { return };
    assert_eq!(out, "true\nfalse\n", "Ruby source must render booleans as true/false");
}

#[test]
fn twig_source_keeps_lisp_booleans() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }
    let Some(out) = compile_run(&bool_module("twig"), "twig") else { return };
    assert_eq!(out, "#t\n#f\n", "non-Ruby source keeps the default Lisp #t/#f");
}
