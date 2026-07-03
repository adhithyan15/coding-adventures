//! Integration test for the `tests/diff/print-tree/` fixture.
//!
//! Exercises CLOC11.52 — `--print_tree` — end-to-end via the
//! built binary. Until the parser produces the typed AST, our
//! `--print_tree` emits the **token stream** (one significant
//! token per line, trivia + EOF filtered), with a `=== <path>
//! ===` banner per input. The fixture pins this exact wire
//! format so regressions in the lexer or in the run-pipeline
//! short-circuit surface as a diff failure.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/print-tree/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn print_tree_fixture_matches_expected_dump() {
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
    let expected = std::fs::read_to_string("tests/diff/print-tree/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.as_ref(),
        expected,
        "print_tree fixture mismatch:\n--- actual ---\n{actual}\n--- expected ---\n{expected}"
    );
}
