//! Execution proof for `__sys_write__` (SIR28 §2) — the console-output
//! primitive `print`/`puts` will migrate to. No frontend emits it yet
//! (that's Slices 4-6 of the SIR28 arc), so this hand-builds a minimal
//! `semantic_ir::Module` directly, one per stream/terminator/unpack_arrays
//! combination SIR28 §2.1 defines, runs it with `node`, and asserts
//! stdout/stderr.
//!
//! As with every other node proof in this crate (see `run_with_node.rs`'s
//! module doc-comment), the TypeScript backend emits genuine TypeScript
//! — an `import * as __Sir from "@coding-adventures/sir-runtime-core"` bare
//! `node` cannot resolve or parse — so we swap that import for an inline
//! `__Sir` stub and strip the small, fixed set of type annotations the
//! emitter adds. The stub's `write`/`writeOne` TRANSCRIBE the real
//! `runtime.ts` logic (not a fake shape check), so this proof genuinely
//! exercises the terminator/stream/unpack_arrays behavior, not just that
//! the emitted call site is present. Skips gracefully when no `node` is on
//! `PATH`.

use std::path::PathBuf;
use std::process::Command;

use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    CURRENT_SIR_VERSION,
};
use semantic_ir_to_typescript::compile;

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

/// `__Sir` stub carrying `toDisplay` plus a faithful transcription of
/// `runtime.ts`'s `writeOne`/`write` (SIR28 §2.1): `"none"` writes every
/// value back-to-back with no newline, `"per_value"` writes one newline per
/// value (honoring `unpackArrays`, recursing into nested arrays), `"once"`
/// space-joins every value with a single trailing newline. `stream`
/// dispatches to `process.stdout`/`process.stderr`.
const SIR_SYS_WRITE_STUB: &str = r#"const __Sir = {
  // Transcribes the real `sir-runtime-core`'s `toDisplay` (`values.ts`):
  // `nil` for null, else `String(v)` — including for arrays, which have NO
  // bracket-formatting special case there (`[1, 2].toString()` = `"1,2"`).
  toDisplay: (v) => (v === null ? "nil" : String(v)),
  writeOne: (out, v, unpackArrays, seen) => {
    if (unpackArrays && Array.isArray(v)) {
      if (seen.has(v)) { out.write("[...]\n"); return; }
      seen.add(v);
      for (const item of v) { __Sir.writeOne(out, item, unpackArrays, seen); }
      seen.delete(v);
      return;
    }
    out.write(__Sir.toDisplay(v) + "\n");
  },
  write: (stream, terminator, unpackArrays, ...values) => {
    const out = stream === "stderr" ? process.stderr : process.stdout;
    if (terminator === "per_value") {
      if (values.length === 0) { out.write("\n"); return null; }
      const seen = new Set();
      for (const v of values) { __Sir.writeOne(out, v, unpackArrays, seen); }
      return null;
    }
    if (terminator === "once") {
      out.write(values.map((v) => __Sir.toDisplay(v)).join(" ") + "\n");
      return null;
    }
    for (const v of values) { out.write(__Sir.toDisplay(v)); }
    return null;
  },
};
"#;

/// Turn emitted TypeScript into runnable JavaScript. See the module
/// doc-comment and `run_with_node.rs`'s `ts_to_runnable_js` for why this
/// (rewrite the runtime import, strip the fixed set of type annotations
/// the emitter adds) is faithful to what the backend actually produced.
fn ts_to_runnable_js(ts: &str) -> String {
    let mut js = ts.to_string();
    js = js.replace(
        "import * as __Sir from \"@coding-adventures/sir-runtime-core\";\n",
        SIR_SYS_WRITE_STUB,
    );
    js = js.replace(" as { [k: string]: __Sir.Val }", "");
    js = js.replace(": __Sir.Val[]", "");
    js = js.replace(": __Sir.Val", "");
    js
}

/// Compile a `sys_write_module`, transform to JS, run under `node`, and
/// return `(stdout, stderr)`, or `None` to skip when no usable `node` is
/// present.
fn run(module: &Module, tag: &str) -> Option<(String, String)> {
    if !node_available() {
        eprintln!("note: `node` unavailable — skipping execution for `{tag}`");
        return None;
    }
    let artifact = compile(module).expect("compile to typescript");
    let js = ts_to_runnable_js(&artifact.source);
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!("sir_ts_syswrite_{}_{}.js", tag, std::process::id()));
    std::fs::write(&path, &js).expect("write temp js");
    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);
    assert!(
        output.status.success(),
        "node exited non-zero for `{tag}`:\nstdout: {}\nstderr: {}\nsource:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        js,
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
fn per_value_without_unpack_arrays_displays_the_array_as_one_value() {
    // `toDisplay`'s array fallback is plain `String(v)` (`values.ts`) — no
    // bracket-formatting special case, unlike the JS backend's own `format`.
    let m = sys_write_module("stdout", "per_value", false, vec![seq_lit(vec![int_lit(1), int_lit(2)])]);
    if let Some((out, _)) = run(&m, "no_unpack") {
        assert_eq!(out, "1,2\n");
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
