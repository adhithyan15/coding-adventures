//! Integration test for the `tests/diff/simple-asi-block/` fixture.
//!
//! Exercises CLOC26 Phase 1 — Automatic Semicolon Insertion before a `}` /
//! end-of-input, implemented in the `javascript-parser` `asi` module. The input
//! omits the `;` after `return w * s`; before ASI this failed the grammar parse
//! and closurec degraded the whole program to WHITESPACE_ONLY. ASI now supplies
//! the missing `;`, the program parses, and `1 + 2` constant-folds to `3`:
//!
//! ```text
//! function area(w){var s;s=3;return w * s};report(area(10));
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-asi-block/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_asi_block_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-asi-block/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Regression guard: the output must NOT be the WHITESPACE_ONLY fallback. The
/// input omits a `;` before a `}`; without ASI the program fails to parse and
/// degrades to whitespace-only, which keeps `1+2` verbatim. So an optimized
/// SIMPLE run — only reachable because ASI made the program parse — must show
/// the folded `s=3` and contain no `1+2`.
#[test]
fn simple_asi_block_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        actual.contains("s=3"),
        "expected `1 + 2` to fold to `3` (proving ASI made the typed pipeline \
         run, not the whitespace fallback); got:\n{actual}",
    );
    assert!(
        !actual.contains("1+2") && !actual.contains("1 + 2"),
        "expected the folded form, not the literal `1+2`; got:\n{actual}",
    );
}
