//! Integration test for the `tests/diff/simple-function-expression/` fixture.
//!
//! Exercises CLOC12.149 / gap-153 — a `function` in **value** position now
//! flows through the full SIMPLE pipeline end-to-end (parser → typed-AST
//! bridge → passes → emitter) instead of declining at the bridge and falling
//! back to WHITESPACE_ONLY.
//!
//! The fixture puts a function expression in every common position — a named
//! RHS (`var f = function make(n){…}`), a function-valued property
//! (`{run: function(){…}}`), an IIFE (`(function(){…})()`), and a callback
//! argument (`arr.map(function(x){…}))` — each with a foldable body. The
//! expected output proves constant-fold and fold-control-flow ran *inside*
//! those bodies (`1+2`→`3`, `2*3`→`6`, `3+4`→`7` with the IIFE's `var`
//! hoisted), which a WHITESPACE_ONLY passthrough would NOT do.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-function-expression/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_function_expression_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-function-expression/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture: the bodies were optimised, so the
    // folded constants must be present and the pre-fold expressions gone.
    // (A WHITESPACE_ONLY fallback would still contain `1+2` / `2*3` / `3+4`.)
    let a = actual.replace(' ', "");
    assert!(a.contains("return 3") || a.contains("return3"), "1+2 not folded: {actual}");
    assert!(!a.contains("1+2") && !a.contains("2*3") && !a.contains("3+4"),
        "a pre-fold expression survived — did it fall back to WHITESPACE_ONLY? {actual}");
}
