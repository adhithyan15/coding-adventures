//! Integration test for the `tests/diff/simple-class-field/` fixture.
//!
//! Exercises **class fields** (`ClassMember::Field` / `PropertyDefinition`)
//! end-to-end at SIMPLE — the CLOC12.175 arc's bridge (PR2) on top of the node
//! + emit + pass arms (PR1).
//!
//! The fixture is `class C { x = 1 + 2; static s = 5 + 6; }` compiled at SIMPLE.
//! Three facts prove the whole pipeline ran through the field:
//!   1. the class declaration round-trips — proving the bridge built a real
//!      `ClassDeclaration` whose body holds two `ClassMember::Field`s the
//!      emitter can print, not a WHITESPACE_ONLY fallback;
//!   2. BOTH field initializers fold — `1 + 2` → `3` and `5 + 6` → `11` —
//!      proving the SIMPLE pipeline descended INTO each field's initializer
//!      (the constant-fold `Field` arm added in PR1). A WHITESPACE_ONLY fallback
//!      would leave `1+2` / `5+6` intact; and
//!   3. the `static` modifier survives on the second field.
//! Were the bridge to decline the field member, the file would drop to
//! WHITESPACE_ONLY and the arithmetic would NOT fold — assertion (2) would fail.

// Literate-programming test docs: intentional prose paragraphs following lists.
// clippy 1.97's doc-list-continuation lints flag them as mis-indented list
// items; the formatting is deliberate, so allow crate-wide for this test.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-class-field/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_class_field_folds_initializers() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-class-field/expected.stdout")
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
    //     `ClassDeclaration` with field members, not a WHITESPACE_ONLY pass.
    assert!(
        a.contains("classC{") || a.contains("class C{"),
        "class declaration did not round-trip: {actual}"
    );
    // (2) BOTH field initializers folded — proving the pipeline descended into
    //     each field's initializer (`1+2`→`3`, `5+6`→`11`). A WHITESPACE_ONLY
    //     fallback would leave the arithmetic intact.
    assert!(
        a.contains("x=3;") && a.contains("s=11;"),
        "field initializers did not fold to `3` / `11`: {actual}"
    );
    assert!(
        !a.contains("1+2") && !a.contains("5+6"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
    // (3) the `static` modifier survived on the second field.
    assert!(
        a.contains("statics=11;"),
        "the `static` field modifier was dropped: {actual}"
    );
    // (4) a class *declaration* emits bare — NO trailing `;` after the closing
    //     `}` and NO wrapping paren (mirrors simple-class-decl).
    let t = actual.trim_end_matches('\n');
    assert!(
        t.ends_with('}') && !t.ends_with("};") && !t.starts_with('('),
        "class declaration must emit bare (no trailing `;`, no wrap): {actual}"
    );
}
