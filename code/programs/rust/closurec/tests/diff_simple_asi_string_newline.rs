//! Integration test for the `tests/diff/simple-asi-string-newline/` fixture.
//!
//! Exercises ASI Rule 1 across a statement that ends in a string literal — the
//! case the Phase-2 start-line heuristic conservatively declined. Now the lexer
//! flags the offending token (`TOKEN_PRECEDED_BY_NEWLINE`) directly, so the
//! semicolon-free, newline-separated program parses and `1 + 2` folds to `3`:
//!
//! ```text
//! var label="total";var n=3;show(label,n);
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-asi-string-newline/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_asi_string_newline_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-asi-string-newline/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Regression guard: the output must NOT be the WHITESPACE_ONLY fallback — the
/// folded `n=3` is only reachable because ASI made the program parse, and a
/// whitespace-only fallback would keep `1+2`.
#[test]
fn simple_asi_string_newline_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        actual.contains("n=3"),
        "expected `1 + 2` to fold to `3` (proving ASI recovered the string-ending \
         statement); got:\n{actual}",
    );
    assert!(
        !actual.contains("1+2") && !actual.contains("1 + 2"),
        "expected the folded form, not the literal `1+2`; got:\n{actual}",
    );
}
