//! Integration test for the `tests/diff/simple-importexpr/` fixture.
//!
//! Exercises CLOC12.169 PR2 — the **`import(x)`** dynamic-import call
//! expression (`ImportExpression`) now flows through the full SIMPLE pipeline
//! (parser → typed-AST bridge → passes → emitter) instead of falling through
//! the bridge's internal-error arm and dragging the whole file to
//! WHITESPACE_ONLY (gap-170, now closed).
//!
//! The fixture is `f(import("m"), 1 + 2);` — a retained call (an unknown
//! `f(...)` has side effects, so DCE keeps it) whose first argument is the
//! dynamic import `import("m")` and whose second argument is the foldable
//! `1 + 2`. Two facts prove the pipeline ran end-to-end rather than falling
//! back:
//!   1. `import("m")` round-trips (the bridge produced a real
//!      `ImportExpression` node — a *compound* node with a real `source`
//!      operand, unlike the atomic `import.meta` leaf — rather than erroring),
//!      and
//!   2. the sibling argument `1 + 2` folds to `3`.
//! A WHITESPACE_ONLY fallback — which a bridge failure would force for the
//! *whole file* — would instead re-emit the source verbatim, leaving `1 + 2`
//! unfolded (`f(import("m"), 1 + 2)`).
//!
//! (`import(x)` is legal in any module or script context; the shared grammar
//! parses the bare statement, and the bridge lowers whatever the parser
//! produced. The point here is the *pipeline plumbing* — a compound
//! single-operand node reaching the emitter — not JS semantic validation. See
//! CLOC12.168's `simple-importmeta` fixture for the sibling atomic-leaf
//! `import.meta` case.)

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-importexpr/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_importexpr_fixture_matches_expected_stdout() {
    let flags = read_flags();
    let out = Command::new(BINARY)
        .args(&flags)
        .output()
        .expect("run closurec");

    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let actual = String::from_utf8_lossy(&out.stdout);
    let expected = std::fs::read_to_string("tests/diff/simple-importexpr/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture. Strip spaces so the checks are
    // insensitive to inter-token whitespace.
    let a = actual.replace(' ', "");
    // (1) the `import("m")` dynamic import survived — proving the bridge
    //     converted it to a real `ImportExpression` node rather than dropping
    //     the file to WHITESPACE_ONLY.
    assert!(
        a.contains("import(\"m\")"),
        "`import(\"m\")` did not round-trip: {actual}"
    );
    // (2) the sibling argument folded — proving the SIMPLE pipeline ran over
    //     the call (`1 + 2` → `3`), not a verbatim WHITESPACE_ONLY pass.
    assert!(
        a.contains("import(\"m\"),3"),
        "argument `1 + 2` did not fold to `3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded `1+2` present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
