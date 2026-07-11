//! Integration test for the `tests/diff/simple-importmeta/` fixture.
//!
//! Exercises CLOC12.168 PR2 — the **`import.meta`** module meta-property
//! (`ImportMeta`) now flows through the full SIMPLE pipeline (parser →
//! typed-AST bridge → passes → emitter) instead of falling through the bridge's
//! internal-error arm and dragging the whole file to WHITESPACE_ONLY
//! (gap-169, now closed).
//!
//! The fixture is `f(import.meta, 1 + 2);` — a retained call (an unknown
//! `f(...)` has side effects, so DCE keeps it) whose first argument is the
//! `import.meta` meta-property and whose second argument is the foldable
//! `1 + 2`. Two facts prove the pipeline ran end-to-end rather than falling
//! back:
//!   1. `import.meta` round-trips (the bridge produced a real `ImportMeta` node
//!      rather than erroring / declining), and
//!   2. the argument `1 + 2` folds to `3`.
//! A WHITESPACE_ONLY fallback — which a bridge failure would force for the
//! *whole file* — would instead re-emit the source verbatim, leaving `1 + 2`
//! unfolded (`f(import.meta, 1 + 2)`).
//!
//! (`import.meta` is syntactically legal only inside a module; the shared
//! grammar is permissive and parses the bare statement, and the bridge simply
//! lowers whatever the parser produced. The point here is the *pipeline
//! plumbing*, not JS semantic validation — see CLOC12.167's `simple-newtarget`
//! fixture for the sibling `new.target` case.)

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-importmeta/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_importmeta_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-importmeta/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture. Strip spaces so the checks are
    // insensitive to inter-token whitespace.
    let a = actual.replace(' ', "");
    // (1) the `import.meta` meta-property survived — proving the bridge
    //     converted it to a real `ImportMeta` node rather than dropping the
    //     file to WHITESPACE_ONLY.
    assert!(
        a.contains("import.meta"),
        "`import.meta` did not round-trip: {actual}"
    );
    // (2) the argument folded — proving the SIMPLE pipeline ran over the call
    //     (`1 + 2` → `3`), not a verbatim WHITESPACE_ONLY pass.
    assert!(
        a.contains("import.meta,3"),
        "argument `1 + 2` did not fold to `3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded `1+2` present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
