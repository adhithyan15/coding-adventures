//! Integration test for the `tests/diff/simple-fold-number/` fixture.
//!
//! End-to-end oracle for global `Number("…")` folding in
//! `closure-pass-constant-fold`: a `Number(lit)` call whose single argument is a
//! string literal collapses to the numeric literal V8 would produce at runtime
//! (ECMAScript §21.1.1.1 → §7.1.4.1.1 `StringToNumber`). Unlike
//! `parseInt`/`parseFloat` the coercion is **total** — a string with any
//! trailing garbage is `NaN` and so declines.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=42;var b=0;var c=3.5;var d=31;var e=5;var f=15;var g=Number("abc");report(a,b,c,d,e,f,g);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-number/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_number_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-number/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// The foldable `Number(...)` calls collapse to numeric literals — decimal,
/// the empty string (→ `0`), trimmed decimal, and the hex/binary/octal forms —
/// while `Number("abc")` (a `NaN` result) is left intact.
#[test]
fn simple_fold_number_folds_to_numeric_literals() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=42"), "Number(\"42\") → 42; got:\n{actual}");
    assert!(actual.contains("b=0"), "Number(\"\") → 0; got:\n{actual}");
    assert!(actual.contains("c=3.5"), "Number(\"  3.5 \") → 3.5; got:\n{actual}");
    assert!(actual.contains("d=31"), "Number(\"0x1F\") → 31; got:\n{actual}");
    assert!(actual.contains("e=5"), "Number(\"0b101\") → 5; got:\n{actual}");
    assert!(actual.contains("f=15"), "Number(\"0o17\") → 15; got:\n{actual}");
    // `Number("abc")` is NaN — no literal token, so the call must survive.
    assert!(
        actual.contains("g=Number(\"abc\")"),
        "Number(\"abc\") is NaN and must NOT fold; got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly one
/// `Number(` call — the declined `Number("abc")` — may remain.
#[test]
fn simple_fold_number_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("Number(").count(),
        1,
        "exactly one Number( call (the NaN decline) should remain — proving the \
         typed SIMPLE optimizer ran, not the whitespace fallback; got:\n{actual}",
    );
}
