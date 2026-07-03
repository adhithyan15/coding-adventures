//! Integration test for the `tests/diff/simple-rename/` fixture.
//!
//! Exercises the CLOC12.160 addition of the `rename` pass to the
//! `--compilation_level SIMPLE` pipeline, which is now
//! `constant-fold → fold-control-flow → dce → inline → remove-unused-vars
//! → treeshake → rename`. `rename` shortens the parameters of leaf
//! functions (functions with no nested function) to short names, while
//! leaving the (potentially externally-visible) function name alone:
//!
//! ```text
//! function distance(horizontal, vertical) {
//!   return horizontal * horizontal + vertical * vertical;
//! }
//! distance(3, 4);
//! ⇒ function distance(a,b){return a * a + b * b};distance(3,4);
//! ```
//!
//! `distance(3, 4)` keeps the function alive past `treeshake`; an uncalled
//! function would be removed before `rename` ran. The same input under
//! WHITESPACE_ONLY keeps the full parameter names (see the
//! `simple_rename_*` unit tests in `src/run.rs`).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-rename/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_rename_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-rename/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
