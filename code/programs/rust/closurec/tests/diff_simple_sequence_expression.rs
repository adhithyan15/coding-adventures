//! Integration test for the `tests/diff/simple-sequence-expression/` fixture.
//!
//! Exercises CLOC12.160 PR2 — a **`SequenceExpression`** (the comma operator)
//! `(a, 1 + 2)` now flows through the full SIMPLE pipeline (parser → typed-AST
//! bridge → passes → emitter) instead of declining at the bridge and dragging
//! the whole file to WHITESPACE_ONLY (gap-161, now closed).
//!
//! The fixture puts `(a, 1 + 2)` as the sole argument to `log(...)`, where a
//! bare comma would otherwise read as a second argument — so the sequence must
//! round-trip parenthesised. Two facts prove the pipeline ran end-to-end:
//!   1. the sequence `(a, 3)` round-trips as a single wrapped argument — the
//!      bridge produced a real `SequenceExpression` rather than declining, and
//!   2. its second operand `1 + 2` folds to `3`.
//! A WHITESPACE_ONLY fallback — which a bridge decline would force — would
//! instead re-emit the source verbatim, leaving `1 + 2` unfolded.

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-sequence-expression/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_sequence_expression_fixture_matches_expected_stdout() {
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
    let expected =
        std::fs::read_to_string("tests/diff/simple-sequence-expression/expected.stdout")
            .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture: the parenthesised sequence survived
    // (proving the bridge converted it, not WHITESPACE_ONLY) AND its second
    // operand `1 + 2` folded to `3` — proving the SIMPLE pipeline ran.
    let a = actual.replace(' ', "");
    assert!(
        a.contains("(a,3)"),
        "sequence did not round-trip with a folded operand: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "a pre-fold expression survived — did it fall back to WHITESPACE_ONLY? {actual}"
    );
}
