//! Integration test for the `tests/diff/div-by-zero/` fixture.
//!
//! Exercises the "do not fold division/modulo by zero" fix end-to-end — the
//! `closure-pass-constant-fold` 0.103.0 arc.
//!
//! ## Why the fold is declined
//!
//! `x / 0` and `x % 0` evaluate to `±Infinity` / `NaN`. The reference Closure
//! Compiler does not fold them to those literals — the result is LONGER than
//! the source, and `Infinity` / `NaN` are ordinary globals that can be shadowed
//! in scope, so emitting them where the source computed the value
//! arithmetically would not even be sound. closurec now keeps the source op,
//! matching Closure: `1/0` stays `1/0`, `0/0` stays `0/0`, `1%0` stays `1%0`.
//! A non-zero divisor still folds (`6/3`→`2`, `5/2`→`2.5`, `1/8`→`.125`).
//!
//! ## Fact — SIMPLE: `x = 1/0; y = 1+1;` → `x=1/0;y=2;`
//!
//! The `1/0` is KEPT (not folded to `Infinity`), while the companion `1+1`
//! folds to `2` — proving the optimization pipeline ran. A WHITESPACE_ONLY
//! fallback would emit `x=1/0;y=1+1;` (only stripping whitespace), so the
//! presence of the folded `y=2` together with the retained `1/0` is the proof.
//! Verified byte-identical to the real Closure jar.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/div-by-zero/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn division_by_zero_is_not_folded_while_neighbour_folds() {
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
    let expected = std::fs::read_to_string("tests/diff/div-by-zero/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // `1/0` was KEPT (not folded to `Infinity`)...
    assert!(
        flat.contains("1/0"),
        "`1/0` was folded away — division by zero must be kept: {actual}"
    );
    assert!(
        !flat.contains("Infinity"),
        "`1/0` was folded to `Infinity` — division by zero must not fold: {actual}"
    );
    // ...while the companion `1+1` DID fold — proof the pipeline ran (not a
    // WHITESPACE_ONLY fallback, which keeps `1+1`).
    assert!(
        flat.contains("y=2"),
        "companion `1+1` did not fold to `2` — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
}
