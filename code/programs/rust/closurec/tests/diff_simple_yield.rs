//! Integration test for the `tests/diff/simple-yield/` fixture.
//!
//! Exercises CLOC12.163 PR2 — a **generator function** and its **`yield`**
//! (`YieldExpression`) now flow through the full SIMPLE pipeline (parser →
//! typed-AST bridge → passes → emitter) instead of declining at the bridge and
//! dragging the whole file to WHITESPACE_ONLY (gap-164, now closed).
//!
//! The fixture is `use(function*(){yield 1 + 2;});` — a retained call (an
//! unknown `use(...)` has side effects, so DCE keeps it) whose argument is a
//! generator expression whose body yields the foldable `1 + 2`. Three facts
//! prove the pipeline ran end-to-end rather than falling back:
//!   1. the generator prints as `function*` (the bridge set the `generator`
//!      flag), and
//!   2. the `yield` round-trips (the bridge produced a real `YieldExpression`
//!      rather than declining), and
//!   3. the yield operand `1 + 2` folds to `3`.
//! A WHITESPACE_ONLY fallback — which a bridge decline would force for the
//! *whole file* — would instead re-emit the source verbatim, leaving `1 + 2`
//! unfolded (`yield 1+2`).

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-yield/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_yield_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-yield/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture. Strip spaces so the checks are
    // insensitive to inter-token whitespace.
    let a = actual.replace(' ', "");
    // (1) the generator marker survived — proving the bridge converted the
    //     generator function rather than declining to WHITESPACE_ONLY.
    assert!(
        a.contains("function*"),
        "generator `function*` did not round-trip: {actual}"
    );
    // (2) the yield survived — proving the bridge produced a real
    //     `YieldExpression`.
    assert!(
        a.contains("yield"),
        "`yield` did not round-trip: {actual}"
    );
    // (3) the operand folded — proving the SIMPLE pipeline ran over the
    //     generator body (`1 + 2` → `3`), not a verbatim WHITESPACE_ONLY pass.
    assert!(
        a.contains("yield3"),
        "yield operand `1 + 2` did not fold to `3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded `1+2` present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
