//! Integration test for the `tests/diff/simple-fold-escape/` fixture.
//!
//! End-to-end oracle for the legacy global `escape` / `unescape` folding in
//! `closure-pass-constant-fold`: a call whose single argument is a string
//! literal collapses to the string literal V8 would produce (ECMAScript Annex B
//! §B.2.1.1 / §B.2.1.2), and an `unescape` whose result would be an unpaired
//! surrogate is left intact. These are the legacy siblings of
//! `encodeURIComponent` / `decodeURIComponent`; the fixture pins down the
//! structural difference — `escape`/`unescape` work on UTF-16 CODE UNITS, so a
//! unit `< 0x100` escapes to `%XX` and a unit `>= 0x100` to `%uXXXX`.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a="a%20b";var b="%7E/@";var c="%E9";var d="%uD83D%uDE00";var e="a b";var f="\u00e9";var g="/";var h=unescape("%uD83D");report(a,b,c,d,e,f,g,h);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-escape/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_escape_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-escape/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// The foldable calls collapse to string literals — `escape` percent-encodes on
/// UTF-16 code units (so `~` escapes but `/`/`@` stay, and `😀` becomes a
/// surrogate pair), `unescape` is the inverse — while the unpaired-surrogate
/// `unescape` survives untouched.
#[test]
fn simple_fold_escape_folds_to_string_literals() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains(r#"a="a%20b""#), "escape(\"a b\") → \"a%20b\"; got:\n{actual}");
    assert!(actual.contains(r#"b="%7E/@""#), "escape(\"~/@\") → \"%7E/@\" (~ escaped, /@ kept); got:\n{actual}");
    assert!(actual.contains(r#"c="%E9""#), "escape(\"é\") → \"%E9\" (code unit < 0x100); got:\n{actual}");
    assert!(actual.contains(r#"d="%uD83D%uDE00""#), "escape(\"😀\") → \"%uD83D%uDE00\" (surrogate pair); got:\n{actual}");
    assert!(actual.contains(r#"e="a b""#), "unescape(\"a%20b\") → \"a b\"; got:\n{actual}");
    assert!(actual.contains(r##"f="\u00e9""##), "unescape(\"%E9\") -> \"é\" emitted as \\u00e9; got:\n{actual}");
    assert!(actual.contains(r#"g="/""#), "unescape(\"%2F\") → \"/\" (every escape decodes); got:\n{actual}");
    // The unpaired-surrogate input is NOT folded — the call must remain.
    assert!(
        actual.contains(r#"h=unescape("%uD83D")"#),
        "unescape(\"%uD83D\") must NOT fold (lone high surrogate); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly one
/// call — the declined `unescape("%uD83D")` — may remain, and no `escape` call
/// may survive.
#[test]
fn simple_fold_escape_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // `unescape(` contains the substring `escape(`, so count bare `escape(`
    // occurrences by excluding the `un` prefix: every `escape(` that is NOT
    // preceded by `un` must have folded away.
    let total_escape = actual.matches("escape(").count();
    let unescape = actual.matches("unescape(").count();
    assert_eq!(
        total_escape - unescape,
        0,
        "every bare escape() call should fold; got:\n{actual}",
    );
    assert_eq!(
        unescape,
        1,
        "exactly one unescape() call (the unpaired-surrogate decline) should remain — \
         proving the typed SIMPLE optimizer ran, not the whitespace fallback; \
         got:\n{actual}",
    );
}
