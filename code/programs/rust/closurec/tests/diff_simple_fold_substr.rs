//! Integration test for the `tests/diff/simple-fold-substr/` fixture.
//!
//! End-to-end oracle for the legacy `String.prototype.substr(start[, length])`
//! folding in `closure-pass-constant-fold`. Unlike `slice`/`substring`,
//! `substr`'s second argument is a *length*, not an end index: a negative start
//! counts from the end (then clamps to 0) and the length clamps into
//! `[0, len - start]`. The fixture is chosen so all those rules are observable:
//!
//! ```text
//! var a="bc";var b="bcde";var c="de";var d="";report(a,b,c,d);
//! ```
//!
//! - `"abcde".substr(1, 2)` → `"bc"` — start at 1, take 2;
//! - `"abcde".substr(1)`    → `"bcde"` — length defaults to the rest;
//! - `"abcde".substr(-2)`   → `"de"` — negative start counts from the end;
//! - `"abcde".substr(10)`   → `""` — start past the end.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-substr/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_substr_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-substr/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `substr` on string literals folds away — no `.substr(` call survives, and
/// the length-counting (`1,2`→"bc"), default-rest (`1`→"bcde"), and
/// negative-from-end (`-2`→"de") rules all hold.
#[test]
fn simple_fold_substr_counts_length_from_start() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // "abcde".substr(1,2) → "bc" (start 1, take 2).
    assert!(
        actual.contains(r#"a="bc""#),
        "\"abcde\".substr(1,2) → \"bc\"; got:\n{actual}"
    );
    // "abcde".substr(1) → "bcde" (length defaults to the rest).
    assert!(
        actual.contains(r#"b="bcde""#),
        "\"abcde\".substr(1) → \"bcde\" (length defaults to rest); got:\n{actual}"
    );
    // "abcde".substr(-2) → "de" (negative start counts from end).
    assert!(
        actual.contains(r#"c="de""#),
        "\"abcde\".substr(-2) → \"de\" (negative start from end); got:\n{actual}"
    );
    // "abcde".substr(10) → "" (start past the end).
    assert!(
        actual.contains(r#"d="""#),
        "\"abcde\".substr(10) → \"\"; got:\n{actual}"
    );
    assert!(
        !actual.contains(".substr("),
        "no `.substr(` call should remain after folding; got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave the calls intact).
#[test]
fn simple_fold_substr_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains(".substr("),
        "expected the calls to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
