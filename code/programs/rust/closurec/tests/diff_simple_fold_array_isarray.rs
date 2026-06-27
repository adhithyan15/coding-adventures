//! Integration test for the `tests/diff/simple-fold-array-isarray/` fixture.
//!
//! End-to-end oracle for static `Array.isArray` folding in
//! `closure-pass-constant-fold`: a call whose single argument is a literal with
//! no side effect to drop collapses to the boolean V8 would produce (ECMAScript
//! §22.1.2.2). A non-empty array/object literal is left intact (its elements
//! might have side effects).
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=true;var b=false;var c=false;var d=false;var e=false;var f=Array.isArray([1,2]);report(a,b,c,d,e,f);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-array-isarray/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_array_isarray_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-array-isarray/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each foldable call collapses to its boolean — `true` only for the empty array
/// literal, `false` for the object and primitive literals — while the non-empty
/// `[1,2]` survives untouched (folding it would drop its element evaluation).
#[test]
fn simple_fold_array_isarray_folds_to_booleans() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=true"), "Array.isArray([]) → true; got:\n{actual}");
    assert!(actual.contains("b=false"), "Array.isArray({{}}) → false; got:\n{actual}");
    assert!(actual.contains("c=false"), "Array.isArray(\"x\") → false; got:\n{actual}");
    assert!(actual.contains("d=false"), "Array.isArray(42) → false; got:\n{actual}");
    assert!(actual.contains("e=false"), "Array.isArray(null) → false; got:\n{actual}");
    // The non-empty array literal is NOT folded — the call must remain.
    assert!(
        actual.contains("f=Array.isArray([1,2])"),
        "Array.isArray([1,2]) must NOT fold (element side effects); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly one
/// call — the declined non-empty `Array.isArray([1,2])` — may remain.
#[test]
fn simple_fold_array_isarray_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("Array.isArray(").count(),
        1,
        "exactly one Array.isArray call (the non-empty-array decline) should \
         remain — proving the typed SIMPLE optimizer ran, not the whitespace \
         fallback; got:\n{actual}",
    );
}
