//! Integration test for the `tests/diff/simple-remove-unused-vars/`
//! fixture.
//!
//! Exercises the CLOC12.158 addition of the `remove-unused-vars` pass to
//! the `--compilation_level SIMPLE` pipeline, which is now
//! `constant-fold → fold-control-flow → dce → inline → remove-unused-vars`.
//! The pass deletes top-level bindings nothing references when their
//! initializer is side-effect-free:
//!
//! ```text
//! var dead = 1 + 2;     ⇒  (removed — folds to a literal, then dropped)
//! var live = 10;        ⇒  var live=10;   (referenced by log(live))
//! var impure = run();   ⇒  var impure=run();   (kept — call may have a side effect)
//! log(live);            ⇒  log(live);
//! ```
//!
//! The `var dead = 1 + 2` row is the load-bearing one: `constant-fold`
//! turns `1 + 2` into the literal `3`, and only then does
//! `remove-unused-vars` see a pure (literal) initializer it can drop —
//! proving the two passes compose. The same input under WHITESPACE_ONLY
//! keeps every declaration (see the `simple_remove_unused_*` unit tests
//! in `src/run.rs`).
//!
//! `remove-unused-vars` needs `inline` registered (it declares
//! `depends_on = ["dce", "inline"]`); `inline` is an identity pass today
//! and is wired in alongside to satisfy the scheduler.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-remove-unused-vars/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_remove_unused_vars_fixture_matches_expected_stdout() {
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
    let expected =
        std::fs::read_to_string("tests/diff/simple-remove-unused-vars/expected.stdout")
            .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nflags: {flags:?}\nactual:\n{actual}\nexpected:\n{expected}",
    );
}
