//! Integration test for the `tests/diff/drop-new-obj-arr/` fixture.
//!
//! Exercises the standard-constructor `new`-drop for `Object`/`Array`
//! end-to-end — `closure-pass-constant-fold` rewrites `new Array(1,2,3)` to the
//! array literal `[1,2,3]`. Calling `Array`/`Object` as an ordinary function
//! constructs the same value as `new`, so the fold is semantics-preserving and
//! byte-identical to the reference Closure Compiler.
//!
//! ## Fact — SIMPLE: `var z=new Array(1,2,3);` → `var z=[1,2,3];`
//!
//! The proof the pipeline ran (not a WHITESPACE_ONLY fallback, which would keep
//! `new Array(1,2,3)`) is the array literal `[1,2,3]`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/drop-new-obj-arr/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn new_array_folds_to_array_literal() {
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
    let expected = std::fs::read_to_string("tests/diff/drop-new-obj-arr/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // `new Array(1,2,3)` → `[1,2,3]`. The presence of the array literal and
    // absence of the `new` keyword prove the pipeline optimized (not a
    // WHITESPACE_ONLY fallback, which would keep `new Array(1,2,3)`).
    assert!(
        flat.contains("[1,2,3]"),
        "Array did not fold to a literal — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("new"),
        "`new` keyword still present — the constructor fold did not run: {actual}"
    );
}
