//! Integration test for the `tests/diff/simple-fold-fromcodepoint/` fixture.
//!
//! End-to-end oracle for the static `String.fromCodePoint(...)` fold in
//! `closure-pass-constant-fold` — building a string from Unicode CODE POINTS
//! (ECMAScript §22.1.2.2). Unlike `fromCharCode` (UTF-16 units), each argument
//! is a whole code point, so a single astral argument suffices. The fixture
//! covers two BMP scalars, a single astral scalar, and astral+BMP:
//!
//! ```text
//! var a="HI";var b="💩";var c="💩A";report(a,b,c);
//! ```
//!
//! - `String.fromCodePoint(72, 73)`       → `"HI"`;
//! - `String.fromCodePoint(128169)`       → `"💩"` (U+1F4A9, emitted escaped);
//! - `String.fromCodePoint(128169, 65)`   → `"💩A"`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-fromcodepoint/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_fromcodepoint_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-fromcodepoint/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `String.fromCodePoint(...)` on integer-literal args folds to string literals
/// — no `fromCodePoint(` call survives — and the BMP / single-astral / astral+BMP
/// cases hold.
#[test]
fn simple_fold_fromcodepoint_folds_to_strings() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // String.fromCodePoint(72,73) → "HI".
    assert!(
        actual.contains("a=\"HI\""),
        "String.fromCodePoint(72,73) → \"HI\"; got:\n{actual}"
    );
    // String.fromCodePoint(128169) → "💩" (a SINGLE astral arg), escaped pair.
    assert!(
        actual.contains("b=\"\\ud83d\\udca9\""),
        "single-astral fromCodePoint → 💩 (escaped pair); got:\n{actual}"
    );
    // String.fromCodePoint(128169, 65) → "💩A".
    assert!(
        actual.contains("c=\"\\ud83d\\udca9A\""),
        "fromCodePoint(128169,65) → 💩A; got:\n{actual}"
    );
    assert!(
        !actual.contains("fromCodePoint"),
        "no `fromCodePoint` call should remain after folding; got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave the calls intact).
#[test]
fn simple_fold_fromcodepoint_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("fromCodePoint"),
        "expected the calls to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
