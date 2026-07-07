//! End-to-end proof for the source-language display convention (SIR
//! display-convention spec) in the Go backend.
//!
//! A translated **Ruby** program's `puts true` must print `true` — not the
//! Twig/Lisp `#t`.  The emitter selects the convention from the module's
//! `source_language` metadata and substitutes it into the runtime's
//! `_sir_display_ruby` constant; `_sir_format` then renders booleans
//! accordingly.
//!
//! Builds the program `puts true; puts false` twice — once tagged
//! `source_language = "ruby"`, once `"twig"` — runs it with `go run`, and
//! asserts:
//!   * Ruby → `true\nfalse\n`   (Ruby-faithful)
//!   * Twig → `#t\n#f\n`        (Lisp default, unchanged)
//!
//! Gates on `go` being available; logs a skip when the toolchain is absent.

use std::process::Command;

use semantic_ir::{
    Block, Effect, EffectSet, Expr, FeatureManifest, Function, Metadata, Module, Span, Stmt,
};
use semantic_ir_to_go::compile;

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

fn go_available() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run(module: &Module, tag: &str) -> Option<String> {
    let artifact = compile(module).expect("module should compile to Go source");
    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_disp_{tag}_{nonce}.go"));
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
    let _ = std::fs::remove_file(&src_path);
    Some(String::from_utf8_lossy(&run_out.stdout).replace("\r\n", "\n"))
}

#[test]
fn ruby_source_prints_true_false() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let Some(out) = run(&bool_module("ruby"), "ruby") else { return };
    assert_eq!(out, "true\nfalse\n", "Ruby source must render booleans as true/false");
}

#[test]
fn twig_source_keeps_lisp_booleans() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let Some(out) = run(&bool_module("twig"), "twig") else { return };
    assert_eq!(out, "#t\n#f\n", "non-Ruby source keeps the default Lisp #t/#f");
}
