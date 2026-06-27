//! Integration test for the `tests/diff/simple-fold-uri/` fixture.
//!
//! End-to-end oracle for global `encodeURI` / `decodeURI` folding in
//! `closure-pass-constant-fold`: a call whose single argument is a string
//! literal collapses to the string literal V8 would produce, and a `decodeURI`
//! whose input would throw a `URIError` is left intact (ECMAScript §19.2.6.4 /
//! §19.2.6.2). These are the whole-URI siblings of `encodeURIComponent` /
//! `decodeURIComponent`; the fixture pins down the one behavioural difference —
//! reserved delimiters (`; , / ? : @ & = + $ #`) are kept unescaped by
//! `encodeURI` and kept ENCODED by `decodeURI`.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a="a%20b";var b="a/b?c=d";var c="%C3%A9";var d="a b";var e="%2F";var f="\u00e9";var g=decodeURI("%E0");report(a,b,c,d,e,f,g);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-uri/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_uri_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-uri/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// The foldable calls collapse to string literals — `encodeURI` keeps the
/// reserved delimiters intact, `decodeURI` keeps a reserved escape ENCODED —
/// while the `URIError` input survives untouched.
#[test]
fn simple_fold_uri_folds_to_string_literals() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains(r#"a="a%20b""#), "encodeURI(\"a b\") → \"a%20b\"; got:\n{actual}");
    assert!(actual.contains(r#"b="a/b?c=d""#), "encodeURI(\"a/b?c=d\") → \"a/b?c=d\" (reserved kept); got:\n{actual}");
    assert!(actual.contains(r#"c="%C3%A9""#), "encodeURI(\"é\") → \"%C3%A9\"; got:\n{actual}");
    assert!(actual.contains(r#"d="a b""#), "decodeURI(\"a%20b\") → \"a b\"; got:\n{actual}");
    assert!(actual.contains(r#"e="%2F""#), "decodeURI(\"%2F\") → \"%2F\" (reserved escape kept); got:\n{actual}");
    assert!(actual.contains(r##"f="\u00e9""##), "decodeURI(\"%C3%A9\") -> \"é\" emitted as \\u00e9; got:\n{actual}");
    // The URIError input is NOT folded — the call must remain.
    assert!(
        actual.contains(r#"g=decodeURI("%E0")"#),
        "decodeURI(\"%E0\") must NOT fold (truncated multi-byte → URIError); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly one
/// call — the declined `decodeURI("%E0")` — may remain, and no `encodeURI` call
/// may survive.
#[test]
fn simple_fold_uri_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("encodeURI(").count(),
        0,
        "every encodeURI call should fold; got:\n{actual}",
    );
    assert_eq!(
        actual.matches("decodeURI(").count(),
        1,
        "exactly one decodeURI call (the URIError decline) should remain — \
         proving the typed SIMPLE optimizer ran, not the whitespace fallback; \
         got:\n{actual}",
    );
}
