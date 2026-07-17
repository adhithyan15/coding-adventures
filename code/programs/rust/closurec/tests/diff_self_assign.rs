//! Integration test for the `tests/diff/self-assign/` fixture.
//!
//! Exercises bare-identifier self-assignment removal end-to-end — the
//! `closure-pass-dce` 0.29.0 arc.
//!
//! ## Why this removal is sound
//!
//! `x = x;` reads the variable `x` and writes the same value straight back to
//! the same binding — a no-op on a lexical binding. The reference Closure
//! Compiler removes it at `SIMPLE`. The removal is scoped to a plain `=`
//! between two identically-named identifiers: a MEMBER self-assign
//! (`o.x = o.x`) can trigger a getter/setter and is KEPT, a compound assign
//! (`x += x`, i.e. `x = x + x`) is not a no-op and is KEPT, and a
//! differently-named assign (`x = y`) is a real write and is KEPT.
//!
//! ## Fact — SIMPLE: `g(1); x = x; g(2);` → `g(1);g(2);`
//!
//! The middle `x = x;` statement is removed while the surrounding calls
//! survive. A WHITESPACE_ONLY fallback emits `g(1);x=x;g(2);` verbatim (only
//! stripping whitespace), so the absence of `x=x` together with the retained
//! `g(1)`/`g(2)` proves the optimization pipeline ran. Verified byte-identical
//! to the real Closure jar (`x=x` removed; `o.x=o.x` / `x+=x` / `x=y` kept).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/self-assign/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn bare_identifier_self_assignment_is_removed() {
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
    let expected = std::fs::read_to_string("tests/diff/self-assign/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // `x=x` was removed but the neighbours survived. Proof the file OPTIMIZED
    // (not a WHITESPACE_ONLY fallback, which keeps `x=x`). Checked on
    // space-stripped output.
    assert!(
        flat.contains("g(1);g(2)"),
        "the self-assign was not removed cleanly between its neighbours — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("x=x"),
        "the `x=x` self-assignment survived: {actual}"
    );
}
