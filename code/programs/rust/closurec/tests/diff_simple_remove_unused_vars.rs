//! Integration test for the `tests/diff/simple-remove-unused-vars/`
//! fixture.
//!
//! `remove-unused-vars` (deletes unreferenced top-level `var/let/const`) is a
//! CLOSED-WORLD pass and runs ONLY at ADVANCED. At `--compilation_level
//! SIMPLE` the compiler is open-world — a top-level binding may be read by
//! another script sharing the global object — so nothing at top level is
//! removed. `constant-fold` still runs, folding each initializer:
//!
//! ```text
//! var dead = 1 + 2;     ⇒  var dead=3,      (KEPT — folded, but open-world)
//! var live = 10;        ⇒  live=10,         (referenced by log(live))
//! var impure = run();   ⇒  impure=run();    (KEPT — call may have a side effect)
//! log(live);            ⇒  log(live);
//! ```
//!
//! The `var dead = 1 + 2` row is the load-bearing one: `constant-fold` turns
//! `1 + 2` into the literal `3` (so it emits as `dead=3`), but
//! `remove-unused-vars` is NOT in the SIMPLE pipeline, so the now-pure binding
//! is still kept. Under ADVANCED, `dead` would be dropped while `impure` is
//! kept by the purity gate; under WHITESPACE_ONLY every declaration survives
//! AND `1 + 2` stays unfolded (see the `simple_remove_unused_*` unit tests in
//! `src/run.rs`).

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
