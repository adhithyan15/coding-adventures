//! Integration test for the `tests/diff/drop-new-error/` fixture.
//!
//! Exercises the standard-constructor `new`-drop end-to-end —
//! `closure-pass-constant-fold` rewrites `new Error(args)` to a plain call
//! `Error(args)`. Calling the built-in `Error` as an ordinary function
//! constructs an Error identically to `new` (ECMAScript §20.5.1.1), so the drop
//! is semantics-preserving and byte-identical to the reference Closure Compiler.
//!
//! ## Fact — SIMPLE: `throw new Error("boom");` → `throw Error("boom");`
//!
//! The proof the pipeline ran (not a WHITESPACE_ONLY fallback, which would keep
//! `new Error("boom")` verbatim) is the absence of the `new` keyword.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/drop-new-error/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn new_error_drops_to_plain_call() {
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
    let expected = std::fs::read_to_string("tests/diff/drop-new-error/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // `new Error("boom")` → `Error("boom")`. The presence of `Error("boom")` and
    // absence of the `new` keyword prove the pipeline optimized (not a
    // WHITESPACE_ONLY fallback, which would keep `new Error("boom")`).
    assert!(
        flat.contains(r#"Error("boom")"#),
        "Error call missing — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("new"),
        "`new` keyword still present — the constructor `new`-drop did not run: {actual}"
    );
}
