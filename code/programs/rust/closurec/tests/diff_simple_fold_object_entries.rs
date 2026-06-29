//! Integration test for the `tests/diff/simple-fold-object-entries/` fixture.
//!
//! End-to-end oracle for static `Object.entries(...)` folding in
//! `closure-pass-constant-fold`: `Object.entries({k: v, …})` collapses to the
//! array of `[key, value]` pair literals `[["k", v], …]` (ECMAScript §20.1.2.5)
//! when every property is a plain data property with a primitive-literal value.
//! An integer-index key (`{1: "x"}`, which would reorder), a `__proto__` key
//! (the prototype setter, not an own property), and a non-global receiver
//! (`o.entries(...)`) are all declined.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=[["a",1],["b",2]];var b=[["x","hi"]];var c=[];var d=Object.entries({1:"x"});var e=Object.entries({__proto__:1});var f=o.entries({a:1});report(a,b,c,d,e,f);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-object-entries/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_object_entries_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-object-entries/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each bare-global `Object.entries({…})` of static data properties folds to the
/// matching array of `[key, value]` pairs; the empty object is `[]`.
#[test]
fn simple_fold_object_entries_folds_to_pairs() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        actual.contains(r#"a=[["a",1],["b",2]]"#),
        "entries({{a:1,b:2}}) → [[\"a\",1],[\"b\",2]]; got:\n{actual}"
    );
    assert!(
        actual.contains(r#"b=[["x","hi"]]"#),
        "entries({{x:\"hi\"}}) → [[\"x\",\"hi\"]]; got:\n{actual}"
    );
    assert!(
        actual.contains("c=[]"),
        "entries({{}}) → [] (empty case); got:\n{actual}"
    );
    // Integer-index key: declined (would reorder ahead of string keys).
    assert!(
        actual.contains(r#"d=Object.entries({1:"x"})"#),
        "entries({{1:\"x\"}}) must NOT fold (integer-index key); got:\n{actual}"
    );
    // __proto__ key: declined (prototype setter, not an own property).
    assert!(
        actual.contains(r#"e=Object.entries({__proto__:1})"#),
        "entries({{__proto__:1}}) must NOT fold (prototype setter); got:\n{actual}"
    );
    // Non-global receiver: declined.
    assert!(
        actual.contains(r#"f=o.entries({a:1})"#),
        "o.entries(...) must NOT fold (only the bare global Object); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly
/// three `entries(` calls survive — the two declined Object.entries (integer
/// index and __proto__) and the non-global receiver — while the foldable calls
/// collapse. (Under the whitespace fallback all six would remain.)
#[test]
fn simple_fold_object_entries_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("entries(").count(),
        3,
        "exactly three entries( calls (two declined Object.entries + the \
         non-global receiver) should remain — proving the typed SIMPLE optimizer \
         ran, not the whitespace fallback; got:\n{actual}",
    );
}
