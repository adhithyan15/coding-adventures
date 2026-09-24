//! Integration test for the `tests/diff/simple-debugger/` fixture.
//!
//! Exercises `--compilation_level SIMPLE` across a `debugger;` statement.
//! Before CLOC21, any program containing one failed the typed-AST parse and
//! closurec fell back to WHITESPACE_ONLY (zero optimization). This fixture is
//! the end-to-end oracle proving the SIMPLE pipeline runs across a `debugger`
//! statement: `1 + 2` folds to `3`, and the `debugger;` itself is PRESERVED.
//!
//! CLOC24 used to strip it, and this file used to assert that. CCR-053
//! measured it against the pinned oracle: upstream keeps `debugger` at SIMPLE
//! and ADVANCED wherever it is reachable. The single-use `log` declaration is
//! KEPT — SIMPLE is open-world and never inlines/deletes observable top-level
//! names; that fold happens only at ADVANCED (closed-world).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-debugger/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_debugger_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-debugger/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}

/// Regression guard: the output must NOT be the WHITESPACE_ONLY fallback. If
/// the typed pipeline silently stopped running, the fixture test above would
/// still "pass" against a regenerated expected file, so this asserts specific
/// transforms instead of comparing bytes.
///
/// It used to assert two things a WHITESPACE_ONLY fallback would not do: the
/// `1 + 2` constant-fold, and the strip of `debugger;`. CCR-053 removed the
/// strip — it was never upstream's behaviour — so only the fold remains as a
/// positive pipeline signal on this input. (The fallback emits
/// `var x=1+2;debugger;` verbatim, so the fold alone does distinguish them.)
///
/// The second assertion is now the opposite one, and it earns its place: it
/// guards CCR-053 itself, so a future pass that starts deleting `debugger`
/// again fails here rather than silently changing what the program does under
/// an attached debugger.
///
/// Note: unlike ADVANCED, SIMPLE is open-world — it must NOT inline the
/// single-use `log` into `report(1)` nor delete the `function log`
/// declaration, so the guard deliberately asserts the declaration is KEPT.
#[test]
fn simple_debugger_did_not_fall_back_to_whitespace_only() {
    let out = Command::new(BINARY)
        .args(read_flags())
        .output()
        .expect("run closurec");
    let actual = String::from_utf8_lossy(&out.stdout);

    assert!(
        actual.contains("var x=3"),
        "expected `1 + 2` to be constant-folded to `3` \
         (proving the typed pipeline ran, not the whitespace fallback); \
         got:\n{actual}",
    );
    assert!(
        actual.contains("debugger"),
        "expected the `debugger;` statement to be PRESERVED (CCR-053): \
         upstream Closure keeps it at SIMPLE wherever it is reachable, and \
         removing it changes observable behaviour under a debugger; \
         got:\n{actual}",
    );
    assert!(
        actual.contains("function log"),
        "SIMPLE is open-world and must keep the observable top-level `log` \
         declaration; got:\n{actual}",
    );
}
