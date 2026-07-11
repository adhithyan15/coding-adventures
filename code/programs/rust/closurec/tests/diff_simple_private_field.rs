//! Integration test for the `tests/diff/simple-private-field/` fixture.
//!
//! Exercises a **private class field** (`#x`, a `PropertyKey::PrivateName` key on
//! a `ClassMember::Field`) end-to-end at SIMPLE — the CLOC12.177 arc's bridge
//! (PR2) on top of the node + emit + pass arms (PR1).
//!
//! The fixture is `class C { #x = 1 + 2 }` compiled at SIMPLE.
//! Two facts prove the whole pipeline ran through the private field:
//!   1. the class declaration round-trips with a `#`-prefixed key — proving the
//!      bridge lowered the bare `PRIVATE_NAME` token to a
//!      `PropertyKey::PrivateName` the emitter can print, not a WHITESPACE_ONLY
//!      fallback; and
//!   2. the initializer folds — `1 + 2` → `3` — proving the SIMPLE pipeline
//!      descended INTO the private field's initializer (the constant-fold `Field`
//!      arm). A WHITESPACE_ONLY fallback would leave `1+2` intact.
//! Were the bridge to decline the private field, the file would drop to
//! WHITESPACE_ONLY (`class C{#x=1+2};`) and assertion (2) would fail.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-private-field/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_private_field_folds_initializer() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-private-field/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture. Strip spaces so the checks are
    // insensitive to inter-token whitespace.
    let a = actual.replace(' ', "");
    // (1) the class declaration round-tripped with a private `#x` key — proving
    //     the bridge lowered the PRIVATE_NAME token, not a WHITESPACE_ONLY pass.
    assert!(
        (a.contains("classC{") || a.contains("class C{")) && a.contains("#x"),
        "private-field class did not round-trip with a `#x` key: {actual}"
    );
    // (2) the initializer folded — proving the pipeline descended INTO the private
    //     field's initializer (`1+2`→`3`). A WHITESPACE_ONLY fallback would leave
    //     the arithmetic intact.
    assert!(
        a.contains("#x=3;"),
        "private field initializer did not fold to `3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
    // (3) a class *declaration* emits bare — NO trailing `;` after the closing
    //     `}` and NO wrapping paren (a WHITESPACE_ONLY fallback appends `;`).
    let t = actual.trim_end_matches('\n');
    assert!(
        t.ends_with('}') && !t.ends_with("};") && !t.starts_with('('),
        "class declaration must emit bare (no trailing `;`, no wrap): {actual}"
    );
}
