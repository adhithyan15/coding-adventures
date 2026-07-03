//! End-to-end proof for POLYMORPHIC `+` / `*` (Ruby semantics) in the Rust
//! backend (sir-polymorphic-operators PO5).
//!
//! Ruby's `+` and `*` are overloaded by receiver type, and before this change
//! the Rust runtime's `plus`/`times` were numeric-only — `"a" + "b"` called
//! `as_i64("a")` and produced integer garbage.  A *shape* assertion is not
//! enough here; we must prove the emitted Rust actually produces the exact
//! bytes Ruby would.  Each case hand-builds a SIR module of the form
//! `puts (<lhs> <op> <rhs>)`, emits Rust, compiles it with `rustc`, runs the
//! binary, and asserts stdout against the Ruby reference:
//!
//! | expression      | output        | printed via |
//! |-----------------|---------------|-------------|
//! | `"a" + "b"`     | `ab`          | `puts`      |
//! | `"ab" * 3`      | `ababab`      | `puts`      |
//! | `[1] + [2]`     | `[1, 2]`      | `print`     |
//! | `[0] * 3`       | `[0, 0, 0]`   | `print`     |
//! | `[1, 2] * ", "` | `1, 2`        | `puts`      |
//! | `1 + 2`         | `3` (regr.)   | `puts`      |
//! | `2 * 3`         | `6` (regr.)   | `puts`      |
//!
//! The array-VALUED cases use `print` (which renders through `__sir::format`,
//! giving the bracketed `[1, 2]` display) rather than `puts` (which flattens a
//! Seq element-per-line), so the assertion pins the array display form exactly
//! as the spec's reference table lists it.
//!
//! Gates on `rustc` being available and degrades gracefully when the host
//! linker is missing (mirrors `compile_and_run_puts.rs`).

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

fn seq(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

/// A binary builtin call `<name>(lhs, rhs)` — `+` lowers to `__sir::plus`,
/// `*` to `__sir::times`.
fn binop(name: &str, lhs: Expr, rhs: Expr) -> Expr {
    Expr::BuiltinCall {
        name: name.into(),
        args: vec![lhs, rhs],
        effects: EffectSet::PURE,
        span: s(),
    }
}

/// `<builtin>(expr)` as an effectful statement (MayPrint, matching the
/// frontend) — `builtin` is `"puts"` or `"print"`.
fn print_like_stmt(builtin: &str, expr: Expr) -> Stmt {
    Stmt::ExprStmt {
        expr: Expr::BuiltinCall {
            name: builtin.into(),
            args: vec![expr],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        },
        span: s(),
    }
}

/// Build a module whose `main` runs `<builtin> (<expr>)`.
fn print_module(builtin: &str, tag: &str, expr: Expr) -> Module {
    Module {
        name: format!("polyops_{tag}"),
        manifest: FeatureManifest::from_features(&[Feature::Sequences, Feature::Strings]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![print_like_stmt(builtin, expr)],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
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
    let src_path = dir.join(format!("sir_polyops_{tag}_{nonce}.rs"));
    let bin_path = dir.join(format!(
        "sir_polyops_{tag}_{nonce}{}",
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
    let stderr = String::from_utf8_lossy(&run_out.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&run_out.stdout).into_owned();
    let _ = std::fs::remove_file(&src_path);
    let _ = std::fs::remove_file(&bin_path);

    assert!(
        run_out.status.success(),
        "compiled binary exited non-zero (should terminate cleanly):\n{stderr}",
    );
    Some(stdout.replace("\r\n", "\n"))
}

/// Compile+run a `<builtin> (<expr>)` module and assert its stdout, unless the
/// toolchain is unavailable (in which case the whole suite is skipped up front
/// by the caller).
fn assert_output(builtin: &str, tag: &str, expr: Expr, expected: &str) {
    let Some(out) = compile_run(&print_module(builtin, tag, expr), tag) else {
        return;
    };
    assert_eq!(
        out, expected,
        "unexpected output for {tag}; full stdout (escaped): {out:?}"
    );
}

/// The whole polymorphic-operator suite in one test so `rustc` is invoked once
/// per case only when the toolchain is actually present.
#[test]
fn polymorphic_plus_times_match_ruby() {
    if !rustc_available() {
        eprintln!("skipping: rustc not on PATH");
        return;
    }

    // ── `+` string concatenation: "a" + "b" → "ab" ──────────────────
    assert_output("puts", "str_concat", binop("+", slit("a"), slit("b")), "ab\n");

    // ── `*` string repeat: "ab" * 3 → "ababab" ──────────────────────
    assert_output("puts", "str_repeat", binop("*", slit("ab"), ilit(3)), "ababab\n");

    // ── `+` array concat: [1] + [2] → [1, 2] ────────────────────────
    // Printed via `print` so the bracketed Seq display form is asserted.
    assert_output(
        "print",
        "arr_concat",
        binop("+", seq(vec![ilit(1)]), seq(vec![ilit(2)])),
        "[1, 2]\n",
    );

    // ── `*` array repeat: [0] * 3 → [0, 0, 0] ───────────────────────
    assert_output(
        "print",
        "arr_repeat",
        binop("*", seq(vec![ilit(0)]), ilit(3)),
        "[0, 0, 0]\n",
    );

    // ── `*` array join: [1, 2] * ", " → "1, 2" (a String) ───────────
    assert_output(
        "puts",
        "arr_join",
        binop("*", seq(vec![ilit(1), ilit(2)]), slit(", ")),
        "1, 2\n",
    );

    // ── regression: numeric `+` unchanged: 1 + 2 → 3 ───────────────
    assert_output("puts", "num_plus", binop("+", ilit(1), ilit(2)), "3\n");

    // ── regression: numeric `*` unchanged: 2 * 3 → 6 ───────────────
    assert_output("puts", "num_times", binop("*", ilit(2), ilit(3)), "6\n");
}
