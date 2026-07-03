//! Integration test for the `tests/diff/simple-fold-object-keys/` fixture.
//!
//! End-to-end oracle for static `Object.keys`/`values`/`entries` folding in
//! `closure-pass-constant-fold` (ECMAScript §20.1.2.16/.22/.5):
//!
//!   * a call whose single argument is an EMPTY object literal `{}` collapses to
//!     the empty array literal `[]` for all three methods; and
//!   * `Object.keys` of a NON-EMPTY static object literal collapses to the array
//!     of its own string keys: `Object.keys({a:1,b:2})` → `["a","b"]`.
//!
//! Declined and left intact: `Object.values` of a non-empty object (no non-empty
//! values fold yet), `Object.keys` of an integer-index-keyed object (indices
//! enumerate first, reordering the result), and `Object.keys` of an array.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=[];var b=[];var c=[];var d=["a","b"];var e=Object.values({a:1});var f=Object.keys({1:"x"});var g=Object.keys([]);report(a,b,c,d,e,f,g);
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

/// The empty-object `[]` folds AND the non-empty `Object.keys` key-array fold
/// fire; the three declined calls survive untouched.
#[test]
fn simple_fold_object_keys_folds_empty_and_nonempty() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    // Empty-object folds → [] for all three methods.
    assert!(actual.contains("a=[]"), "Object.keys({{}}) → []; got:\n{actual}");
    assert!(actual.contains("b=[]"), "Object.values({{}}) → []; got:\n{actual}");
    assert!(actual.contains("c=[]"), "Object.entries({{}}) → []; got:\n{actual}");
    // Non-empty Object.keys → array of its string keys.
    assert!(
        actual.contains(r#"d=["a","b"]"#),
        "Object.keys({{a:1,b:2}}) → [\"a\",\"b\"]; got:\n{actual}"
    );
    // The three declines survive: non-empty values, integer-index keys, array.
    assert!(
        actual.contains("e=Object.values({a:1})"),
        "Object.values({{a:1}}) must NOT fold (no non-empty values fold); got:\n{actual}"
    );
    assert!(
        actual.contains(r#"f=Object.keys({1:"x"})"#),
        "Object.keys({{1:\"x\"}}) must NOT fold (integer-index reorders); got:\n{actual}"
    );
    assert!(
        actual.contains("g=Object.keys([])"),
        "Object.keys([]) must NOT fold (array declined); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly
/// three calls — the non-empty-values, integer-index, and array declines — may
/// remain.
#[test]
fn simple_fold_object_keys_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("Object.").count(),
        3,
        "exactly three Object. calls (the values + integer-index + array declines) \
         should remain — proving the typed SIMPLE optimizer ran, not the whitespace \
         fallback; got:\n{actual}",
    );
}
