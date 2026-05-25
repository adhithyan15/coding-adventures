//! Integration test for the `tests/diff/help-markdown/` fixture.
//!
//! Exercises CLOC11.54 — `--help_markdown` — end-to-end via the
//! built binary. The fixture pins the *exact* markdown output for
//! closurec's flag surface. Whenever a flag is added, renamed,
//! re-described, or has its default changed, this diff fails and
//! the change must be acknowledged by regenerating the fixture.
//!
//! That's the intent: the user-facing flag surface is part of
//! the binary's public contract, and a diff failure here is the
//! right place to surface "you changed something users will see."
//!
//! To regenerate after an intentional change:
//!
//! ```sh
//! ./target/debug/closurec --help_markdown > \
//!   code/programs/rust/closurec/tests/diff/help-markdown/expected.stdout
//! ```

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/help-markdown/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn help_markdown_fixture_matches_expected_dump() {
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
    let expected = std::fs::read_to_string("tests/diff/help-markdown/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.as_ref(),
        expected,
        "help_markdown fixture mismatch — regenerate with:\n  ./target/debug/closurec --help_markdown > code/programs/rust/closurec/tests/diff/help-markdown/expected.stdout"
    );
}
