//! Execution proof for `__sys_write__` (SIR28 §2) — the console-output
//! primitive `print`/`puts` will migrate to. No frontend emits it yet
//! (that's Slices 4-6 of the SIR28 arc), so this hand-builds a minimal
//! `semantic_ir::Module` directly, one per stream/terminator/unpack_arrays
//! combination SIR28 §2.1 defines, runs it with `node`, and asserts
//! stdout/stderr. Skips gracefully when no `node` is on `PATH`.

use std::path::PathBuf;
use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    CURRENT_SIR_VERSION,
};
use semantic_ir_to_javascript::compile;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

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

/// Run a `sys_write_module` under `node`, returning (stdout, stderr), or
/// `None` to skip when no usable `node` is present.
fn run(module: &Module, tag: &str) -> Option<(String, String)> {
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let artifact = compile(module).expect("compile to javascript");
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_js_syswrite_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &artifact.source).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero for `{tag}`:\nstdout: {}\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        artifact.source,
    );
    Some((
        String::from_utf8(output.stdout).expect("utf8 stdout").replace("\r\n", "\n"),
        String::from_utf8(output.stderr).expect("utf8 stderr").replace("\r\n", "\n"),
    ))
}

#[test]
fn terminator_none_writes_values_back_to_back_no_newline() {
    let m = sys_write_module("stdout", "none", false, vec![str_lit("a"), str_lit("b")]);
    if let Some((out, _)) = run(&m, "none") {
        assert_eq!(out, "ab");
    }
}

#[test]
fn terminator_per_value_writes_one_newline_per_value() {
    let m = sys_write_module("stdout", "per_value", false, vec![int_lit(1), int_lit(2)]);
    if let Some((out, _)) = run(&m, "per_value") {
        assert_eq!(out, "1\n2\n");
    }
}

#[test]
fn terminator_once_space_joins_and_writes_a_single_trailing_newline() {
    let m = sys_write_module("stdout", "once", false, vec![int_lit(1), int_lit(2)]);
    if let Some((out, _)) = run(&m, "once") {
        assert_eq!(out, "1 2\n");
    }
}

#[test]
fn per_value_with_unpack_arrays_flattens_a_nested_array_one_leaf_per_line() {
    let m = sys_write_module(
        "stdout",
        "per_value",
        true,
        vec![seq_lit(vec![int_lit(1), seq_lit(vec![int_lit(2), int_lit(3)]), int_lit(4)])],
    );
    if let Some((out, _)) = run(&m, "unpack") {
        assert_eq!(out, "1\n2\n3\n4\n");
    }
}

#[test]
fn per_value_without_unpack_arrays_bracket_displays_the_array() {
    let m = sys_write_module("stdout", "per_value", false, vec![seq_lit(vec![int_lit(1), int_lit(2)])]);
    if let Some((out, _)) = run(&m, "no_unpack") {
        assert_eq!(out, "[1, 2]\n");
    }
}

#[test]
fn stream_stderr_writes_to_stderr_not_stdout() {
    let m = sys_write_module("stderr", "once", false, vec![str_lit("oops")]);
    if let Some((out, err)) = run(&m, "stderr") {
        assert_eq!(out, "");
        assert_eq!(err, "oops\n");
    }
}

#[test]
fn terminator_per_value_with_zero_values_writes_a_single_blank_line() {
    let m = sys_write_module("stdout", "per_value", false, vec![]);
    if let Some((out, _)) = run(&m, "empty_puts") {
        assert_eq!(out, "\n");
    }
}
