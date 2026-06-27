//! Integration test for the `tests/diff/simple-fold-number-parse/` fixture.
//!
//! End-to-end oracle for static `Number.parseInt` / `Number.parseFloat` folding
//! in `closure-pass-constant-fold`: a call whose single argument is a string
//! literal collapses to the numeric literal V8 would produce (ECMAScript
//! §21.1.2.12/.13). These are the same functions as the global
//! `parseInt`/`parseFloat`, so a `NaN`/`Infinity` result is left intact.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=12;var b=255;var c=31;var d=314;var e=Number.parseInt("");report(a,b,c,d,e);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-number-parse/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_number_parse_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-number-parse/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each foldable call collapses to its numeric value — including the explicit
/// radix and the `0x` prefix — while the `NaN`-producing `Number.parseInt("")`
/// survives untouched.
#[test]
fn simple_fold_number_parse_folds_to_numbers() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=12"), "Number.parseInt(\"12px\") → 12; got:\n{actual}");
    assert!(actual.contains("b=255"), "Number.parseInt(\"FF\", 16) → 255; got:\n{actual}");
    assert!(actual.contains("c=31"), "Number.parseInt(\"0x1F\") → 31; got:\n{actual}");
    assert!(actual.contains("d=314"), "Number.parseFloat(\"3.14e2abc\") → 314; got:\n{actual}");
    // The NaN input is NOT folded — the call must remain.
    assert!(
        actual.contains(r#"e=Number.parseInt("")"#),
        "Number.parseInt(\"\") must NOT fold (NaN); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly one
/// call — the declined `Number.parseInt("")` — may remain.
#[test]
fn simple_fold_number_parse_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("Number.parse").count(),
        1,
        "exactly one Number.parse call (the NaN decline) should remain — proving \
         the typed SIMPLE optimizer ran, not the whitespace fallback; got:\n{actual}",
    );
}
