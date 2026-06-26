//! Integration test for the `tests/diff/simple-fold-fromcharcode/` fixture.
//!
//! End-to-end oracle for the static `String.fromCharCode(...)` fold in
//! `closure-pass-constant-fold` — building a string from UTF-16 code units
//! (ECMAScript §22.1.2.1). It is the first fold whose receiver is the bare
//! global `String` rather than a string/number literal. The fixture covers two
//! BMP units, a surrogate PAIR assembling an astral scalar, and the no-arg
//! empty case:
//!
//! ```text
//! var a="HI";var b="💩";var c="";report(a,b,c);
//! ```
//!
//! - `String.fromCharCode(72, 73)`        → `"HI"`;
//! - `String.fromCharCode(0xD83D, 0xDCA9)`→ `"💩"` (emitted escaped as the pair);
//! - `String.fromCharCode()`              → `""`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-fromcharcode/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_fromcharcode_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-fromcharcode/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `String.fromCharCode(...)` on integer-literal args folds to string literals —
/// no `fromCharCode(` call survives — and the BMP / surrogate-pair / empty
/// cases hold.
#[test]
fn simple_fold_fromcharcode_folds_to_strings() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // String.fromCharCode(72,73) → "HI".
    assert!(
        actual.contains("a=\"HI\""),
        "String.fromCharCode(72,73) → \"HI\"; got:\n{actual}"
    );
    // String.fromCharCode(0xD83D,0xDCA9) → "💩", emitted as the escaped pair.
    assert!(
        actual.contains("b=\"\\ud83d\\udca9\""),
        "surrogate-pair fromCharCode → 💩 (escaped pair); got:\n{actual}"
    );
    // String.fromCharCode() → "".
    assert!(
        actual.contains("c=\"\""),
        "String.fromCharCode() → \"\"; got:\n{actual}"
    );
    assert!(
        !actual.contains("fromCharCode"),
        "no `fromCharCode` call should remain after folding; got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave the calls intact).
#[test]
fn simple_fold_fromcharcode_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("fromCharCode"),
        "expected the calls to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
