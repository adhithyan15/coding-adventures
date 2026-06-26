//! Integration test for the `tests/diff/simple-fold-codepointat/` fixture.
//!
//! End-to-end oracle for `String.prototype.codePointAt(i)` folding in
//! `closure-pass-constant-fold` — the Unicode code POINT starting at UTF-16
//! code unit `i` (ECMAScript §22.1.3.4). Unlike the already-folded
//! `charCodeAt` (a single 16-bit code unit), `codePointAt` combines a leading
//! high surrogate with the following low surrogate into one astral code point.
//! The fixture covers the BMP path, surrogate-pair combination, and a lone low
//! surrogate:
//!
//! ```text
//! var a=97;var b=128169;var c=56489;report(a,b,c);
//! ```
//!
//! - `"a💩b".codePointAt(0)` → `97`     — the BMP `a` (same as `charCodeAt`);
//! - `"a💩b".codePointAt(1)` → `128169` — the pair `[0xD83D,0xDCA9]` = `U+1F4A9`;
//! - `"💩".codePointAt(1)`   → `56489`  — the lone trailing low surrogate.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-codepointat/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_codepointat_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-codepointat/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `codePointAt` on string literals folds to numerics — no `.codePointAt(`
/// call survives — and the BMP / surrogate-pair / lone-low-surrogate rules hold.
#[test]
fn simple_fold_codepointat_folds_to_numbers() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // "a💩b".codePointAt(0) → 97 (BMP 'a').
    assert!(
        actual.contains("a=97"),
        "\"a💩b\".codePointAt(0) → 97; got:\n{actual}"
    );
    // "a💩b".codePointAt(1) → 128169 (combines the surrogate pair into U+1F4A9).
    assert!(
        actual.contains("b=128169"),
        "\"a💩b\".codePointAt(1) → 128169 (astral code point); got:\n{actual}"
    );
    // "💩".codePointAt(1) → 56489 (the lone trailing low surrogate 0xDCA9).
    assert!(
        actual.contains("c=56489"),
        "\"💩\".codePointAt(1) → 56489 (lone low surrogate); got:\n{actual}"
    );
    assert!(
        !actual.contains("codePointAt"),
        "no `codePointAt` call should remain after folding; got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave the calls intact).
#[test]
fn simple_fold_codepointat_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("codePointAt"),
        "expected the calls to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
