//! Integration test for the `tests/diff/simple-this/` fixture.
//!
//! Exercises CLOC12.165 PR2 — the **`this`** keyword (`ThisExpression`) now
//! flows through the full SIMPLE pipeline (parser → typed-AST bridge → passes →
//! emitter) instead of declining at the bridge and dragging the whole file to
//! WHITESPACE_ONLY (gap-166, now closed).
//!
//! The fixture is `this.f(1 + 2);` — a retained call (an unknown `this.f(...)`
//! member call has side effects, so DCE keeps it) whose receiver is `this` and
//! whose argument is the foldable `1 + 2`. Two facts prove the pipeline ran
//! end-to-end rather than falling back:
//!   1. the `this` receiver round-trips (the bridge produced a real
//!      `ThisExpression` rather than declining), and
//!   2. the argument `1 + 2` folds to `3`.
//! A WHITESPACE_ONLY fallback — which a bridge decline would force for the
//! *whole file* — would instead re-emit the source verbatim, leaving `1 + 2`
//! unfolded (`this.f(1 + 2)`).

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-this/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_this_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-this/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture. Strip spaces so the checks are
    // insensitive to inter-token whitespace.
    let a = actual.replace(' ', "");
    // (1) the `this` receiver survived — proving the bridge converted `this`
    //     to a real `ThisExpression` rather than declining to WHITESPACE_ONLY.
    assert!(
        a.contains("this.f"),
        "`this` receiver did not round-trip: {actual}"
    );
    // (2) the argument folded — proving the SIMPLE pipeline ran over the call
    //     (`1 + 2` → `3`), not a verbatim WHITESPACE_ONLY pass.
    assert!(
        a.contains("this.f(3)"),
        "argument `1 + 2` did not fold to `3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded `1+2` present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
