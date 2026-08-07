//! Integration test for the `tests/diff/empty-statement/` fixture.
//!
//! Exercises statement-list-level empty-statement removal end-to-end — the
//! CLOC12.195 arc. `closure-pass-dce`'s `dce_program` now sweeps bare `;` out of
//! the program body (mirroring the block-body sweep `dce_block_statement` already
//! did), so stray semicolons — hand-written, or left behind by the CLOC12.194
//! block-flatten — are removed. An empty statement at statement-list position is
//! a no-op, so removing it is byte-safe.
//!
//! ## Fact — SIMPLE: stray top-level `;` are removed
//!
//! `;g(1);;g(2);` at SIMPLE emits `g(1);g(2);` — the leading `;` and the interior
//! `;;` collapse away, leaving only the two real statements. Byte-identical to the
//! reference Closure Compiler. A `for(;;);` / `if(c);` empty *substatement* (a
//! loop/if body) is NOT a statement-list member and is left intact — but this
//! fixture has none, so the point here is purely the removal.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/empty-statement/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn stray_top_level_empty_statements_are_removed() {
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
    let expected = std::fs::read_to_string("tests/diff/empty-statement/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // Both real statements survive; every stray `;` is gone — no `;;` run and no
    // leading `;`. This is the proof DCE swept the top-level empties.
    assert!(flat.contains("g(1)") && flat.contains("g(2)"), "real statements dropped: {actual}");
    assert!(!flat.contains(";;"), "a `;;` empty-statement run survived: {actual}");
    assert!(!flat.starts_with(';'), "a leading `;` empty statement survived: {actual}");
}
