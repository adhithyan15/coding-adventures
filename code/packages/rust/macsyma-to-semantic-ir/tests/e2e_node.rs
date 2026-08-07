//! End-to-end round-trip: Macsyma → SIR → JavaScript → `node`.
//!
//! Mirrors `wolfram-to-semantic-ir`'s own `tests/e2e_node.rs` harness
//! (itself modeled on `matlab-to-semantic-ir`'s): lower a Macsyma program
//! to SIR with this crate, validate the module, emit JavaScript with the
//! merged `semantic-ir-to-javascript` backend (a dev-dependency), write it
//! to a temp file, and **execute it with `node`**.
//!
//! # Why these tests check "runs cleanly", not a printed value
//!
//! See `wolfram-to-semantic-ir/tests/e2e_node.rs`'s own module doc comment
//! for the full explanation — the same "everything is symbolic data"
//! design applies here (`lower.rs`'s module doc comment): a Macsyma
//! program never produces a host-language computed value through this
//! pipeline, so there is no `disp`-equivalent stdout to assert on. These
//! tests prove instead that every one of these programs compiles to
//! JavaScript that `node` actually executes without throwing — genuine,
//! executable confirmation that the SIR23 codegen `semantic-ir-to-javascript`
//! implements handles the shapes this frontend emits (this grammar has no
//! pattern-matching syntax at all, per `tests/test_validator.rs`'s own
//! disclosure, so only arithmetic/assignment/control-flow/function-call
//! shapes are exercised here — no rule/replace forms, unlike Wolfram's).

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use macsyma_to_semantic_ir::compile_source;

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
    path.push(format!("macsyma_sir_e2e_{name}_{}.js", std::process::id()));
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
    run_via_node("arithmetic", "1 + 2 * 3$\n");
}

#[test]
fn a_function_definition_and_call_run_in_node() {
    if !node_available() {
        eprintln!("skipping a_function_definition_and_call_run_in_node: `node` not available");
        return;
    }
    run_via_node("function_call", "f(x) := x^2$\nf(3)$\n");
}

#[test]
fn assignment_runs_in_node() {
    if !node_available() {
        eprintln!("skipping assignment_runs_in_node: `node` not available");
        return;
    }
    run_via_node("assignment", "x : 5$\ny : x + 1$\n");
}

#[test]
fn control_flow_constructs_run_in_node() {
    if !node_available() {
        eprintln!("skipping control_flow_constructs_run_in_node: `node` not available");
        return;
    }
    run_via_node(
        "control_flow",
        "if x > 0 then 1 else -1$\nwhile x do x : x - 1$\nfor i in [1, 2, 3] do i$\n\
         block([total : 0], total)$\nreturn(5)$\n",
    );
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
        "f(x) := x^2$\ng(x) := f(x) + 1$\nresult : g(3)$\n[result, f(2)]$\n",
    );
}
