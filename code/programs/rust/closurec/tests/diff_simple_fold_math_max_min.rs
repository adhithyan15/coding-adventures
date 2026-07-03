//! Integration test for the `tests/diff/simple-fold-math-max-min/` fixture.
//!
//! End-to-end oracle for static `Math.max(...)` / `Math.min(...)` folding in
//! `closure-pass-constant-fold`: when every argument is a numeric literal the
//! call collapses to the largest / smallest as a numeric literal (ECMAScript
//! §21.3.2.24 / .25). A non-literal argument (`Math.max(1, x)`), the empty call
//! (`Math.max()` → -Infinity), and a non-global receiver (`m.max(1, 2)`) are all
//! declined.
//!
//! At SIMPLE the fixture optimizes to:
//!
//! ```text
//! var a=3;var b=1;var c=-1;var d=7;var e=Math.max(1,x);var f=Math.max();var g=m.max(1,2);report(a,b,c,d,e,f,g);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fold-math-max-min/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fold_math_max_min_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fold-math-max-min/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Each all-numeric-literal `Math.max`/`Math.min` folds to the literal result;
/// non-literal-arg, empty, and non-global-receiver calls are left intact.
#[test]
fn simple_fold_math_max_min_folds_to_numeric() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(actual.contains("a=3"), "max(1,2,3) → 3; got:\n{actual}");
    assert!(actual.contains("b=1"), "min(1,2,3) → 1; got:\n{actual}");
    assert!(actual.contains("c=-1"), "max(-5,-1) → -1; got:\n{actual}");
    assert!(actual.contains("d=7"), "max(7) → 7; got:\n{actual}");
    // Declined cases survive verbatim.
    assert!(
        actual.contains("e=Math.max(1,x)"),
        "Math.max(1,x) must NOT fold (non-literal arg); got:\n{actual}"
    );
    assert!(
        actual.contains("f=Math.max()"),
        "Math.max() must NOT fold (empty → -Infinity); got:\n{actual}"
    );
    assert!(
        actual.contains("g=m.max(1,2)"),
        "m.max(1,2) must NOT fold (non-global receiver); got:\n{actual}"
    );
}

/// Regression guard: the output must be the SIMPLE typed pipeline, not the
/// `WHITESPACE_ONLY` fallback (which would leave every call intact). Exactly
/// three `max(`/`min(` calls survive — the non-literal-arg, the empty, and the
/// non-global receiver — while the four foldable calls collapse. (Under the
/// whitespace fallback all seven would remain.)
#[test]
fn simple_fold_math_max_min_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    let surviving = actual.matches("max(").count() + actual.matches("min(").count();
    assert_eq!(
        surviving, 3,
        "exactly three Math max/min calls (non-literal arg, empty, non-global \
         receiver) should remain — proving the typed SIMPLE optimizer ran, not \
         the whitespace fallback; got:\n{actual}",
    );
}
