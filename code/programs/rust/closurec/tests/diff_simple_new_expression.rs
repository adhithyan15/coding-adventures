//! Integration test for the `tests/diff/simple-new-expression/` fixture.
//!
//! Exercises CLOC12.159 PR2 — a **`new` expression** (`new Widget(1 + 2)`) now
//! flows through the full SIMPLE pipeline (parser → typed-AST bridge → passes →
//! emitter) instead of declining at the bridge and dragging the whole file to
//! WHITESPACE_ONLY (gap-160, now closed).
//!
//! The fixture puts `new Widget(1 + 2)` as an argument to `log(...)` so the
//! call keeps the construction alive. Two facts prove the pipeline ran
//! end-to-end:
//!   1. the `new Widget(...)` round-trips — the bridge produced a real
//!      `NewExpression` rather than declining, and
//!   2. the argument `1 + 2` folds to `3`.
//! A WHITESPACE_ONLY fallback — which a bridge decline would force — would
//! instead re-emit the source verbatim, leaving `1 + 2` unfolded.

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-new-expression/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_new_expression_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-new-expression/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture: the `new Widget(...)` construction
    // survived (proving the bridge converted it, not WHITESPACE_ONLY) AND its
    // argument `1 + 2` folded to `3` — proving the SIMPLE pipeline ran.
    let a = actual.replace(' ', "");
    assert!(
        a.contains("newWidget(3)"),
        "new expression did not round-trip with a folded arg: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "a pre-fold expression survived — did it fall back to WHITESPACE_ONLY? {actual}"
    );
}
