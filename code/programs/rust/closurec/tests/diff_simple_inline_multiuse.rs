//! Integration test for the `tests/diff/simple-inline-multiuse/` fixture.
//!
//! Inlining a top-level function's body into its call sites is a
//! CLOSED-WORLD transform (it rewrites an observable global) and runs ONLY at
//! ADVANCED. At `--compilation_level SIMPLE` the compiler is open-world, so
//! the small pure function `sq` is KEPT and both `a(sq(3))` / `b(sq(4))` stay
//! as calls — the arithmetic `3 * 3` / `4 * 4` never appears because the body
//! is not substituted. Result: `function sq(x){return x*x};a(sq(3));b(sq(4));`.
//! Under ADVANCED the body is inlined at both sites, `sq` is tree-shaken, and
//! constant-fold folds `3 * 3` / `4 * 4` to `9` / `16`, giving `a(9);b(16);`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-inline-multiuse/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_inline_multiuse_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-inline-multiuse/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
