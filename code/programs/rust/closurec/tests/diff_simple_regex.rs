//! Integration test for the `tests/diff/simple-regex/` fixture.
//!
//! Exercises CLOC12.172 PR2 — a **regular-expression literal** (`/pat/flags`,
//! a `RegExpLiteral`) now flows through the full SIMPLE pipeline (parser →
//! typed-AST bridge → passes → emitter) instead of being mis-encoded at the
//! bridge as an `Identifier` whose name is the raw `/pat/flags` text
//! (gap-RegExpAsIdentifier, now closed).
//!
//! The fixture is `f(/ab+c/gi, 1 + 2);` — an unknown call `f(...)` (side
//! effects, so DCE keeps it) with two arguments: a regex literal `/ab+c/gi`
//! and the foldable `1 + 2`. Two facts prove the pipeline ran end-to-end
//! rather than falling back to WHITESPACE_ONLY:
//!   1. the regex `/ab+c/gi` round-trips verbatim (delimiters and both flags),
//!      proving the bridge built a real `RegExpLiteral` the emitter can print,
//!      and
//!   2. the second argument `1 + 2` folds to `3`, proving the passes ran.
//! A WHITESPACE_ONLY fallback — which a bridge decline would force for the
//! *whole file* — would instead re-emit the source verbatim, leaving `1 + 2`
//! unfolded (`f(/ab+c/gi, 1 + 2)`).

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-regex/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_regex_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-regex/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture. Strip spaces so the checks are
    // insensitive to inter-token whitespace.
    let a = actual.replace(' ', "");
    // (1) the regex literal round-tripped intact — proving the bridge built a
    //     real `RegExpLiteral` (delimiters + both flags) rather than declining
    //     to WHITESPACE_ONLY or mangling it into an identifier.
    assert!(
        a.contains("/ab+c/gi"),
        "regex literal did not round-trip: {actual}"
    );
    // (2) the sibling argument folded — proving the SIMPLE pipeline ran over
    //     the call (`1 + 2` → `3`), not a verbatim WHITESPACE_ONLY pass.
    assert!(
        a.contains("/ab+c/gi,3)"),
        "argument `1 + 2` did not fold to `3` beside the regex: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded `1+2` present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
