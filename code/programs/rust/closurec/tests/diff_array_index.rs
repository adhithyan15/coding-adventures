//! Integration test for the `tests/diff/array-index/` fixture.
//!
//! Exercises array-literal index-access constant folding end-to-end — the
//! CLOC12.196 arc. `closure-pass-constant-fold`'s `fold_member` computed path now
//! folds `[e0..en][K]` (a constant integer index into an array literal) to the
//! selected element, the companion to the CLOC12.193 array-`.length` fold.
//!
//! ## Fact — SIMPLE: `[1,2,3][1]` folds to `2`
//!
//! `g([1,2,3][1]);` at SIMPLE emits `g(2);` — the array literal and index access
//! collapse to the element at index 1. Byte-identical to the reference Closure
//! Compiler. The proof the pipeline ran (not a WHITESPACE_ONLY fallback, which
//! would keep `g([1,2,3][1]);` verbatim) is that the output is `g(2)` with no
//! residual array-index `[1,2,3][1]`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/array-index/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn array_literal_index_folds_to_element() {
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
    let expected = std::fs::read_to_string("tests/diff/array-index/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // `[1,2,3][1]` folded to `2`. The presence of `g(2)` and absence of the
    // array-index `[1,2,3][1]` prove the pipeline optimized (not a
    // WHITESPACE_ONLY fallback).
    assert!(
        flat.contains("g(2)"),
        "array index did not fold to `2` — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("]["),
        "unfolded array-index access present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
