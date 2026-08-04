//! Integration test for the `tests/diff/advanced-class-constructor/` fixture.
//!
//! Exercises the **`constructor` no-rename guard** end-to-end at ADVANCED, on
//! top of the CLOC12.173 PR2 class bridge.
//!
//! At ADVANCED, the rename-properties pass renames property/member keys to
//! short names. A class's `constructor` member is special: renaming its key
//! would turn the constructor into an ordinary prototype method (`new C()`
//! would no longer run it) — a silent miscompile. PR1 added a guard that pins
//! `constructor` so it is never renamed; this fixture is that guard's
//! end-to-end regression.
//!
//! The fixture is `f(class { constructor() { return 1 + 2 } });` compiled at
//! ADVANCED. Two facts prove both that the pipeline ran and that the guard
//! held:
//!   1. the constructor key survives verbatim — the output still contains
//!      `constructor(`, not a renamed short key; and
//!   2. the constructor body folds — `return 1 + 2` → `return 3` — proving the
//!      ADVANCED pipeline actually descended into the class (a WHITESPACE_ONLY
//!      fallback would leave `1 + 2` intact).
//! Were the guard absent, rename-properties would have rewritten `constructor`
//! to a short name (e.g. `class{a(){…}}`) — and the `constructor(` assertion
//! would fail.

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/advanced-class-constructor/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn advanced_class_constructor_is_never_renamed() {
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
    let expected = std::fs::read_to_string("tests/diff/advanced-class-constructor/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let a = actual.replace(' ', "");
    // (1) THE GUARD: the `constructor` key is preserved verbatim at ADVANCED —
    //     rename-properties must never rename it (doing so would break
    //     `new C()`). If the guard regressed, this key would be a short name.
    assert!(
        a.contains("constructor("),
        "constructor key was renamed at ADVANCED — the no-rename guard regressed: {actual}"
    );
    // (2) the constructor body folded — proving the ADVANCED pipeline ran over
    //     the class (`1 + 2` → `3`), not a verbatim WHITESPACE_ONLY pass.
    assert!(
        a.contains("return 3") || a.contains("return3}"),
        "constructor body `1 + 2` did not fold to `3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded `1+2` present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
