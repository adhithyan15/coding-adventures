//! End-to-end round-trip: MATLAB → SIR → JavaScript → `node`.
//!
//! Mirrors `javascript-to-semantic-ir`'s own `e2e_node.rs` harness exactly:
//! lower a MATLAB program to SIR with this crate, validate the module, emit
//! JavaScript with the merged `semantic-ir-to-javascript` backend (a
//! dev-dependency), write it to a temp file, and **execute it with
//! `node`**, asserting the printed output.
//!
//! As `test_validator.rs`'s module doc comment explains, this crate's
//! "scalar fast path" only recognises *purely literal* arithmetic as
//! provably scalar — any variable-involving arithmetic takes the
//! `ElementwiseOp`/`MatMul` path, which the JS backend does not implement
//! codegen for yet. So, for now, only a purely-literal MATLAB program can
//! make it all the way through this pipeline; that is exactly what this
//! test proves, gated on `node` availability like its JS counterpart.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use matlab_to_semantic_ir::compile_source;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_via_node(name: &str, src: &str) -> String {
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
    path.push(format!("matlab_sir_e2e_{name}_{}.js", std::process::id()));
    // `create_new` fails if the path already exists (including as a
    // symlink) instead of following it -- unlike `std::fs::write`, which
    // truncates through a symlink at this predictable, shared-temp-dir
    // path. Each test uses a unique name+PID, so this should never
    // legitimately collide; if it does, failing loudly is correct for a
    // test rather than silently overwriting whatever the path pointed to.
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
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn a_function_over_pure_literals_runs_in_node() {
    if !node_available() {
        eprintln!("skipping a_function_over_pure_literals_runs_in_node: `node` not available");
        return;
    }
    let out = run_via_node(
        "literal_function",
        "function r = seven()\n  r = 3 + 4;\nend\ndisp(seven());\n",
    );
    assert_eq!(out, "7");
}

#[test]
fn nested_literal_arithmetic_runs_in_node() {
    if !node_available() {
        eprintln!("skipping nested_literal_arithmetic_runs_in_node: `node` not available");
        return;
    }
    // `2 * 3 + 4 * 5` -- both multiplications are literal*literal (the
    // scalar fast path), and the outer addition is then built from two
    // already-known-scalar BuiltinCall results, so the whole expression
    // stays on the plain-BuiltinCall path transitively.
    let out = run_via_node(
        "nested_literal_arithmetic",
        "disp(2 * 3 + 4 * 5);\n",
    );
    assert_eq!(out, "26");
}

#[test]
fn a_call_before_its_textual_definition_runs_in_node() {
    if !node_available() {
        eprintln!(
            "skipping a_call_before_its_textual_definition_runs_in_node: `node` not available"
        );
        return;
    }
    // Proves the two-pass function-name collection resolves a forward
    // reference correctly all the way through actual execution, not just
    // at the lowered-Module-shape level. `double_seven`'s own body stays
    // on the pure-literal path (composing its result with a further `*`
    // outside the function -- e.g. `2 * double_seven()` -- would push the
    // whole expression onto the ElementwiseOp path instead, since a
    // DirectCall result is never provably scalar; see this file's module
    // doc comment).
    let out = run_via_node(
        "forward_reference",
        "disp(double_seven());\nfunction r = double_seven()\n  r = 3 + 4 + 3 + 4;\nend\n",
    );
    assert_eq!(out, "14");
}
