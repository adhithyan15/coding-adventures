//! Integration test for the `tests/diff/array-length/` fixture.
//!
//! Exercises the array-literal `.length` constant fold end-to-end — the
//! CLOC12.193 arc. Before it, closurec folded string-literal `.length`
//! (`"abc".length` → `3`) but left array-literal `.length` alone, so
//! `[1, 2, 3].length` survived into the output verbatim. The reference Closure
//! Compiler folds it to the element count; this fixture proves closurec now
//! matches.
//!
//! The arc landed in two PRs:
//!   - PR1 (#8336): the fold itself, in `closure-pass-constant-fold`'s
//!     `fold_member`. `[e0, e1, …].length` → the static element count, guarded
//!     so it declines on a spread (`[...x]`, unknown length) or a
//!     side-effecting element (dropping the array would drop the effect). Holes
//!     (`[,,]`) evaluate nothing but still count. Verified byte-identical to the
//!     real Closure jar across the full truth table.
//!   - PR2 (this test): the closurec end-to-end proof.
//!
//! ## Fact — SIMPLE: the array `.length` folds to the element count
//!
//! `g([1,2,3].length);` at SIMPLE emits `g(3);`. The `g(...)` call consumes the
//! folded value so the whole expression is retained (not dropped as dead), and
//! the `[1,2,3].length` member expression is gone — replaced by the literal `3`.
//! A WHITESPACE_ONLY fallback would emit `g([1,2,3].length);` verbatim (only
//! stripping whitespace), so the presence of `g(3)` and the absence of
//! `[1,2,3].length` together prove the optimization pipeline ran.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/array-length/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn array_literal_length_folds_to_element_count() {
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
    let expected = std::fs::read_to_string("tests/diff/array-length/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // The array `.length` folded: `[1,2,3].length` → `3`. This is the proof the
    // file OPTIMIZED (not a WHITESPACE_ONLY fallback, which would keep
    // `[1,2,3].length`). Checked on space-stripped output.
    assert!(
        flat.contains("g(3)"),
        "array `.length` did not fold to `3` — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("].length"),
        "unfolded array `.length` present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
