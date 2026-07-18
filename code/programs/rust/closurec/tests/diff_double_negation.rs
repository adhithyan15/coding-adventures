//! Integration test for the `tests/diff/double-negation/` fixture.
//!
//! Exercises the idempotent double-negation collapse end-to-end — the
//! `closure-pass-constant-fold` 0.102.0 arc.
//!
//! ## Why this fold is sound
//!
//! A `!` whose operand is itself a `!!y` drops that inner `!!` pair:
//! `!!!x` → `!x`. This is sound for ANY operand with no side-effect gate,
//! because `!` never re-evaluates its operand — the operand is evaluated
//! exactly once no matter how many `!` wrap it — and `ToBoolean` invokes no
//! user coercion. `!!!x` computes `¬¬¬ToBoolean(x)` = `¬ToBoolean(x)` = `!x`.
//! A lone `!!y` is preserved (it is the minified `Boolean(y)` coercion, whose
//! value differs from `y`). The reference Closure Compiler performs exactly
//! this at `SIMPLE`.
//!
//! ## Fact — SIMPLE: `g(!!!a)` → `g(!a)`
//!
//! `g(!!!a);` at SIMPLE emits `g(!a);` — two of the three `!` collapse. A
//! WHITESPACE_ONLY fallback emits `g(!!!a);` verbatim (only stripping
//! whitespace), so the reduction from `!!!` to a single `!` proves the
//! optimization pipeline ran. Verified byte-identical to the real Closure jar
//! across the truth table (`!!!a`→`!a`, `!!!!a`→`!!a`, `!!a` kept, impure and
//! compound operands).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/double-negation/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn triple_negation_collapses_to_single() {
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
    let expected = std::fs::read_to_string("tests/diff/double-negation/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // `!!!a` collapsed to a single `!a`. Proof the file OPTIMIZED (not a
    // WHITESPACE_ONLY fallback, which keeps `!!!a`). Checked on space-stripped
    // output.
    assert!(
        flat.contains("g(!a)"),
        "double-negation did not collapse to `!a` — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("!!"),
        "a `!!` survived — `!!!` should reduce to `!`: {actual}"
    );
}
