//! End-to-end round-trip: Reduce → SIR → JavaScript → `node`.
//!
//! Mirrors `derive-to-semantic-ir`'s (and `wolfram-to-semantic-ir`'s /
//! `macsyma-to-semantic-ir`'s) own `tests/e2e_node.rs` harnesses: lower a
//! Reduce program to SIR with this crate, validate the module, emit
//! JavaScript with the merged `semantic-ir-to-javascript` backend (a
//! dev-dependency), write it to a temp file, and **execute it with
//! `node`**.
//!
//! # Why these tests check "runs cleanly", not a printed value
//!
//! See `wolfram-to-semantic-ir/tests/e2e_node.rs`'s own module doc
//! comment for the full explanation — the same "everything is symbolic
//! data" design applies here (`lower.rs`'s module doc comment): a Reduce
//! program never produces a host-language computed value through this
//! pipeline, so there is no `disp`-equivalent stdout to assert on. These
//! tests prove instead that every one of these programs compiles to
//! JavaScript that `node` actually executes without throwing — genuine,
//! executable confirmation that the SIR23 codegen `semantic-ir-to-
//! javascript` implements handles the shapes this frontend emits,
//! including the constructs with no `symbolic-vm` evaluation handler at
//! all (`<< ... >>`/`CompoundExpression`, a non-folding `.`/`Cons`, and
//! the list accessors `first`/`rest`/`part`/`append`/`reverse`) — see
//! `tests/test_validator.rs`'s own module doc comment for why that gap is
//! moot for pure data construction/codegen.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use reduce_to_semantic_ir::compile_source;

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
    let artifact = semantic_ir_to_javascript::compile(&module).expect("backend emit should succeed");

    let mut path = std::env::temp_dir();
    path.push(format!("reduce_sir_e2e_{name}_{}.js", std::process::id()));
    // See `matlab-to-semantic-ir`'s / `derive-to-semantic-ir`'s identical
    // helper for why `create_new` (not `std::fs::write`) is used here: it
    // fails loudly on an existing path (including a symlink) instead of
    // silently following/truncating through it. Each test uses a unique
    // name+PID, so this should never legitimately collide.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp js (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes()).expect("write temp js");
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
    run_via_node("arithmetic", "1 + 2 * 3;\n");
}

#[test]
fn a_procedure_definition_and_call_run_in_node() {
    if !node_available() {
        eprintln!("skipping a_procedure_definition_and_call_run_in_node: `node` not available");
        return;
    }
    run_via_node("procedure_call", "h(x) := x^2;\nh(3);\n");
}

#[test]
fn assignment_runs_in_node() {
    if !node_available() {
        eprintln!("skipping assignment_runs_in_node: `node` not available");
        return;
    }
    run_via_node("assignment", "x := 5;\ny := x + 1;\n");
}

#[test]
fn list_accessor_calls_run_in_node() {
    if !node_available() {
        eprintln!("skipping list_accessor_calls_run_in_node: `node` not available");
        return;
    }
    run_via_node(
        "list_accessors",
        "first({1, 2, 3});\nrest({1, 2, 3});\nappend({1}, {2});\nreverse({1, 2});\npart({1, 2}, 1);\n",
    );
}

#[test]
fn lists_and_cons_run_in_node() {
    if !node_available() {
        eprintln!("skipping lists_and_cons_run_in_node: `node` not available");
        return;
    }
    run_via_node("lists_cons", "{1, 2, 3};\na . {b, c};\na . b;\n");
}

#[test]
fn if_expression_runs_in_node() {
    if !node_available() {
        eprintln!("skipping if_expression_runs_in_node: `node` not available");
        return;
    }
    run_via_node("if_expr", "if x > 0 then 1 else -1;\nif x then y;\n");
}

#[test]
fn group_statement_runs_in_node() {
    if !node_available() {
        eprintln!("skipping group_statement_runs_in_node: `node` not available");
        return;
    }
    run_via_node("group_expr", "<< x := 1; x + 1 >>;\n");
}

#[test]
fn a_complex_multi_statement_program_runs_in_node() {
    if !node_available() {
        eprintln!("skipping a_complex_multi_statement_program_runs_in_node: `node` not available");
        return;
    }
    run_via_node(
        "complex",
        "h(x) := x^2;\ng(x) := h(x) + 1;\nresult := g(3);\n{result, h(2)};\nif result > 0 then result else 0;\n<< result := result + 1; result >>;\n",
    );
}
