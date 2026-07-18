//! Integration test for the `tests/diff/division-fold-threshold/` fixture.
//!
//! Exercises `closure-pass-constant-fold` 0.107.0: `a / b` folds to a numeric
//! literal only when the quotient has <= 7 digits after the decimal point;
//! otherwise Closure keeps the source `a / b` (numeric byte-cost heuristic).
//!
//! ## Facts (oracle-verified against the real Closure jar, SIMPLE)
//!
//! - `1/3`     -> `1/3`         (16 fractional digits, non-terminating)
//! - `1/4`     -> `.25`         (2 -> fold)
//! - `6/3`     -> `2`           (integer -> fold)
//! - `811/128` -> `6.3359375`   (7 -> fold, even though longer than 811/128)
//! - `1/256`   -> `1/256`       (8 -> kept)
//! - `1/128`   -> `.0078125`    (7 -> fold)
//! - `2/3`     -> `2/3`         (non-terminating -> kept)
//! - `10/4`    -> `2.5`         (1 -> fold)

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/division-fold-threshold/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn division_folds_only_within_seven_fractional_digits() {
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
    let expected = std::fs::read_to_string("tests/diff/division-fold-threshold/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
