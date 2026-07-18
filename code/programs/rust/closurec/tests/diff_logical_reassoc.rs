//! Integration test for the `tests/diff/logical-reassoc/` fixture.
//!
//! Exercises the `&&`/`||` left-associativity normalization end-to-end —
//! `closure-pass-constant-fold` rewrites a right-nested same-operator logical
//! `a && (b && c)` to the left-nested `(a && b) && c`, which prints without the
//! parens the right-nested form requires. Byte-identical to the reference
//! Closure Compiler.
//!
//! ## Fact — SIMPLE: `x=a&&(b&&c);` → `x=a&&b&&c;`
//!
//! The distinguishing output is the absence of the inner `(b&&c)` parens.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/logical-reassoc/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn right_nested_logical_reassociates_left() {
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
    let expected = std::fs::read_to_string("tests/diff/logical-reassoc/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // `a&&(b&&c)` re-associated to `a&&b&&c` — the inner parens are gone.
    assert!(
        flat.contains("a&&b&&c"),
        "logical not re-associated — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("(b&&c)"),
        "inner parens remain — the re-association did not run: {actual}"
    );
}
