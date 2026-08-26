//! End-to-end execution proof: Java → SIR → JavaScript → `node`, for
//! task #64's own `break`/`continue` support.
//!
//! Mirrors `e2e_python.rs`'s own "observe via `main`'s return value"
//! harness (see that file's doc comment for the full rationale — `System.
//! out.println` is a qualified method call, out of scope for this
//! frontend), but targets the JavaScript backend instead of Python: as of
//! this crate's own v0.12.0, JavaScript is the only backend that accepts
//! `Feature::LoopControl` (`semantic-ir-to-javascript` v0.54.0 — see task
//! #62). None of these tests use string concatenation, so JS's own
//! `Feature::StringInterpolation` gap (the reason `e2e_python.rs` picked
//! Python instead) never applies here.
//!
//! Two of these tests (`do_while_continue_...`, `classic_for_continue_...`)
//! are direct regression tests for the two correctness bugs task #64 found
//! and fixed in `lower_do_while_statement`/`lower_for_statement_inner`
//! (see those functions' own doc comments): before the fix, the specific
//! `continue` placement each test exercises made the loop **run forever**
//! — not just compute the wrong answer. If a future change reintroduces
//! either bug, the affected test here will hang rather than fail cleanly;
//! that is the nature of a termination-correctness regression test, not a
//! flaw in the test itself.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::process::Command;

use java_to_semantic_ir::compile_source;
use semantic_ir::Stmt;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Lower `java_src`, validate it, then run it through the JavaScript
/// backend and `node`, returning trimmed stdout. See this file's own doc
/// comment for the "last statement becomes the observed return value"
/// convention `java_src`'s `main` body must follow.
fn run_via_node(name: &str, java_src: &str) -> String {
    let mut module = compile_source(java_src, "prog").expect("lowering should succeed");
    let report = semantic_ir::validate(&module);
    assert!(
        report.is_ok(),
        "SIR validation failed for {name}: {:?}",
        report.issues
    );

    let main = module
        .functions
        .iter_mut()
        .find(|f| f.name == "main")
        .expect("expected a synthesized `main` function");
    match main.body.stmts.pop() {
        Some(Stmt::ExprStmt { expr, .. }) => main.body.value = expr,
        Some(other) => panic!(
            "expected `main`'s last statement to be a bare expression statement, got {other:?}"
        ),
        None => panic!("`main` has no statements to observe"),
    }
    // The JavaScript backend has no special-casing for a function named
    // `main` (unlike the Python backend's auto-invoke convention — see
    // `e2e_python.rs`'s own comment), so this rename is purely cosmetic
    // here; kept for parity with that harness and to avoid ever colliding
    // with a real top-level JS `main` some future test source declares.
    main.name = "probe".to_string();

    let artifact =
        semantic_ir_to_javascript::compile(&module).expect("backend emit should succeed");

    let mut path = std::env::temp_dir();
    path.push(format!("java_sir_e2e_{name}_{}.js", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create temp js (create_new, not following an existing symlink)");
    file.write_all(artifact.source.as_bytes())
        .expect("write temp js");
    writeln!(file, "console.log(probe());").expect("write console.log epilogue");
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

fn wrap(body: &str) -> String {
    format!("class Main {{ public static void main(String[] args) {{ {body} }} }}")
}

#[test]
fn while_loop_continue_skips_evens_and_break_stops_after_seven() {
    if !node_available() {
        eprintln!("skipping: `node` not available");
        return;
    }
    let src = wrap(concat!(
        "int i = 0; int sum = 0; ",
        "while (i < 10) { ",
        "  i = i + 1; ",
        "  if (i % 2 == 0) { continue; } ",
        "  if (i > 7) { break; } ",
        "  sum = sum + i; ",
        "} ",
        "sum;"
    ));
    // Odd i values up to and including 7: 1 + 3 + 5 + 7 = 16.
    assert_eq!(run_via_node("while_break_continue", &src), "16");
}

#[test]
fn do_while_continue_still_terminates_and_computes_the_right_count() {
    // Direct regression test for the flag-clear-vs-continue bug this
    // file's own doc comment describes: before the fix, the `continue`
    // on `i`'s first even value left the loop's own guard flag
    // permanently `true`, so `flag || cond` never went false — this
    // test hung forever rather than returning "3".
    if !node_available() {
        eprintln!("skipping: `node` not available");
        return;
    }
    let src = wrap(concat!(
        "int i = 0; int count = 0; ",
        "do { ",
        "  i = i + 1; ",
        "  if (i % 2 == 0) { continue; } ",
        "  count = count + 1; ",
        "} while (i < 6); ",
        "count;"
    ));
    // i runs 1..6; odd values (1, 3, 5) each increment count once.
    assert_eq!(run_via_node("do_while_continue", &src), "3");
}

#[test]
fn classic_for_continue_still_terminates_and_sums_the_odd_values() {
    // Direct regression test for the update-clause-vs-continue bug this
    // file's own doc comment describes: before the fix, `continue` on
    // `i == 0` (the loop's very first, always-even iteration) skipped
    // the appended `i++` entirely — `i` stayed `0` forever, an infinite
    // loop from the first iteration, not an edge case reached only after
    // several iterations.
    if !node_available() {
        eprintln!("skipping: `node` not available");
        return;
    }
    let src = wrap(concat!(
        "int sum = 0; ",
        "for (int i = 0; i < 10; i++) { ",
        "  if (i % 2 == 0) { continue; } ",
        "  sum = sum + i; ",
        "} ",
        "sum;"
    ));
    // Odd i in [0, 10): 1 + 3 + 5 + 7 + 9 = 25.
    assert_eq!(run_via_node("classic_for_continue", &src), "25");
}

#[test]
fn enhanced_for_break_stops_before_the_first_element_over_five() {
    if !node_available() {
        eprintln!("skipping: `node` not available");
        return;
    }
    let src = wrap(concat!(
        "int[] xs = {1, 2, 3, 7, 8, 9}; ",
        "int sum = 0; ",
        "for (int x : xs) { ",
        "  if (x > 5) { break; } ",
        "  sum = sum + x; ",
        "} ",
        "sum;"
    ));
    // 1 + 2 + 3 = 6; the loop breaks the moment it reaches 7.
    assert_eq!(run_via_node("enhanced_for_break", &src), "6");
}

#[test]
fn nested_while_loops_break_targets_only_the_innermost_loop() {
    // The real behavioral counterpart of `test_lower.rs`'s own
    // structural-only `break_targets_the_innermost_enclosing_loop_when_
    // nested` test: an inner `break` must stop only the inner loop, not
    // propagate to the outer one.
    if !node_available() {
        eprintln!("skipping: `node` not available");
        return;
    }
    let src = wrap(concat!(
        "int total = 0; int i = 0; ",
        "while (i < 3) { ",
        "  int j = 0; ",
        "  while (j < 5) { ",
        "    if (j == 2) { break; } ",
        "    total = total + 1; ",
        "    j = j + 1; ",
        "  } ",
        "  i = i + 1; ",
        "} ",
        "total;"
    ));
    // Each of the 3 outer iterations runs the inner loop for j = 0, 1
    // (adding 1 each time) before j == 2 breaks it — 3 * 2 = 6. If the
    // inner `break` incorrectly escaped to the outer loop instead, the
    // outer loop itself would run only once (total == 2).
    assert_eq!(run_via_node("nested_break_innermost", &src), "6");
}

#[test]
fn nested_while_loops_continue_targets_only_the_innermost_loop() {
    if !node_available() {
        eprintln!("skipping: `node` not available");
        return;
    }
    let src = wrap(concat!(
        "int total = 0; int i = 0; ",
        "while (i < 3) { ",
        "  int j = 0; ",
        "  while (j < 5) { ",
        "    j = j + 1; ",
        "    if (j % 2 == 0) { continue; } ",
        "    total = total + 1; ",
        "  } ",
        "  i = i + 1; ",
        "} ",
        "total;"
    ));
    // Each of the 3 outer iterations runs the full inner loop (j = 1..5),
    // incrementing total on the 3 odd values of j (1, 3, 5) — 3 * 3 = 9.
    // If the inner `continue` incorrectly targeted the outer loop instead,
    // this would either hang or produce a very different count.
    assert_eq!(run_via_node("nested_continue_innermost", &src), "9");
}
