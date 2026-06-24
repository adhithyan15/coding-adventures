//! Integration test for the `tests/diff/simple-fold-parseint/` fixture.
//!
//! End-to-end oracle for global `parseInt` / `parseFloat` folding in
//! `closure-pass-constant-fold`: a `parseInt(lit[, radix])` or `parseFloat(lit)`
//! call whose first argument is a string literal collapses to the numeric
//! literal V8 would produce at runtime (ECMAScript §19.2.5 / §19.2.4).
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=12;var b=255;var c=3.14;var d=31;report(a,b,c,d);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-parseint/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_parseint_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-parseint/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `parseInt`/`parseFloat` on string literals fold to numeric literals — no
/// call survives, and the radix and `0x` prefix are honoured.
#[test]
fn simple_fold_parseint_folds_to_numeric_literals() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // parseInt("12px") → 12, parseInt("FF",16) → 255,
    // parseFloat("3.14abc") → 3.14, parseInt("0x1F") → 31.
    assert!(actual.contains("a=12"), "parseInt(\"12px\") → 12; got:\n{actual}");
    assert!(actual.contains("b=255"), "parseInt(\"FF\",16) → 255; got:\n{actual}");
    assert!(actual.contains("c=3.14"), "parseFloat(\"3.14abc\") → 3.14; got:\n{actual}");
    assert!(actual.contains("d=31"), "parseInt(\"0x1F\") → 31; got:\n{actual}");
    for f in ["parseInt", "parseFloat"] {
        assert!(
            !actual.contains(f),
            "no `{f}` call should remain after folding; got:\n{actual}",
        );
    }
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave the calls intact).
#[test]
fn simple_fold_parseint_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        !actual.contains("parseInt(") && !actual.contains("parseFloat("),
        "expected the calls to be folded away by the typed pipeline \
         (proving this is the SIMPLE optimizer, not the whitespace fallback); \
         got:\n{actual}",
    );
}
