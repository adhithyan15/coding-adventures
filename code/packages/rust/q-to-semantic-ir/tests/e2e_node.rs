//! End-to-end round-trip: Q → SIR → JavaScript → `node`.
//!
//! Mirrors `apl-to-semantic-ir`'s/`maple-to-semantic-ir`'s own
//! `tests/e2e_node.rs` harness exactly: lower a Q program to SIR with this
//! crate, validate the module, emit JavaScript with the merged
//! `semantic-ir-to-javascript` backend (a dev-dependency), write it to a
//! temp file, and **execute it with `node`**, asserting the printed output.
//!
//! `tests/oracle.rs` already runs a much larger corpus through `node` and
//! diffs it against `q-runtime`'s own ground truth (including a documented,
//! not-fixed-here shared-crate display gap for negative NDArray results) --
//! this file is a smaller, curated set proving the SIR22 codegen path is
//! genuinely executable end-to-end for each of this crate's own distinctive
//! constructs, with every expected value chosen to be positive so none of
//! them are affected by that display gap (see `tests/oracle.rs`'s module
//! doc comment for the full writeup of which shapes are/aren't affected).
//!
//! The one construct new to this crate relative to every prior SIR22
//! frontend (APL/J) is the function-literal machinery -- `Function` /
//! `DirectCall` / `MakeClosure` / `IndirectCall` -- so this file weights its
//! coverage toward that: a named function, an inline immediately-applied
//! literal, a function calling another already-defined function, and the
//! genuinely higher-order case (a function value passed as an argument and
//! called dynamically through a parameter).

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use q_to_semantic_ir::compile_source;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_via_node(name: &str, src: &str) -> String {
    let module = compile_source(src, "prog").unwrap_or_else(|e| panic!("lowering failed: {e}"));
    let report = semantic_ir::validate(&module);
    assert!(report.is_ok(), "SIR validation failed for {name}: {:?}", report.issues);
    let artifact =
        semantic_ir_to_javascript::compile(&module).expect("backend emit should succeed");

    let mut path = std::env::temp_dir();
    path.push(format!("q_sir_e2e_{name}_{}.js", std::process::id()));
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
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn dyadic_arithmetic_runs_in_node() {
    if !node_available() {
        eprintln!("skipping dyadic_arithmetic_runs_in_node: `node` not available");
        return;
    }
    let out = run_via_node("dyadic_arithmetic", "2*3+4\n");
    assert_eq!(out, "14");
}

#[test]
fn reduce_and_scan_run_in_node() {
    if !node_available() {
        eprintln!("skipping reduce_and_scan_run_in_node: `node` not available");
        return;
    }
    assert_eq!(run_via_node("reduce_add", "+/1 2 3 4\n"), "10");
    assert_eq!(run_via_node("scan_running_sum", "+\\1 2 3\n"), "1 3 6");
}

#[test]
fn til_zero_based_runs_in_node() {
    if !node_available() {
        eprintln!("skipping til_zero_based_runs_in_node: `node` not available");
        return;
    }
    // The single most safety-critical assertion for `!` (MA11 §4): `!5` is
    // `0 1 2 3 4`, never APL's 1-based `1 2 3 4 5`. This also proves the
    // `zero_base_index` correction (`- 1` wrapping IndexGenerator) is wired
    // correctly end to end, through real codegen and a real `node` run.
    let out = run_via_node("til", "!5\n");
    assert_eq!(out, "0 1 2 3 4");
}

#[test]
fn the_five_new_q_specific_builtins_run_in_node() {
    if !node_available() {
        eprintln!("skipping the_five_new_q_specific_builtins_run_in_node: `node` not available");
        return;
    }
    assert_eq!(run_via_node("q_first", "*1 2 3\n"), "1");
    assert_eq!(run_via_node("q_where", "&0 1 1 0 1\n"), "1 2 4");
    assert_eq!(run_via_node("q_reverse", "|1 2 3\n"), "3 2 1");
    assert_eq!(run_via_node("q_not", "~0 1 5\n"), "1 0 0");
    assert_eq!(run_via_node("q_take", "5#1 2 3\n"), "1 2 3 1 2");
    assert_eq!(run_via_node("q_drop", "2_1 2 3 4\n"), "3 4");
    assert_eq!(run_via_node("q_match_true", "(1 2 3)~1 2 3\n"), "1");
    assert_eq!(run_via_node("q_match_false", "(1 2 3)~1 2 4\n"), "0");
}

#[test]
fn join_reusing_catenate_runs_in_node() {
    if !node_available() {
        eprintln!("skipping join_reusing_catenate_runs_in_node: `node` not available");
        return;
    }
    assert_eq!(run_via_node("join", "1,2 3\n"), "1 2 3");
}

#[test]
fn a_named_top_level_function_literal_runs_in_node() {
    if !node_available() {
        eprintln!("skipping a_named_top_level_function_literal_runs_in_node: `node` not available");
        return;
    }
    assert_eq!(run_via_node("named_fn_dyadic", "f:{x+y}\n2 f 3\n"), "5");
    assert_eq!(run_via_node("named_fn_monadic", "f:{x+1}\nf 5\n"), "6");
    assert_eq!(run_via_node("named_fn_explicit_params", "f:{[a;b] a*b}\n3 f 4\n"), "12");
}

#[test]
fn an_inline_immediately_applied_function_literal_runs_in_node() {
    if !node_available() {
        eprintln!(
            "skipping an_inline_immediately_applied_function_literal_runs_in_node: `node` not available"
        );
        return;
    }
    assert_eq!(run_via_node("inline_monadic", "{x*2} 5\n"), "10");
    assert_eq!(run_via_node("inline_dyadic", "2 {x+y} 3\n"), "5");
}

#[test]
fn a_function_calling_another_already_defined_function_runs_in_node() {
    if !node_available() {
        eprintln!(
            "skipping a_function_calling_another_already_defined_function_runs_in_node: `node` not available"
        );
        return;
    }
    let out = run_via_node("chained_calls", "double:{x*2}\nadd1:{x+1}\ndouble(add1 5)\n");
    assert_eq!(out, "12");
}

#[test]
fn the_higher_order_function_value_case_runs_in_node() {
    if !node_available() {
        eprintln!("skipping the_higher_order_function_value_case_runs_in_node: `node` not available");
        return;
    }
    // `apply`'s own parameter `g` is called dynamically (IndirectCall)
    // inside its body -- proving `MakeClosure`/`IndirectCall` genuinely
    // execute correctly together, not just validate.
    let out = run_via_node("higher_order", "apply:{[g] g 5}\ninc:{x+1}\napply inc\n");
    assert_eq!(out, "6");
}

#[test]
fn a_function_reads_a_top_level_global_variable_runs_in_node() {
    if !node_available() {
        eprintln!(
            "skipping a_function_reads_a_top_level_global_variable_runs_in_node: `node` not available"
        );
        return;
    }
    // Proves the Global (not Local) top-level scoping decision (src/
    // lower.rs's module doc comment) genuinely works across two SEPARATE
    // compiled JS functions -- `n` is assigned in `main`, and read from
    // inside the sibling `q_lambda_*` function `f` compiles to.
    let out = run_via_node("global_read", "n:10\nf:{x+n}\nf 5\n");
    assert_eq!(out, "15");
}

#[test]
fn multi_statement_function_body_runs_in_node() {
    if !node_available() {
        eprintln!("skipping multi_statement_function_body_runs_in_node: `node` not available");
        return;
    }
    let out = run_via_node("multi_stmt_body", "f:{[x] a:x+1; a*2}\nf 5\n");
    assert_eq!(out, "12"); // (5+1)*2
}

#[test]
fn list_literal_matches_stranding_runs_in_node() {
    if !node_available() {
        eprintln!("skipping list_literal_matches_stranding_runs_in_node: `node` not available");
        return;
    }
    assert_eq!(run_via_node("list_literal", "(1;2;3)\n"), "1 2 3");
}
