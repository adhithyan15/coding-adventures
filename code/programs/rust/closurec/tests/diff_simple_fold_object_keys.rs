//! Integration test for the `tests/diff/simple-fold-object-keys/` fixture.
//!
//! End-to-end oracle for static `Object.keys`/`values`/`entries` folding in
//! `closure-pass-constant-fold`: a call whose single argument is an EMPTY object
//! literal `{}` collapses to the empty array literal `[]` V8 would produce
//! (ECMAScript §20.1.2.16/.22/.5). A non-empty object and an array are left
//! intact.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=[];var b=[];var c=[];var d=Object.keys({a:1});var e=Object.keys([]);report(a,b,c,d,e);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-object-keys/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_object_keys_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-object-keys/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// `Object.keys/values/entries({})` each collapse to `[]`, while the non-empty
/// object and the array argument survive untouched.
#[test]
fn simple_fold_object_keys_folds_empty_object_to_empty_array() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=[]"), "Object.keys({{}}) → []; got:\n{actual}");
    assert!(actual.contains("b=[]"), "Object.values({{}}) → []; got:\n{actual}");
    assert!(actual.contains("c=[]"), "Object.entries({{}}) → []; got:\n{actual}");
    // The non-empty object and the array are NOT folded — the calls must remain.
    assert!(
        actual.contains("d=Object.keys({a:1})"),
        "Object.keys({{a:1}}) must NOT fold (property side effects); got:\n{actual}"
    );
    assert!(
        actual.contains("e=Object.keys([])"),
        "Object.keys([]) must NOT fold (array declined); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly two
/// calls — the declined non-empty object and the array — may remain.
#[test]
fn simple_fold_object_keys_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("Object.").count(),
        2,
        "exactly two Object. calls (the non-empty-object + array declines) should \
         remain — proving the typed SIMPLE optimizer ran, not the whitespace \
         fallback; got:\n{actual}",
    );
}
