//! Integration test for the `tests/diff/simple-class/` fixture.
//!
//! Exercises CLOC12.173 PR2 — a **class expression** (`class { … }`, a
//! `ClassExpression` with a `MethodDefinition` body) now flows through the full
//! SIMPLE pipeline (parser → typed-AST bridge → passes → emitter) instead of
//! being declined at the bridge (`class_expression` → `UnsupportedSyntax`,
//! which dropped the whole file to WHITESPACE_ONLY).
//!
//! The fixture is `f(class { m() { return 1 + 2 } }, 3 + 4);` — an unknown call
//! `f(...)` (side effects, so DCE keeps it) with two arguments: a class
//! expression carrying one method `m`, and the foldable `3 + 4`. Three facts
//! prove the pipeline ran end-to-end rather than falling back:
//!   1. the class round-trips as `class{m(){…}}` (minified, no inner spaces),
//!      proving the bridge built a real `ClassExpression` the emitter prints;
//!   2. the method body folds — `return 1 + 2` → `return 3` — proving the
//!      constant-fold pass descended into the method's `FunctionExpression`
//!      body (PR1 wired `fold_class`); and
//!   3. the sibling argument folds — `3 + 4` → `7`.
//! A WHITESPACE_ONLY fallback — which a bridge decline forces for the *whole*
//! file — would instead re-emit the source verbatim, leaving `1 + 2` and
//! `3 + 4` unfolded and the source spacing intact.

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-class/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_class_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-class/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture. Strip spaces so the checks are
    // insensitive to inter-token whitespace.
    let a = actual.replace(' ', "");
    // (1) the class round-tripped — proving the bridge built a real
    //     `ClassExpression` the emitter can print, not a WHITESPACE_ONLY pass.
    assert!(
        a.contains("class{m()"),
        "class expression did not round-trip: {actual}"
    );
    // (2) the method body folded — proving the pass descended into the
    //     method's function body (`1 + 2` → `3`).
    assert!(
        a.contains("return 3") || a.contains("return3}"),
        "method body `1 + 2` did not fold to `3`: {actual}"
    );
    // (3) the sibling argument folded — proving the SIMPLE pipeline ran over
    //     the call (`3 + 4` → `7`), not a verbatim WHITESPACE_ONLY pass.
    assert!(
        a.contains("},7)"),
        "argument `3 + 4` did not fold to `7`: {actual}"
    );
    assert!(
        !a.contains("1+2") && !a.contains("3+4"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
