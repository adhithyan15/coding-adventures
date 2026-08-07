//! End-to-end round-trip: IDL → SIR → JavaScript → `node`.
//!
//! Mirrors `scilab-to-semantic-ir/tests/e2e_node.rs`'s own harness exactly:
//! lower an IDL program to SIR with this crate, validate the module, emit
//! JavaScript with the merged `semantic-ir-to-javascript` backend (a
//! dev-dependency), write it to a temp file, and **execute it with
//! `node`**, asserting the printed output. Every test is gated on `node`
//! availability, matching every other `-to-semantic-ir` frontend's own
//! e2e harness in this repo.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use coding_adventures_idl_to_semantic_ir::compile_source;

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
    path.push(format!("idl_sir_e2e_{name}_{}.js", std::process::id()));
    // `create_new` fails if the path already exists (including as a
    // symlink) instead of following it -- see
    // `scilab-to-semantic-ir/tests/e2e_node.rs`'s identical comment for why.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp js (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes())
        .expect("write temp js");
    drop(file);

    let output = Command::new("node")
        .arg(&path)
        .output()
        .expect("spawn node");
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
        "FUNCTION seven\n RETURN, 3 + 4\nEND\nPRINT, seven()\n",
    );
    assert_eq!(out, "7");
}

#[test]
fn a_function_computing_its_output_conditionally_in_if_else_runs_in_node() {
    // The true end-to-end proof of the branch-locals-hoisting fix ported
    // from scilab-to-semantic-ir: a function computing its own return
    // value inside IF/ELSE (arguably the single most common
    // IDL/MATLAB/Scilab function shape there is).
    if !node_available() {
        eprintln!(
            "skipping a_function_computing_its_output_conditionally_in_if_else_runs_in_node: \
             `node` not available"
        );
        return;
    }
    let out = run_via_node(
        "conditional_output",
        "FUNCTION sign_of, x\n IF x GT 0 THEN BEGIN\n  y = 1\n ENDIF ELSE BEGIN\n  IF x LT 0 THEN BEGIN\n   y = -1\n  ENDIF ELSE BEGIN\n   y = 0\n  ENDELSE\n ENDELSE\n RETURN, y\nEND\nPRINT, sign_of(5)\nPRINT, sign_of(-5)\nPRINT, sign_of(0)\n",
    );
    assert_eq!(out, "1\n-1\n0");
}

#[test]
fn for_loop_accumulator_converges_in_node() {
    if !node_available() {
        eprintln!("skipping for_loop_accumulator_converges_in_node: `node` not available");
        return;
    }
    let out = run_via_node(
        "for_loop_accumulator",
        "total = 0\nFOR i = 1, 10 DO total = total + i\nPRINT, total\n",
    );
    assert_eq!(out, "55");
}

#[test]
fn while_loop_accumulator_converges_in_node() {
    if !node_available() {
        eprintln!("skipping while_loop_accumulator_converges_in_node: `node` not available");
        return;
    }
    let out = run_via_node(
        "while_loop_accumulator",
        "x = 0\nWHILE x LT 5 DO x = x + 1\nPRINT, x\n",
    );
    assert_eq!(out, "5");
}

#[test]
fn repeat_until_runs_in_node() {
    if !node_available() {
        eprintln!("skipping repeat_until_runs_in_node: `node` not available");
        return;
    }
    let out = run_via_node(
        "repeat_until",
        "x = 0\nREPEAT x = x + 1 UNTIL x GE 3\nPRINT, x\n",
    );
    assert_eq!(out, "3");
}

#[test]
fn array_literal_indexing_and_indexed_assignment_run_in_node() {
    if !node_available() {
        eprintln!(
            "skipping array_literal_indexing_and_indexed_assignment_run_in_node: `node` not \
             available"
        );
        return;
    }
    let out = run_via_node("indexing", "a = [10, 20, 30]\na[1] = 99\nPRINT, a[1]\n");
    assert_eq!(out, "99");
}

#[test]
fn matrix_product_operators_hash_and_hash_hash_run_in_node() {
    if !node_available() {
        eprintln!(
            "skipping matrix_product_operators_hash_and_hash_hash_run_in_node: `node` not \
             available"
        );
        return;
    }
    // 1x1 "matrices" (genuinely rank-1, single-element arrays) so `#`/`##`'s
    // operand order is observable without needing a real 2-D array (this
    // frontend has no in-scope way to construct one -- see this crate's
    // README).
    let out = run_via_node("hash_hash", "a = [2]\nb = [3]\nc = a ## b\nPRINT, c\n");
    assert_eq!(out, "6");
}

#[test]
fn transpose_runs_in_node() {
    if !node_available() {
        eprintln!("skipping transpose_runs_in_node: `node` not available");
        return;
    }
    // `array_runtime::ops::transpose` treats a rank-1 `[n]` vector as a 1×n
    // row and transposes it to a genuine n×1 column -- so the printed
    // result is one element per line, not the original space-separated row.
    let out = run_via_node("transpose", "a = [1,2,3]\nb = TRANSPOSE(a)\nPRINT, b\n");
    assert_eq!(out, "1\n2\n3");
}

#[test]
fn indgen_runs_in_node() {
    if !node_available() {
        eprintln!("skipping indgen_runs_in_node: `node` not available");
        return;
    }
    let out = run_via_node("indgen", "a = INDGEN(5)\nPRINT, a\n");
    assert_eq!(out, "0 1 2 3 4");
}

#[test]
fn keyword_argument_call_runs_in_node() {
    if !node_available() {
        eprintln!("skipping keyword_argument_call_runs_in_node: `node` not available");
        return;
    }
    let out = run_via_node(
        "keyword_call",
        "FUNCTION plot_it, x, COLOR=hue\n RETURN, x + hue\nEND\nPRINT, plot_it(1, COLOR=10)\n",
    );
    assert_eq!(out, "11");
}

#[test]
fn boolean_keyword_shorthand_runs_in_node() {
    if !node_available() {
        eprintln!("skipping boolean_keyword_shorthand_runs_in_node: `node` not available");
        return;
    }
    let out = run_via_node(
        "bool_keyword",
        "FUNCTION check, YLOG=ylog\n RETURN, ylog\nEND\nPRINT, check(/YLOG)\n",
    );
    assert_eq!(out, "1");
}

#[test]
fn two_namespace_pro_and_function_both_run_correctly_in_node() {
    if !node_available() {
        eprintln!("skipping two_namespace_pro_and_function_both_run_correctly_in_node: `node` not available");
        return;
    }
    let out = run_via_node(
        "two_namespace",
        "PRO DOIT, x\n PRINT, x * 2\nEND\nFUNCTION DOIT, x\n RETURN, x * 3\nEND\nDOIT, 5\nPRINT, DOIT(5)\n",
    );
    assert_eq!(out, "10\n15");
}

#[test]
fn case_folded_identifiers_resolve_to_the_same_binding_in_node() {
    if !node_available() {
        eprintln!("skipping case_folded_identifiers_resolve_to_the_same_binding_in_node: `node` not available");
        return;
    }
    let out = run_via_node("case_folding", "MyVar = 5\nPRINT, MYVAR\n");
    assert_eq!(out, "5");
}
