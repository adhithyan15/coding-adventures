//! Integration test for the `tests/diff/for-body-fuse/` fixture.
//!
//! Exercises `for`-loop body comma-fusion end-to-end —
//! `closure-pass-fold-control-flow` collapses a block body of plain expression
//! statements to a single comma-sequenced expression statement, dropping the
//! braces. The comma operator runs the statements left-to-right with the same
//! side effects, and a loop body discards the value, so the rewrite is
//! behaviour-preserving and byte-identical to the reference Closure Compiler.
//!
//! ## Fact — SIMPLE: `for(;x;){a();b();}` → `for(;x;)a(),b();`
//!
//! The proof the pipeline ran (not a WHITESPACE_ONLY fallback, which would keep
//! `for(;x;){a();b()}`) is the absence of the body braces.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/for-body-fuse/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn for_body_fuses_to_comma_sequence() {
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
    let expected = std::fs::read_to_string("tests/diff/for-body-fuse/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // `{a();b()}` fused to `a(),b()`. The presence of `a(),b()` and absence of
    // the body braces prove the pipeline optimized (not a WHITESPACE_ONLY
    // fallback, which would keep `for(;x;){a();b()}`).
    assert!(
        flat.contains("a(),b()"),
        "body did not fuse to a comma-sequence — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("{a()"),
        "body braces still present — the fusion did not run: {actual}"
    );
}
