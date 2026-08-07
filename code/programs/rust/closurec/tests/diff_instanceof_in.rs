//! Integration test for the `tests/diff/instanceof-in/` fixture.
//!
//! Exercises the `in` / `instanceof` hard-boundary space-drop in
//! `closure-emitter` 0.54.0.
//!
//! ## Why the space is dropped
//!
//! The word operators `in` / `instanceof` need a separating space only when the
//! touching character is an identifier-part char (`[A-Za-z0-9_$]`) — otherwise
//! the keyword would fuse with its neighbour into one identifier. At a hard
//! boundary (a string's closing quote, `{`, `[`, …) no space is needed, and the
//! reference Closure Compiler drops it in compact mode.
//!
//! ## Fact — SIMPLE
//!
//! - `"k" in obj` → `"k"in obj`  (the closing `"` absorbs the LEFT space)
//! - `a in {}`    → `a in{}`      (the `{` absorbs the RIGHT space)
//! - `a instanceof b` → `a instanceof b`  (both seams are identifiers: kept)
//!
//! Verified byte-identical to the real Closure jar.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/instanceof-in/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn word_operators_drop_space_at_hard_boundaries() {
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
    let expected = std::fs::read_to_string("tests/diff/instanceof-in/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
