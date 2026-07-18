//! End-to-end round-trip: Wolfram → SIR → JavaScript → `node`.
//!
//! Mirrors `matlab-to-semantic-ir`'s own `tests/e2e_node.rs` harness: lower
//! a Wolfram program to SIR with this crate, validate the module, emit
//! JavaScript with the merged `semantic-ir-to-javascript` backend (a
//! dev-dependency), write it to a temp file, and **execute it with
//! `node`**.
//!
//! # Why these tests check "runs cleanly", not a printed value
//!
//! Under this frontend's "everything is data" design (see `lower.rs`'s
//! module doc comment), a Wolfram program never produces a host-language
//! computed value — `1 + 2` lowers to an *unevaluated* `SymApply` term
//! tree (`apply(sym("Add"), [int(1), int(2)])`), and the compiled JS's
//! `main()` evaluates that expression only for its (non-existent) side
//! effects before returning `null`; nothing is ever printed. Evaluating
//! symbolic data — actually reducing `Add(1, 2)` to `3`, or running the
//! pattern matcher — is `sir-runtime-symbolic`'s own job (a runtime
//! *library*, invoked separately, not something this compiled-`main`
//! shape triggers automatically), not something the compiled entry point
//! does on its own. So there is no `disp`-equivalent output to assert on
//! here the way `matlab-to-semantic-ir`'s tests assert `stdout`.
//!
//! What these tests DO prove, and what matters most given this crate's
//! history: every one of these programs compiles to JavaScript that
//! `node` actually executes without throwing — a genuine, executable
//! proof (not just `check_module` returning no errors) that the SIR23
//! codegen `semantic-ir-to-javascript` implements for `SymApply`/
//! `SymPatternBlank`/`SymPatternNamed`/`SymRule`/`SymReplaceAll` handles
//! the full range of shapes this frontend emits: plain arithmetic,
//! patterns, rules, and both `/.`/`//.` replacement forms.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use wolfram_to_semantic_ir::compile_source;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_via_node(name: &str, src: &str) {
    let module = compile_source(src, "prog").expect("lowering should succeed");
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );
    let artifact =
        semantic_ir_to_javascript::compile(&module).expect("backend emit should succeed");

    let mut path = std::env::temp_dir();
    path.push(format!("wolfram_sir_e2e_{name}_{}.js", std::process::id()));
    // See `matlab-to-semantic-ir`'s identical helper for why `create_new`
    // (not `std::fs::write`) is used here: it fails loudly on an existing
    // path (including a symlink) instead of silently following/truncating
    // through it. Each test uses a unique name+PID, so this should never
    // legitimately collide.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp js (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes())
        .expect("write temp js");
    drop(file);

    let output = Command::new("node").arg(&path).output().expect("spawn node");
    let _ = std::fs::remove_file(&path);

    assert!(
        output.status.success(),
        "node failed for {name}: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn plain_arithmetic_runs_in_node() {
    if !node_available() {
        eprintln!("skipping plain_arithmetic_runs_in_node: `node` not available");
        return;
    }
    run_via_node("arithmetic", "1 + 2 * 3\n");
}

#[test]
fn a_pattern_and_function_call_run_in_node() {
    if !node_available() {
        eprintln!("skipping a_pattern_and_function_call_run_in_node: `node` not available");
        return;
    }
    run_via_node("pattern_call", "f[x_] := x^2\nf[3]\n");
}

#[test]
fn a_replaceall_program_runs_in_node() {
    if !node_available() {
        eprintln!("skipping a_replaceall_program_runs_in_node: `node` not available");
        return;
    }
    run_via_node("replaceall", "{1, 2, 3} /. a_ -> a + 1\n");
}

#[test]
fn a_replacerepeated_program_runs_in_node() {
    if !node_available() {
        eprintln!("skipping a_replacerepeated_program_runs_in_node: `node` not available");
        return;
    }
    run_via_node("replacerepeated", "x //. a -> b\n");
}

#[test]
fn a_complex_multi_statement_program_runs_in_node() {
    if !node_available() {
        eprintln!(
            "skipping a_complex_multi_statement_program_runs_in_node: `node` not available"
        );
        return;
    }
    run_via_node(
        "complex",
        "f[x_] := x^2\ng[x_] := f[x] + 1\nresult = g[3] /. Null -> 0\n{result, f[2]}\n",
    );
}
