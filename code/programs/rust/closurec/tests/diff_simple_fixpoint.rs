//! Integration test for the `tests/diff/simple-fixpoint/` fixture.
//!
//! Exercises CLOC13.F — the pass pipeline runs to a FIXED POINT. Inlining a
//! top-level function is the classic multi-sweep trigger, but it rewrites an
//! observable global, so it runs ONLY at ADVANCED. At open-world SIMPLE the
//! single-use `double` is KEPT and `double(7)` stays a call — no inline, hence
//! no `7 * 2` for a later sweep to fold — so the output is
//! `function double(x){return x*2};log(double(7));`. Under ADVANCED the fixed
//! point inlines `double(7)` → `7 * 2`, tree-shakes `double`, then folds
//! `7 * 2` → `14`, giving `log(14);`. (The fixed-point interplay that survives
//! at SIMPLE — `inline-variables` exposing a later `constant-fold` — is
//! covered by `simple-inline-variables`.)

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-fixpoint/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_fixpoint_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-fixpoint/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
