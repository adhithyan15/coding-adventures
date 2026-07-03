//! Integration test for the `tests/diff/simple-fold-json-stringify/` fixture.
//!
//! End-to-end oracle for static `JSON.stringify` folding in
//! `closure-pass-constant-fold`: a call whose single argument is a primitive
//! literal (number / boolean / null) collapses to the string literal V8 would
//! produce (ECMAScript §25.5.2). A string argument (JSON escaping) and a
//! fractional/large number are left intact.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a="42";var b="-7";var c="true";var d="null";var e=JSON.stringify("x");var f=JSON.stringify(3.5);report(a,b,c,d,e,f);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-json-stringify/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_json_stringify_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-json-stringify/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each foldable call collapses to its JSON text AS a string literal — the
/// number/boolean/null cases — while the string argument (escaping) and the
/// fractional number survive untouched.
#[test]
fn simple_fold_json_stringify_folds_to_string_literals() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains(r#"a="42""#), "JSON.stringify(42) → \"42\"; got:\n{actual}");
    assert!(actual.contains(r#"b="-7""#), "JSON.stringify(-7) → \"-7\"; got:\n{actual}");
    assert!(actual.contains(r#"c="true""#), "JSON.stringify(true) → \"true\"; got:\n{actual}");
    assert!(actual.contains(r#"d="null""#), "JSON.stringify(null) → \"null\"; got:\n{actual}");
    // The string and fractional inputs are NOT folded — the calls must remain.
    assert!(
        actual.contains(r#"e=JSON.stringify("x")"#),
        "JSON.stringify(\"x\") must NOT fold (escaping); got:\n{actual}"
    );
    assert!(
        actual.contains("f=JSON.stringify(3.5)"),
        "JSON.stringify(3.5) must NOT fold (fractional); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly two
/// calls — the declined string and fractional `JSON.stringify` — may remain.
#[test]
fn simple_fold_json_stringify_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("JSON.stringify(").count(),
        2,
        "exactly two JSON.stringify calls (the string + fractional declines) \
         should remain — proving the typed SIMPLE optimizer ran, not the \
         whitespace fallback; got:\n{actual}",
    );
}
