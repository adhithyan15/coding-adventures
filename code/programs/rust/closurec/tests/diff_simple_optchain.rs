//! Integration test for the `tests/diff/simple-optchain/` fixture.
//!
//! Exercises CLOC12.171 PR2 — **optional chaining** `a?.b` (ES2020) now flows
//! through the full SIMPLE pipeline (parser → typed-AST bridge → passes →
//! emitter) instead of falling through the bridge's `OptionalChainExpression`
//! decline arm and dragging the whole file to WHITESPACE_ONLY
//! (gap-OptionalChain, now closed).
//!
//! The fixture is `f(a?.b, 1 + 2);` — a retained call (an unknown `f(...)` has
//! side effects, so DCE keeps it) whose arguments are an optional member access
//! `a?.b` and the foldable `1 + 2`. Two facts prove the pipeline ran
//! end-to-end rather than falling back:
//!   1. `a?.b` round-trips — the bridge produced a real `ChainExpression`
//!      wrapping an `OptionalMemberExpression` (rather than declining), and the
//!      printer re-emitted the `?.` — so the `?.` link survived, and
//!   2. the sibling argument `1 + 2` folds to `3` — proving the SIMPLE passes
//!      walked *through* the call arguments, not a verbatim WHITESPACE_ONLY
//!      re-emit.
//! A WHITESPACE_ONLY fallback — which a bridge failure would force for the
//! *whole file* — would instead re-emit the source verbatim, leaving `1 + 2`
//! unfolded (`f(a?.b, 1 + 2)`).

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-optchain/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_optchain_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-optchain/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture. Strip spaces so the checks are
    // insensitive to inter-token whitespace.
    let a = actual.replace(' ', "");
    // (1) the optional link `a?.b` survived — proving the bridge converted it to
    //     a real ChainExpression/OptionalMemberExpression rather than dropping
    //     the file to WHITESPACE_ONLY.
    assert!(
        a.contains("a?.b"),
        "optional chain `a?.b` did not round-trip: {actual}"
    );
    // (2) the sibling argument folded — proving the SIMPLE pipeline walked into
    //     the call arguments (`1 + 2` → `3`), not a verbatim WHITESPACE_ONLY
    //     pass.
    assert!(
        a.contains(",3)"),
        "argument `1 + 2` did not fold to `3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded `1+2` present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
