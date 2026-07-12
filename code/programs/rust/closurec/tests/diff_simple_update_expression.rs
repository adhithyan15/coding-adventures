//! Integration test for the `tests/diff/simple-update-expression/` fixture.
//!
//! Exercises CLOC12.158 PR2 — an **update expression** (`i++`) now flows
//! through the full SIMPLE pipeline (parser → typed-AST bridge → passes →
//! emitter) instead of declining at the bridge and dragging the whole file to
//! WHITESPACE_ONLY.
//!
//! The fixture puts `i++` as a statement next to a foldable `1 + 2`. Two facts
//! prove the pipeline ran end-to-end:
//!   1. `i++` round-trips as a postfix update — never silently dropped to `i`
//!      (which a bridge that returned the bare operand would have produced), and
//!   2. `1 + 2` folds to `3`.
//! A WHITESPACE_ONLY fallback — which a bridge decline would force — would
//! instead re-emit the source verbatim, leaving `1 + 2` unfolded.

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-update-expression/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_update_expression_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-update-expression/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture: the update survived as `i++` (postfix,
    // never dropped to `i`) AND the adjacent `1 + 2` folded — proving the
    // SIMPLE pipeline ran rather than falling back to WHITESPACE_ONLY.
    let a = actual.replace(' ', "");
    assert!(a.contains("i++"), "update did not round-trip (dropped to `i`?): {actual}");
    assert!(a.contains("log(3)"), "1+2 not folded — did it fall back to WHITESPACE_ONLY? {actual}");
    assert!(!a.contains("1+2"), "a pre-fold expression survived: {actual}");
}
