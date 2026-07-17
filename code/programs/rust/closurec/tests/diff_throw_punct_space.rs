//! Integration test for the `tests/diff/throw-punct-space/` fixture.
//!
//! Exercises the `throw`/`return` keyword-before-punctuation spacing rule
//! end-to-end — `closure-emitter` emits no separating space when the argument
//! begins with punctuation (`throw{a:1}`, `throw"x"`, `throw[1]`), matching the
//! reference Closure Compiler, while keeping the space where a word token would
//! fuse (`throw x`, `throw 5`).
//!
//! ## Fact — SIMPLE: `throw {a:1};` → `throw{a:1};`
//!
//! The distinguishing output is the absence of the space between `throw` and
//! the object literal `{`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/throw-punct-space/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn throw_before_object_literal_has_no_space() {
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
    let expected = std::fs::read_to_string("tests/diff/throw-punct-space/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // The whole point: `throw` immediately followed by `{`, no space.
    assert!(
        actual.contains("throw{"),
        "expected `throw{{` with no space; got: {actual}"
    );
    assert!(
        !actual.contains("throw {"),
        "a space remained after `throw` before the object literal: {actual}"
    );
}
