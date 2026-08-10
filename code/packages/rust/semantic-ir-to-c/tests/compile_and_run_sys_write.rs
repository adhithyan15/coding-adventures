//! Execution proof for `__sys_write__` (SIR28 §2) — the console-output
//! primitive `print`/`puts` will migrate to. No frontend emits it yet
//! (that's Slices 4-6 of the SIR28 arc), so this hand-builds a minimal
//! `semantic_ir::Module` directly, one per stream/terminator/unpack_arrays
//! combination SIR28 §2.1 defines, emits C, compiles with a real cc, runs,
//! and asserts stdout. Skips gracefully when no `cc` is present.

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    CURRENT_SIR_VERSION,
};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

fn find_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    ["cc", "clang", "gcc"]
        .iter()
        .find(|c| {
            Command::new(c)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .map(|s| s.to_string())
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn compile_and_run(cc: &str, module: &Module, name: &str) -> String {
    let artifact = semantic_ir_to_c::compile(module).expect("C backend compile");

    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stem = format!("sirc_syswrite_{name}_{}_{n}", std::process::id());
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));

    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");

    let out = Command::new(cc)
        .args(["-std=c99", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")
        .output()
        .expect("spawn C compiler");
    assert!(
        out.status.success(),
        "compile failed for `{name}`:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        artifact.source
    );

    let run = Command::new(&exe).output().expect("run emitted program");
    assert!(
        run.status.success(),
        "run failed for `{name}` (exit {:?}): {}",
        run.status.code(),
        String::from_utf8_lossy(&run.stderr)
    );

    let _ = std::fs::remove_file(&cpath);
    let _ = std::fs::remove_file(&exe);

    String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n")
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

/// One `__sys_write__` call as the module's `main` body value.
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
    }
}

#[test]
fn terminator_none_writes_values_back_to_back_no_newline() {
    let Some(cc) = find_cc() else {
        eprintln!("SKIP: no C compiler found");
        return;
    };
    let m = sys_write_module("stdout", "none", false, vec![str_lit("a"), str_lit("b")]);
    assert_eq!(compile_and_run(&cc, &m, "none"), "ab");
}

#[test]
fn terminator_per_value_writes_one_newline_per_value() {
    let Some(cc) = find_cc() else {
        eprintln!("SKIP: no C compiler found");
        return;
    };
    let m = sys_write_module("stdout", "per_value", false, vec![int_lit(1), int_lit(2)]);
    assert_eq!(compile_and_run(&cc, &m, "per_value"), "1\n2\n");
}

#[test]
fn terminator_once_space_joins_and_writes_a_single_trailing_newline() {
    let Some(cc) = find_cc() else {
        eprintln!("SKIP: no C compiler found");
        return;
    };
    let m = sys_write_module("stdout", "once", false, vec![int_lit(1), int_lit(2)]);
    assert_eq!(compile_and_run(&cc, &m, "once"), "1 2\n");
}

#[test]
fn per_value_with_unpack_arrays_flattens_a_nested_array_one_leaf_per_line() {
    let Some(cc) = find_cc() else {
        eprintln!("SKIP: no C compiler found");
        return;
    };
    let m = sys_write_module(
        "stdout",
        "per_value",
        true,
        vec![seq_lit(vec![int_lit(1), seq_lit(vec![int_lit(2), int_lit(3)]), int_lit(4)])],
    );
    assert_eq!(
        compile_and_run(&cc, &m, "unpack"),
        "1\n2\n3\n4\n"
    );
}

#[test]
fn per_value_without_unpack_arrays_bracket_displays_the_array() {
    let Some(cc) = find_cc() else {
        eprintln!("SKIP: no C compiler found");
        return;
    };
    let m = sys_write_module(
        "stdout",
        "per_value",
        false,
        vec![seq_lit(vec![int_lit(1), int_lit(2)])],
    );
    assert_eq!(compile_and_run(&cc, &m, "no_unpack"), "[1, 2]\n");
}

#[test]
fn stream_stderr_writes_to_stderr_not_stdout() {
    let Some(cc) = find_cc() else {
        eprintln!("SKIP: no C compiler found");
        return;
    };
    let m = sys_write_module("stderr", "once", false, vec![str_lit("oops")]);
    // stdout must be empty; the process must still exit 0 (this is a
    // faithful stderr write, not an error path).
    assert_eq!(compile_and_run(&cc, &m, "stderr"), "");
}

#[test]
fn terminator_per_value_with_zero_values_writes_a_single_blank_line() {
    let Some(cc) = find_cc() else {
        eprintln!("SKIP: no C compiler found");
        return;
    };
    let m = sys_write_module("stdout", "per_value", false, vec![]);
    assert_eq!(compile_and_run(&cc, &m, "empty_puts"), "\n");
}

#[test]
fn compound_value_argument_still_reads_the_literal_stream_and_terminator() {
    // A value argument that is itself an `If` is never `is_simple` (see
    // `emit.rs::is_simple`), so the WHOLE `__sys_write__` call is compound
    // and lands in `emit_compound_call`'s dedicated arm rather than
    // `emit_builtin_simple`'s — a separate code path from every other test
    // in this file, which only exercises the simple-call arm. Proves the
    // compile-time `stream`/`terminator`/`unpack_arrays` literals still
    // reach `_sir_write` correctly (as int constants, not through a hoisted
    // temp) even when a trailing value needs statement hoisting.
    let Some(cc) = find_cc() else {
        eprintln!("SKIP: no C compiler found");
        return;
    };
    let cond = Expr::If {
        cond: Box::new(bool_lit(true)),
        then_branch: Box::new(Block {
            stmts: vec![],
            value: str_lit("yes"),
            span: s(),
        }),
        else_branch: Box::new(Block {
            stmts: vec![],
            value: str_lit("no"),
            span: s(),
        }),
        span: s(),
    };
    let m = sys_write_module("stdout", "once", false, vec![cond]);
    assert_eq!(compile_and_run(&cc, &m, "compound"), "yes\n");
}
