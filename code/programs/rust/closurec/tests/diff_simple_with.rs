//! Integration test for the `tests/diff/simple-with/` fixture.
//!
//! Exercises the legacy `with (obj) { … }` statement end-to-end — the
//! CLOC12.187 arc. Before it, `with` landed in the parser bridge's *unsupported*
//! bucket, so any file containing a `with` DECLINED to WHITESPACE_ONLY (no
//! optimization at all). The arc landed in three PRs:
//!   - PR1: the `WithStatement` AST node + emitter + pass traversal (atomic);
//!   - PR2a: the renaming-soundness gate — a `with` splices its object onto the
//!     scope chain, so a bare name in the body may resolve to a property of the
//!     object rather than a lexical binding; the rename / rename-globals /
//!     rename-properties passes therefore *decline to rename* when a `with` is
//!     present (`program_contains_with_statement`);
//!   - PR2b (this test): the bridge flip — `with_statement` now converts to a
//!     `WithStatement` instead of declining, which is sound precisely because
//!     PR2a's gate is in place.
//!
//! ## Fact 1 — SIMPLE: the `with` survives and the pipeline descends into it
//!
//! `with (o) { x(1 + 2); }` at SIMPLE emits `with(o){x(3)}`. Two things prove
//! the whole pipeline ran through the `with`:
//!   1. the `with` round-trips (`with(o){…}`), proving the bridge modelled it —
//!      not a WHITESPACE_ONLY fallback; and
//!   2. the argument folds — `1 + 2` → `3` — proving the SIMPLE pipeline
//!      descended INTO the `with` body. A WHITESPACE_ONLY fallback would leave
//!      `1+2` intact (it would emit `with(o){x(1+2)}` verbatim).
//!
//! ## Fact 2 — ADVANCED: the soundness gate leaves a `with` program un-renamed
//!
//! `function longName() { with (o) { x(); } } longName();` at ADVANCED keeps
//! `longName` (and the `with`) verbatim, because the PR2a gate disables the
//! renaming passes whenever a `with` is present. The identical program WITHOUT
//! the `with` is optimized down to `x();` (the function is inlined and its now
//! dead global removed), so the surviving `longName` is a direct, end-to-end
//! observation of the gate firing — not merely "nothing to rename".

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-with/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_with_round_trips_and_folds() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-with/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let a = actual.replace(' ', "");
    // (1) the `with` round-tripped — the bridge modelled it (not WHITESPACE_ONLY).
    assert!(
        a.contains("with(o)"),
        "`with` did not round-trip: {actual}"
    );
    // (2) the pipeline descended INTO the `with` body: `1+2` folded to `3`.
    assert!(
        a.contains("x(3)"),
        "argument did not fold to `x(3)`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}

#[test]
fn advanced_with_disables_renaming() {
    // ADVANCED on a program that contains a renameable global AND a `with`.
    // The PR2a soundness gate must fire, leaving `longName` un-renamed.
    let out = Command::new(BINARY)
        .args([
            "--js",
            "tests/diff/simple-with/input/adv.js",
            "--compilation_level",
            "ADVANCED",
        ])
        .output()
        .expect("run closurec");

    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let actual = String::from_utf8_lossy(&out.stdout);
    let a = actual.replace(' ', "");
    // The `with` bridged (not WHITESPACE_ONLY)…
    assert!(a.contains("with(o)"), "`with` did not round-trip: {actual}");
    // …and the gate left the otherwise-renameable global `longName` intact.
    // (The identical program without the `with` optimizes down to `x();`.)
    assert!(
        a.contains("longName"),
        "gate did not fire — `longName` was renamed away under a `with`: {actual}"
    );
}
