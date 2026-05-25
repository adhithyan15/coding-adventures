//! Integration test for the `tests/diff/print-tree-json/` fixture.
//!
//! Exercises CLOC11.53 — `--print_tree_json` — end-to-end via the
//! built binary. Single-file invocation, so the wire format is
//! the bare tokens array (no file-object wrapper). The fixture
//! pins both the structural shape and the exact whitespace so
//! regressions in the lexer or in the JSON emitter surface as a
//! diff failure.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/print-tree-json/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn print_tree_json_fixture_matches_expected_dump() {
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
    let expected = std::fs::read_to_string("tests/diff/print-tree-json/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.as_ref(),
        expected,
        "print_tree_json fixture mismatch:\n--- actual ---\n{actual}\n--- expected ---\n{expected}"
    );
}
