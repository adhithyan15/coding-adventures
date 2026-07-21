//! End-to-end round-trip: Scilab → SIR → JavaScript → `node`.
//!
//! Mirrors `matlab-to-semantic-ir/tests/e2e_node.rs`'s own harness exactly:
//! lower a Scilab program to SIR with this crate, validate the module, emit
//! JavaScript with the merged `semantic-ir-to-javascript` backend (a
//! dev-dependency), write it to a temp file, and **execute it with
//! `node`**, asserting the printed output. Every test is gated on `node`
//! availability, matching every other `-to-semantic-ir` frontend's own
//! e2e harness in this repo.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use scilab_to_semantic_ir::compile_source;

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
    path.push(format!("scilab_sir_e2e_{name}_{}.js", std::process::id()));
    // `create_new` fails if the path already exists (including as a
    // symlink) instead of following it -- see
    // `matlab-to-semantic-ir/tests/e2e_node.rs`'s identical comment (a
    // confirmed, fixed LOW-severity finding from that crate's own security
    // review) for why this is `create_new`, not `std::fs::write`.
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
        "function r = seven()\n  r = 3 + 4;\nendfunction\ndisp(seven());\n",
    );
    assert_eq!(out, "7");
}

#[test]
fn a_function_computing_its_output_conditionally_in_if_else_runs_in_node() {
    // Security regression (round 4 review) -- the actual, end-to-end
    // headline case: a function computing its own designated output
    // variable inside `if`/`else` (arguably the single most common
    // Scilab/MATLAB function shape there is) previously produced a
    // module that ALWAYS failed `semantic_ir::validate` (a dangling
    // "var-ref scope=local references unknown name" error), since the
    // output variable's own `LetStarBinding` lived inside a branch's
    // nested `Block`, invisible to the function's own trailing
    // `VarRef` reading it as the return value. This is the true
    // end-to-end proof the fix (hoisting a real pre-declaration to the
    // enclosing scope) works: it compiles AND actually runs correctly.
    if !node_available() {
        eprintln!(
            "skipping a_function_computing_its_output_conditionally_in_if_else_runs_in_node: \
             `node` not available"
        );
        return;
    }
    let out = run_via_node(
        "conditional_output",
        "function y = sign_of(x)\n  if x > 0\n    y = 1;\n  elseif x < 0\n    y = -1;\n  else\n    y = 0;\n  end\nendfunction\ndisp(sign_of(5));\ndisp(sign_of(-5));\ndisp(sign_of(0));\n",
    );
    assert_eq!(out, "1\n-1\n0");
}

#[test]
fn a_while_loop_introducing_a_fresh_variable_is_visible_afterward_in_node() {
    // Security regression (round 4 review): `lower_while` did zero scope
    // tracking around its body at all -- a fresh variable assigned inside
    // a `while` loop was correctly known to this crate's own lowering-time
    // bookkeeping (no rewind ever undid it, unlike `if`/`select`'s
    // mutually-exclusive branches or `for`'s own loop-scoped counter), but
    // its `LetStarBinding` still lived inside the loop body's own nested
    // `Block`, invisible to `semantic_ir::validate` from the ENCLOSING
    // scope -- so reading it after the loop failed identically to the
    // `if`/`else` case.
    if !node_available() {
        eprintln!(
            "skipping a_while_loop_introducing_a_fresh_variable_is_visible_afterward_in_node: \
             `node` not available"
        );
        return;
    }
    let out = run_via_node(
        "while_fresh_var",
        "n = 3;\nwhile n > 0\n  doubled = n * 2;\n  n = n - 1;\nend\ndisp(doubled);\n",
    );
    assert_eq!(out, "2");
}

#[test]
fn a_for_loop_body_introducing_a_fresh_non_counter_variable_is_visible_afterward_in_node() {
    // Security regression (round 4 review): the identical hazard as the
    // `while` case above, for a `for` loop's BODY (not its own counter
    // variable, which already has correct, separate, loop-scoped
    // handling) introducing a fresh name for the first time.
    if !node_available() {
        eprintln!(
            "skipping a_for_loop_body_introducing_a_fresh_non_counter_variable_is_visible_afterward_in_node: \
             `node` not available"
        );
        return;
    }
    let out = run_via_node(
        "for_body_fresh_var",
        "for i = 1:3\n  last_seen = i * 10;\nend\ndisp(last_seen);\n",
    );
    assert_eq!(out, "30");
}

#[test]
fn nested_literal_arithmetic_runs_in_node() {
    if !node_available() {
        eprintln!("skipping nested_literal_arithmetic_runs_in_node: `node` not available");
        return;
    }
    let out = run_via_node("nested_literal_arithmetic", "disp(2 * 3 + 4 * 5);\n");
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
    let out = run_via_node(
        "forward_reference",
        "disp(double_seven());\nfunction r = double_seven()\n  r = 3 + 4 + 3 + 4;\nendfunction\n",
    );
    assert_eq!(out, "14");
}

#[test]
fn matrix_multiplication_runs_in_node() {
    if !node_available() {
        eprintln!("skipping matrix_multiplication_runs_in_node: `node` not available");
        return;
    }
    // A * A where A = [1 2; 3 4] -> [7 10; 15 22]. A(1, 1) (1-based) = 7.
    let out = run_via_node("matmul", "A = [1 2; 3 4];\nB = A * A;\ndisp(B(1, 1));\n");
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
    let out = run_via_node(
        "range_and_transpose",
        "A = [1 2; 3 4];\nv = 1:5;\nB = A';\ndisp(B(2, 1));\n",
    );
    assert_eq!(out, "2");
}

#[test]
fn select_case_desugaring_produces_the_correct_branch_in_node() {
    if !node_available() {
        eprintln!(
            "skipping select_case_desugaring_produces_the_correct_branch_in_node: `node` not available"
        );
        return;
    }
    // `y` is pre-declared before the `select` -- the SIR validator scopes
    // each branch `Block` lexically (mark/rewind around every block, per
    // `semantic-ir/src/validator.rs`'s `check_block`), so a name that is
    // only ever *introduced* inside a branch (`LetStarBinding`) does not
    // survive past it, even though this frontend's own lowering-time
    // `FunctionCtx` (matching real Scilab/MATLAB semantics) treats an
    // `if`/`select`-introduced binding as scoped to the whole function.
    // Pre-declaring `y` here means each branch re-*assigns* it (`Assign`,
    // which does not introduce a new name into the validator's scope) --
    // ordinary, idiomatic Scilab practice matches this shape too.
    let out = run_via_node(
        "select_case",
        "x = 2;\ny = 0;\nselect x\n  case 1\n    y = 10;\n  case 2\n    y = 20;\n  else\n    y = 0;\nend\ndisp(y);\n",
    );
    assert_eq!(out, "20");
}

#[test]
fn select_case_falls_through_to_else_when_nothing_matches_in_node() {
    if !node_available() {
        eprintln!(
            "skipping select_case_falls_through_to_else_when_nothing_matches_in_node: `node` not available"
        );
        return;
    }
    let out = run_via_node(
        "select_case_else",
        "x = 99;\ny = 0;\nselect x\n  case 1\n    y = 10;\n  case 2\n    y = 20;\n  else\n    y = -1;\nend\ndisp(y);\n",
    );
    assert_eq!(out, "-1");
}

#[test]
fn percent_pi_constant_computes_correctly_in_node() {
    if !node_available() {
        eprintln!("skipping percent_pi_constant_computes_correctly_in_node: `node` not available");
        return;
    }
    // Observe the branch taken by an `if` rather than `disp`-ing a raw
    // comparison result directly -- the latter would hit an unrelated,
    // already-documented representational difference between a bare
    // comparison's runtime JS shape and Scilab's own "logicals are
    // doubles" convention (see `matlab-to-semantic-ir`'s own oracle tests,
    // which take the identical precaution for the identical reason).
    let out = run_via_node(
        "percent_pi",
        "if %pi > 3\n  disp(1);\nelse\n  disp(0);\nend\n",
    );
    assert_eq!(out, "1");
}

#[test]
fn for_loop_accumulator_converges_in_node() {
    if !node_available() {
        eprintln!("skipping for_loop_accumulator_converges_in_node: `node` not available");
        return;
    }
    let out = run_via_node(
        "for_accumulator",
        "total = 0;\nfor i = 1:10\n  total = total + i;\nend\ndisp(total);\n",
    );
    assert_eq!(out, "55");
}

#[test]
fn for_loop_reusing_an_already_assigned_variable_as_the_counter_runs_in_node() {
    // Security regression (round 3 review, a bug the round-2 `HashSet`
    // optimization itself introduced): reusing an already-assigned
    // variable as a `for`-loop counter -- an ordinary Scilab idiom --
    // previously desynced `FunctionCtx::locals`/`locals_set`, so the name
    // "forgot" it was already known once the loop's own scope rewound.
    // The resulting JavaScript had two top-level `let y` declarations in
    // the same scope, which `node` rejected outright with "Identifier 'y'
    // has already been declared" -- this test would fail with that exact
    // error if the bug were still present.
    if !node_available() {
        eprintln!(
            "skipping for_loop_reusing_an_already_assigned_variable_as_the_counter_runs_in_node: \
             `node` not available"
        );
        return;
    }
    let out = run_via_node(
        "for_reuses_existing_var",
        "y = 1;\nfor y = 1:3\n  disp(y);\nend\ny = 99;\ndisp(y);\n",
    );
    assert_eq!(out, "1\n2\n3\n99");
}
