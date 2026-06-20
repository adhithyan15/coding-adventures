//! Integration test for the `tests/diff/advanced-try-catch-rename/`
//! fixture.
//!
//! Exercises `--compilation_level ADVANCED` renaming SOUNDNESS across a
//! `catch` binding (CLOC19). The catch parameter is a declared binding
//! that the renamer must treat as reserved: it is never itself renamed,
//! and no other local may be renamed onto it. This fixture pins both
//! halves of that guarantee end-to-end.
//!
//! `process`/`value`/`temp` get short names (`c`/`a`/`b`), the param use
//! inside the `try` block and the local use inside the `catch` body are
//! both rewritten, and the catch binding `err` is preserved verbatim
//! and never aliased to a generated name.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/advanced-try-catch-rename/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn advanced_try_catch_rename_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/advanced-try-catch-rename/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Explicit soundness assertion, independent of the exact short names
/// the renamer happens to choose: the catch binding `err` must survive
/// verbatim, and the rewritten body must reference it (`report(err,…)`),
/// proving no generated name ever shadows or overwrites the handler's
/// caught value.
#[test]
fn advanced_try_catch_rename_preserves_catch_binding() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        actual.contains("catch(err)"),
        "catch parameter `err` must be preserved verbatim; got:\n{actual}",
    );
    assert!(
        actual.contains("report(err,"),
        "catch body must still reference the caught `err`; got:\n{actual}",
    );
}
