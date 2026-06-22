//! Integration test for the `tests/diff/simple-asi-restricted/` fixture.
//!
//! End-to-end oracle for **ASI Phase 3 — restricted productions** (ECMAScript
//! §12.10.1). A line terminator is not allowed between `return` and its
//! argument, so
//!
//! ```text
//! function f(){return
//! 42}
//! ```
//!
//! is `function f(){ return; 42; }` (empty return + a now-dead expression
//! statement), NOT `return 42`. closurec's grammar is newline-blind, so without
//! the Phase-3 pre-pass it would parse `return 42` and re-emit it — a silent
//! miscompile. At SIMPLE the fixture optimizes to:
//!
//! ```text
//! function f(){return};report(f());
//! ```
//!
//! The dead `42` is gone, which is the double proof: it can only disappear if
//! (a) the restricted production was honored and (b) the SIMPLE *typed* pipeline
//! ran — the `WHITESPACE_ONLY` re-stitcher would instead emit
//! `function f(){return 42}`, the very miscompile this guards against.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-asi-restricted/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_asi_restricted_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-asi-restricted/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// The restricted production must be honored: `return` does NOT swallow the `42`
/// on the next line, so the dead expression is dropped and `42` is absent.
#[test]
fn simple_asi_restricted_does_not_merge_return_with_next_line() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("42"),
        "the dead `42` after `return` must be dropped (restricted production \
         honored, typed pipeline ran); got:\n{actual}",
    );
    assert!(
        !actual.contains("return 42"),
        "`return 42` would be the miscompile this fixture guards against; \
         got:\n{actual}",
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which re-stitches `return 42` and cannot drop the
/// dead `42`).
#[test]
fn simple_asi_restricted_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // The typed pipeline removed the dead statement; the whitespace fallback
    // never can (it only strips whitespace), so its presence proves the typed
    // path. `report(f())` survives because it has an observable effect.
    assert!(
        actual.contains("report(f())"),
        "the live call must survive; got:\n{actual}",
    );
    assert!(
        !actual.contains("return 42"),
        "expected the typed pipeline to split `return` from `42`, proving this \
         is the SIMPLE optimizer, not the whitespace fallback; got:\n{actual}",
    );
}
