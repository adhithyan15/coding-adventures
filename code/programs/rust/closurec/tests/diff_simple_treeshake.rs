//! Integration test for the `tests/diff/simple-treeshake/` fixture.
//!
//! `treeshake` (deletes unreferenced top-level `function`/`class`
//! declarations) is a CLOSED-WORLD pass and runs ONLY at ADVANCED. At
//! `--compilation_level SIMPLE` the compiler is open-world: although
//! *declaring* a function has no side effect, *deleting* an observable global
//! is itself observable — another script sharing the page could call `dead` —
//! so nothing at top level is removed:
//!
//! ```text
//! function dead() { return 1; }   ⇒  function dead(){return 1}    (KEPT — open-world)
//! function live() { return 2; }   ⇒  function live(){return 2}    (called below)
//! log(live());                    ⇒  log(live());
//! sink(live);                     ⇒  sink(live);
//! ```
//!
//! Under ADVANCED, `dead` would be tree-shaken away. Under WHITESPACE_ONLY
//! both functions also survive (it never runs treeshake) — see the
//! `simple_treeshake_*` unit tests in `src/run.rs`.

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
