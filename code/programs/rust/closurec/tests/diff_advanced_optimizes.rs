//! Integration test for the `tests/diff/advanced-optimizes/` fixture.
//!
//! Exercises CLOC12.161 — `--compilation_level ADVANCED` now runs the
//! typed optimization pipeline instead of being a literal no-op (it used
//! to return the source verbatim). ADVANCED is specified to be at least
//! as aggressive as SIMPLE, so it currently reuses the SIMPLE pipeline:
//! constant-folding, dead-code elimination, unused-binding removal, and
//! local renaming all apply. Advanced-only passes (aggressive
//! property/global renaming, cross-module tree-shaking) layer on as they
//! are implemented.
//!
//! The companion `advanced_matches_simple_*` unit test in `src/run.rs`
//! pins that ADVANCED and SIMPLE produce identical output today.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/advanced-optimizes/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn advanced_optimizes_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/advanced-optimizes/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
