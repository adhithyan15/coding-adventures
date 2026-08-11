//! Execution proof for `__sys_write__` (SIR28 §2) — the console-output
//! primitive `print`/`puts` will migrate to. No frontend emits it yet
//! (that's Slices 4-6 of the SIR28 arc), so this hand-builds a minimal
//! `semantic_ir::Module` directly, one per stream/terminator/unpack_arrays
//! combination SIR28 §2.1 defines, emits Ruby, runs it with a real `ruby`
//! interpreter, and asserts stdout. Skips gracefully when no `ruby` is on
//! `PATH`.

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    CURRENT_SIR_VERSION,
};

/// Run emitted Ruby with a `ruby` interpreter if one is available; return its
/// stdout (trailing newline trimmed, matching `emit_tests.rs`'s convention),
/// or `None` to signal a skip.
fn run_ruby(source: &str) -> Option<String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "sir_ruby_syswrite_{}_{}.rb",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::File::create(&path)
        .ok()?
        .write_all(source.as_bytes())
        .ok()?;
    let out = std::process::Command::new("ruby").arg(&path).output().ok();
    let _ = std::fs::remove_file(&path);
    let out = out?;
    if !out.status.success() {
        panic!(
            "emitted ruby exited non-zero:\n{}\n--- source ---\n{source}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Some(String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"))
}

fn s() -> Span {
    Span::synthetic()
}

fn str_lit(v: &str) -> Expr {
    Expr::StrLit {
        value: v.into(),
        span: s(),
    }
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

/// Emit Ruby source for one `__sys_write__` call as the module's `main` body
/// value.
fn sys_write_source(stream: &str, terminator: &str, unpack_arrays: bool, values: Vec<Expr>) -> String {
    let mut args = vec![str_lit(stream), str_lit(terminator), bool_lit(unpack_arrays)];
    args.extend(values);
    let call = Expr::BuiltinCall {
        name: "__sys_write__".into(),
        args,
        effects: EffectSet::PURE,
        span: s(),
    };
    let module = Module {
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
            body: Block {
                stmts: vec![],
                value: call,
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }],
        globals: vec![],
        metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
        span: s(),
    };
    semantic_ir_to_ruby::compile(&module).expect("ruby emit").source
}

#[test]
fn terminator_none_writes_values_back_to_back_no_newline() {
    let rb = sys_write_source("stdout", "none", false, vec![str_lit("a"), str_lit("b")]);
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "ab"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn terminator_per_value_writes_one_newline_per_value() {
    let rb = sys_write_source("stdout", "per_value", false, vec![int_lit(1), int_lit(2)]);
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "1\n2\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn terminator_once_space_joins_and_writes_a_single_trailing_newline() {
    let rb = sys_write_source("stdout", "once", false, vec![int_lit(1), int_lit(2)]);
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "1 2\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn per_value_with_unpack_arrays_flattens_a_nested_array_one_leaf_per_line() {
    let rb = sys_write_source(
        "stdout",
        "per_value",
        true,
        vec![seq_lit(vec![int_lit(1), seq_lit(vec![int_lit(2), int_lit(3)]), int_lit(4)])],
    );
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "1\n2\n3\n4\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn per_value_without_unpack_arrays_bracket_displays_the_array() {
    let rb = sys_write_source(
        "stdout",
        "per_value",
        false,
        vec![seq_lit(vec![int_lit(1), int_lit(2)])],
    );
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "[1, 2]\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn stream_stderr_writes_to_stderr_not_stdout() {
    let rb = sys_write_source("stderr", "once", false, vec![str_lit("oops")]);
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, ""),
        None => eprintln!("skip: no ruby on PATH"),
    }
}

#[test]
fn terminator_per_value_with_zero_values_writes_a_single_blank_line() {
    let rb = sys_write_source("stdout", "per_value", false, vec![]);
    match run_ruby(&rb) {
        Some(out) => assert_eq!(out, "\n"),
        None => eprintln!("skip: no ruby on PATH"),
    }
}
