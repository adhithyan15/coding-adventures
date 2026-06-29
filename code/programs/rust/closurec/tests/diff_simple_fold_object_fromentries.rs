//! Integration test for the `tests/diff/simple-fold-object-fromentries/` fixture.
//!
//! End-to-end oracle for static `Object.fromEntries(...)` folding in
//! `closure-pass-constant-fold`: `Object.fromEntries([[k, v], …])` collapses to
//! the object literal `{k: v, …}` (ECMAScript §20.1.2.7) when every pair is a
//! static `[key, value]` array literal with a string/numeric key and a
//! primitive-literal value. Identifier-name keys emit bare (`{a: 1}`), other
//! keys quoted (`{"1": "x"}`); a duplicate key keeps its first position but
//! takes its last value; a non-global receiver (`o.fromEntries(...)`) is
//! declined.
//!
//! A `__proto__` key is also declined: `Object.fromEntries([["__proto__", v]])`
//! makes an OWN "__proto__" property, but `{__proto__: v}` is the §B.3.1
//! prototype setter — folding would change semantics.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a={a:1,b:2};var b={"1":"x"};var c={a:2};var d={};var e=o.fromEntries([["a",1]]);var f=Object.fromEntries([["__proto__",1]]);report(a,b,c,d,e,f);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-object-fromentries/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_object_fromentries_fixture_matches_expected_stdout() {
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
    let expected =
        std::fs::read_to_string("tests/diff/simple-fold-object-fromentries/expected.stdout")
            .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each bare-global `Object.fromEntries(...)` folds to the matching object
/// literal; identifier keys emit bare, numeric keys ToString-then-quoted, a
/// duplicate key keeps its first position with its last value, and the empty
/// input is `{}`.
#[test]
fn simple_fold_object_fromentries_folds_to_object_literal() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        actual.contains("a={a:1,b:2}"),
        "fromEntries([[a,1],[b,2]]) → {{a:1,b:2}} (identifier keys); got:\n{actual}"
    );
    assert!(
        actual.contains(r#"b={"1":"x"}"#),
        "fromEntries([[1,\"x\"]]) → {{\"1\":\"x\"}} (numeric key ToString, quoted); got:\n{actual}"
    );
    assert!(
        actual.contains("c={a:2}"),
        "fromEntries([[a,1],[a,2]]) → {{a:2}} (duplicate key, last value wins); got:\n{actual}"
    );
    assert!(
        actual.contains("d={}"),
        "fromEntries([]) → {{}} (empty object); got:\n{actual}"
    );
    // The non-global receiver is NOT folded — the call must remain.
    assert!(
        actual.contains(r#"e=o.fromEntries([["a",1]])"#),
        "o.fromEntries(...) must NOT fold (only the bare global Object); got:\n{actual}"
    );
    // A `__proto__` key is NOT folded: `Object.fromEntries([["__proto__", v]])`
    // makes an OWN "__proto__" property, but the object literal `{__proto__: v}`
    // is the §B.3.1 prototype setter — different semantics, so the call remains.
    assert!(
        actual.contains(r#"f=Object.fromEntries([["__proto__",1]])"#),
        "__proto__ key must NOT fold (own-property vs prototype setter); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly two
/// `fromEntries(` calls survive — the declined non-global receiver and the
/// declined `__proto__` key — while the four foldable bare-global calls all fold
/// away. (Under the whitespace fallback all six would remain.)
#[test]
fn simple_fold_object_fromentries_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        actual.matches("fromEntries(").count(),
        2,
        "exactly two fromEntries( calls (the declined non-global receiver and the \
         declined __proto__ key) should remain — proving the typed SIMPLE optimizer \
         ran and folded the four foldable calls, not the whitespace fallback; \
         got:\n{actual}",
    );
}
