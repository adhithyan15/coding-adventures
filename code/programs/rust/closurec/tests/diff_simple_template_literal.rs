//! Integration test for the `tests/diff/simple-template-literal/` fixture.
//!
//! Exercises CLOC12.155 (a **no-substitution template literal** `` `hello` ``
//! flows through the full SIMPLE pipeline: parser → typed-AST bridge → passes →
//! emitter) *and* CLOC12.197 (constant-fold collapses a no-sub template to a
//! plain string literal, matching the reference Closure Compiler which folds
//! `` `hello` `` → `"hello"`).
//!
//! The fixture puts the template in a call argument next to a foldable `1 + 2`.
//! Two facts prove the pipeline ran end-to-end:
//!   1. the no-sub template **folds** to a string literal (`"hello"`), and
//!   2. `1 + 2` folds to `3`.
//! A WHITESPACE_ONLY fallback — which a bridge decline would force — would
//! instead re-emit the source verbatim, leaving `` `hello` `` and `1 + 2` intact.
//!
//! *Substitution* templates (`` `a${x}b` ``) still don't parse in the grammar
//! (so the CLOC12.197 fold, though correct, can't fire on them end-to-end yet),
//! and *tagged* templates remain Phase 3 — neither appears here (see the
//! CLOC12-gaps §CLOC12.155 note).

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-template-literal/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_template_literal_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-template-literal/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture: the no-sub template FOLDED to a string
    // literal (CLOC12.197) AND the adjacent `1 + 2` folded — proving the SIMPLE
    // pipeline ran rather than falling back to WHITESPACE_ONLY.
    let a = actual.replace(' ', "");
    assert!(a.contains("\"hello\""), "no-sub template did not fold to a string literal: {actual}");
    assert!(!a.contains("`hello`"), "template survived as a backtick — CLOC12.197 fold did not fire: {actual}");
    assert!(a.contains(",3)"), "1+2 not folded — did it fall back to WHITESPACE_ONLY? {actual}");
    assert!(!a.contains("1+2"), "a pre-fold expression survived: {actual}");
}
