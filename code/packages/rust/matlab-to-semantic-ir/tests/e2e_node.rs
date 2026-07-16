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
//! `ElementwiseOp`/`MatMul` path. The JS backend now implements real codegen
//! for that path (the SIR22 base cut), so a real array/matrix MATLAB
//! program round-trips through this pipeline too — the tests below prove
//! that with actual `A = [1 2; 3 4]; B = A * A;`-style source, not hand-built
//! SIR. `disp` on a computed matrix has no display-formatting story yet
//! (out of scope for the codegen PR that unblocked this file), so each test
//! reads back a single element via MATLAB indexing (`disp(B(1, 1))`) rather
//! than `disp`-ing the whole matrix.

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

#[test]
fn matrix_multiplication_runs_in_node() {
    if !node_available() {
        eprintln!("skipping matrix_multiplication_runs_in_node: `node` not available");
        return;
    }
    // A * A where A = [1 2; 3 4] -> [7 10; 15 22] (standard matrix
    // product, not elementwise). A(1, 1) (1-based MATLAB indexing) is
    // the top-left element, 7.
    let out = run_via_node(
        "matmul",
        "A = [1 2; 3 4];\nB = A * A;\ndisp(B(1, 1));\n",
    );
    assert_eq!(out, "7");
}

#[test]
fn elementwise_scale_with_a_bare_scalar_operand_runs_in_node() {
    if !node_available() {
        eprintln!(
            "skipping elementwise_scale_with_a_bare_scalar_operand_runs_in_node: `node` not available"
        );
        return;
    }
    // A .* 2 -- the frontend emits `2` as a bare (unwrapped) scalar
    // ElementwiseOp operand, exactly the shape
    // `semantic-ir-to-javascript`'s runtime coercion exists for. A(2, 2)
    // is 4 before scaling, 8 after.
    let out = run_via_node(
        "elementwise_scalar",
        "A = [1 2; 3 4];\nB = A .* 2;\ndisp(B(2, 2));\n",
    );
    assert_eq!(out, "8");
}

#[test]
fn indexed_assignment_mutates_in_place_and_reads_back_in_node() {
    if !node_available() {
        eprintln!(
            "skipping indexed_assignment_mutates_in_place_and_reads_back_in_node: `node` not available"
        );
        return;
    }
    let out = run_via_node("index_set", "A = [1 2 3];\nA(2) = 9;\ndisp(A(2));\n");
    assert_eq!(out, "9");
}

#[test]
fn range_and_transpose_run_in_node() {
    if !node_available() {
        eprintln!("skipping range_and_transpose_run_in_node: `node` not available");
        return;
    }
    // transpose([1 2; 3 4]) = [1 2; 3 4] with rows/cols swapped, i.e.
    // [1 3; 2 4]; B(2, 1) (1-based) is the original A(1, 2) = 2. The
    // range `v = 1:5` is exercised for its manifest/codegen path
    // (compiles and runs without error) even though this particular
    // program never reads `v` back.
    let out = run_via_node(
        "range_and_transpose",
        "A = [1 2; 3 4];\nv = 1:5;\nB = A';\ndisp(B(2, 1));\n",
    );
    assert_eq!(out, "2");
}
