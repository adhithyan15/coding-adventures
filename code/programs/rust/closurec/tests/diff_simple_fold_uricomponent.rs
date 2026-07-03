//! Integration test for the `tests/diff/simple-fold-uricomponent/` fixture.
//!
//! End-to-end oracle for global `encodeURIComponent` / `decodeURIComponent`
//! folding in `closure-pass-constant-fold`: a call whose single argument is a
//! string literal collapses to the string literal V8 would produce, and a
//! `decodeURIComponent` whose input would throw a `URIError` is left intact
//! (ECMAScript §19.2.6.5 / §19.2.6.3).
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a="a%20b";var b="%C3%A9";var c="%2F";var d="a b";var e="\u00e9";var f=decodeURIComponent("%E0");report(a,b,c,d,e,f);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-uricomponent/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_uricomponent_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-uricomponent/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// The foldable calls collapse to string literals — `encodeURIComponent`
/// percent-escapes (reserved delimiters included), `decodeURIComponent`
/// reverses it — while the `URIError` input survives untouched.
#[test]
fn simple_fold_uricomponent_folds_to_string_literals() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains(r#"a="a%20b""#), "encodeURIComponent(\"a b\") → \"a%20b\"; got:\n{actual}");
    assert!(actual.contains(r#"b="%C3%A9""#), "encodeURIComponent(\"é\") → \"%C3%A9\"; got:\n{actual}");
    assert!(actual.contains(r#"c="%2F""#), "encodeURIComponent(\"/\") → \"%2F\" (reserved escaped); got:\n{actual}");
    assert!(actual.contains(r#"d="a b""#), "decodeURIComponent(\"a%20b\") → \"a b\"; got:\n{actual}");
    assert!(actual.contains(r#"e="\u00e9""#), "decodeURIComponent(\"%C3%A9\") -> \"é\" emitted as \\u00e9; got:\n{actual}");
    // The URIError input is NOT folded — the call must remain.
    assert!(
        actual.contains(r#"f=decodeURIComponent("%E0")"#),
        "decodeURIComponent(\"%E0\") must NOT fold (truncated multi-byte → URIError); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly one
/// call — the declined `decodeURIComponent("%E0")` — may remain.
#[test]
fn simple_fold_uricomponent_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("encodeURIComponent(").count(),
        0,
        "every encodeURIComponent call should fold; got:\n{actual}",
    );
    assert_eq!(
        actual.matches("decodeURIComponent(").count(),
        1,
        "exactly one decodeURIComponent call (the URIError decline) should remain — \
         proving the typed SIMPLE optimizer ran, not the whitespace fallback; \
         got:\n{actual}",
    );
}
