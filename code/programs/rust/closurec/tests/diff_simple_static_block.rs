//! Integration test for the `tests/diff/simple-static-block/` fixture.
//!
//! Exercises **static initialization blocks** (`ClassMember::StaticBlock` /
//! `BlockStatement`) end-to-end at SIMPLE — the CLOC12.176 arc's bridge (PR2) on
//! top of the node + emit + pass arms (PR1).
//!
//! The fixture is `class C { static { x = 1 + 2 } }` compiled at SIMPLE.
//! Two facts prove the whole pipeline ran through the static block:
//!   1. the class declaration round-trips — proving the bridge built a real
//!      `ClassDeclaration` whose body holds a `ClassMember::StaticBlock` the
//!      emitter can print, not a WHITESPACE_ONLY fallback; and
//!   2. the block body folds — `1 + 2` → `3` INSIDE `static { … }` — proving
//!      the SIMPLE pipeline descended into the block's statement list (the
//!      shared statement converter feeds the constant-fold `StaticBlock` arm
//!      added in PR1). A WHITESPACE_ONLY fallback would leave `1+2` intact.
//! Were the bridge to decline the `static_block` member, the file would drop to
//! WHITESPACE_ONLY (`class C{static{x=1+2}};`) and assertion (2) would fail.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-static-block/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_static_block_folds_body() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-static-block/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture. Strip spaces so the checks are
    // insensitive to inter-token whitespace.
    let a = actual.replace(' ', "");
    // (1) the class declaration round-tripped — proving the bridge built a real
    //     `ClassDeclaration` with a static-block member, not a WHITESPACE_ONLY
    //     pass.
    assert!(
        a.contains("classC{") || a.contains("class C{"),
        "class declaration did not round-trip: {actual}"
    );
    // (2) the static block body folded — proving the pipeline descended INTO the
    //     block's statement list (`1+2`→`3`). A WHITESPACE_ONLY fallback would
    //     leave the arithmetic intact.
    assert!(
        a.contains("static{x=3}"),
        "static block body did not fold to `x=3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
    // (3) a class *declaration* emits bare — NO trailing `;` after the closing
    //     `}` and NO wrapping paren (mirrors simple-class-decl; a
    //     WHITESPACE_ONLY fallback appends `;`).
    let t = actual.trim_end_matches('\n');
    assert!(
        t.ends_with('}') && !t.ends_with("};") && !t.starts_with('('),
        "class declaration must emit bare (no trailing `;`, no wrap): {actual}"
    );
}
