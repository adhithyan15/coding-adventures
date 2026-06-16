//! Integration test for the `tests/diff/simple-treeshake/` fixture.
//!
//! Exercises the CLOC12.159 addition of the `treeshake` pass to the
//! `--compilation_level SIMPLE` pipeline, which is now
//! `constant-fold → fold-control-flow → dce → inline → remove-unused-vars
//! → treeshake`. `treeshake` deletes top-level `function`/`class`
//! declarations that nothing references — the function-shaped complement
//! to `remove-unused-vars` (which skips functions):
//!
//! ```text
//! function dead() { return 1; }   ⇒  (removed — never called)
//! function live() { return 2; }   ⇒  function live(){return 2}   (called below)
//! log(live());                    ⇒  log(live());
//! ```
//!
//! Removing an unused function declaration is unconditionally safe —
//! declaring a function has no side effect, so (unlike a `var`
//! initializer) no purity gate is needed. The same input under
//! WHITESPACE_ONLY keeps both functions (see the `simple_treeshake_*`
//! unit tests in `src/run.rs`).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-treeshake/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_treeshake_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-treeshake/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
