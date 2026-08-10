//! Execution proof for `__sys_write__` (SIR28 §2) — the console-output
//! primitive `print`/`puts` will migrate to. No frontend emits it yet
//! (that's Slices 4-6 of the SIR28 arc), so this hand-builds a minimal
//! `semantic_ir::Module` directly, one per stream/terminator/unpack_arrays
//! combination SIR28 §2.1 defines, emits Go, runs it with `go run`, and
//! asserts stdout/stderr. Skips gracefully when no `go` is on `PATH`.

use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    CURRENT_SIR_VERSION,
};
use semantic_ir_to_go::compile;

fn s() -> Span {
    Span::synthetic()
}

fn str_lit(v: &str) -> Expr {
    Expr::StrLit { value: v.into(), span: s() }
}

fn bool_lit(v: bool) -> Expr {
    Expr::BoolLit { value: v, span: s() }
}

fn int_lit(v: i64) -> Expr {
    Expr::IntLit { value: v, span: s() }
}

fn seq_lit(items: Vec<Expr>) -> Expr {
    Expr::SeqLit { items, span: s() }
}

fn sys_write_module(stream: &str, terminator: &str, unpack_arrays: bool, values: Vec<Expr>) -> Module {
    let mut args = vec![str_lit(stream), str_lit(terminator), bool_lit(unpack_arrays)];
    args.extend(values);
    let call = Expr::BuiltinCall {
        name: "__sys_write__".into(),
        args,
        effects: EffectSet::PURE,
        span: s(),
    };
    Module {
        name: "prog".into(),
        manifest: FeatureManifest::from_features(&[
            Feature::ConsoleIO,
            Feature::Strings,
            Feature::Sequences,
        ]),
        imports: vec![],
        exports: vec![],
        functions: vec![Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: call, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
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

/// Compile+run, returning (stdout, stderr) normalised.
fn run(module: &Module, name: &str) -> (String, String) {
    let artifact = compile(module).expect("module should compile to Go source");

    let dir = std::env::temp_dir();
    let nonce = std::process::id();
    let src_path = dir.join(format!("sir_go_syswrite_{name}_{nonce}.go"));
    std::fs::write(&src_path, &artifact.source).expect("write temp source");

    let run_out = Command::new("go").arg("run").arg(&src_path).output().expect("invoke go run");

    let stdout = String::from_utf8_lossy(&run_out.stdout).replace("\r\n", "\n");
    let stderr = String::from_utf8_lossy(&run_out.stderr).replace("\r\n", "\n");
    let _ = std::fs::remove_file(&src_path);

    if !run_out.status.success() {
        panic!(
            "emitted Go ({name}) failed to compile/run:\n--- stderr ---\n{stderr}\n--- source ---\n{}",
            artifact.source,
        );
    }
    (stdout, stderr)
}

#[test]
fn terminator_none_writes_values_back_to_back_no_newline() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let m = sys_write_module("stdout", "none", false, vec![str_lit("a"), str_lit("b")]);
    let (out, _) = run(&m, "none");
    assert_eq!(out, "ab");
}

#[test]
fn terminator_per_value_writes_one_newline_per_value() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let m = sys_write_module("stdout", "per_value", false, vec![int_lit(1), int_lit(2)]);
    let (out, _) = run(&m, "per_value");
    assert_eq!(out, "1\n2\n");
}

#[test]
fn terminator_once_space_joins_and_writes_a_single_trailing_newline() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let m = sys_write_module("stdout", "once", false, vec![int_lit(1), int_lit(2)]);
    let (out, _) = run(&m, "once");
    assert_eq!(out, "1 2\n");
}

#[test]
fn per_value_with_unpack_arrays_flattens_a_nested_array_one_leaf_per_line() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let m = sys_write_module(
        "stdout",
        "per_value",
        true,
        vec![seq_lit(vec![int_lit(1), seq_lit(vec![int_lit(2), int_lit(3)]), int_lit(4)])],
    );
    let (out, _) = run(&m, "unpack");
    assert_eq!(out, "1\n2\n3\n4\n");
}

#[test]
fn per_value_without_unpack_arrays_bracket_displays_the_array() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let m = sys_write_module("stdout", "per_value", false, vec![seq_lit(vec![int_lit(1), int_lit(2)])]);
    let (out, _) = run(&m, "no_unpack");
    assert_eq!(out, "[1, 2]\n");
}

#[test]
fn stream_stderr_writes_to_stderr_not_stdout() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let m = sys_write_module("stderr", "once", false, vec![str_lit("oops")]);
    let (out, err) = run(&m, "stderr");
    assert_eq!(out, "");
    assert_eq!(err, "oops\n");
}

#[test]
fn terminator_per_value_with_zero_values_writes_a_single_blank_line() {
    if !go_available() {
        eprintln!("skipping: go not on PATH");
        return;
    }
    let m = sys_write_module("stdout", "per_value", false, vec![]);
    let (out, _) = run(&m, "empty_puts");
    assert_eq!(out, "\n");
}
