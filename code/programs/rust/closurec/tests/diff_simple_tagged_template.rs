//! Integration test for the `tests/diff/simple-tagged-template/` fixture.
//!
//! Exercises CLOC12.161 PR2 — a **`TaggedTemplateExpression`** (a tagged
//! template `` tag`abc` ``) now flows through the full SIMPLE pipeline
//! (parser → typed-AST bridge → passes → emitter) instead of declining at the
//! bridge and dragging the whole file to WHITESPACE_ONLY (gap-162, now closed).
//!
//! The fixture is `log(tag`abc`, 1 + 2);` — a call whose first argument is a
//! tagged template and whose second is a foldable `1 + 2`. Two facts prove the
//! pipeline ran end-to-end:
//!   1. the tagged template `` tag`abc` `` round-trips (the bridge produced a
//!      real `TaggedTemplateExpression` rather than declining), and
//!   2. the sibling operand `1 + 2` folds to `3`.
//! A WHITESPACE_ONLY fallback — which a bridge decline would force for the
//! *whole file* — would instead re-emit the source verbatim, leaving `1 + 2`
//! unfolded. (Substitution templates `` `a${x}b` `` do not parse in the grammar
//! yet, so the tagged form is exercised no-substitution here, matching the
//! template bridge's scope.)

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-tagged-template/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_tagged_template_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-tagged-template/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture: the tagged template survived (proving
    // the bridge converted it, not WHITESPACE_ONLY) AND the sibling `1 + 2`
    // folded to `3` — proving the SIMPLE pipeline ran over the whole file.
    let a = actual.replace(' ', "");
    assert!(
        a.contains("tag`abc`"),
        "tagged template did not round-trip: {actual}"
    );
    assert!(
        a.contains(",3)"),
        "sibling operand `1 + 2` did not fold to `3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "a pre-fold expression survived — did it fall back to WHITESPACE_ONLY? {actual}"
    );
}
