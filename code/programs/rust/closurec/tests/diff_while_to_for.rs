//! Integration test for the `tests/diff/while-to-for/` fixture.
//!
//! Exercises the `while (cond) body` → `for (; cond; ) body` canonicalization
//! end-to-end — `closure-pass-fold-control-flow`'s `fold_while_statement` now
//! rewrites every live `while` loop to the equivalent `for`, the form the
//! reference Closure Compiler always emits.
//!
//! ## Fact — SIMPLE: `while(x)a();` becomes `for(;x;)a();`
//!
//! A `while` and a `for` with an empty init *and* empty update are exactly
//! equivalent: no init runs, and `continue` targets the test in both (there is
//! no update clause to fall through to). So this is a pure spelling change.
//! Byte-identical to the reference Closure Compiler. The proof the pipeline ran
//! (not a WHITESPACE_ONLY fallback, which would keep `while(x)a();` verbatim)
//! is that the output spells the loop `for`.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/while-to-for/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn while_loop_canonicalizes_to_for() {
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
    let expected = std::fs::read_to_string("tests/diff/while-to-for/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // `while(x)a();` was rewritten to `for(;x;)a();`. The presence of `for(;x;)`
    // and absence of the `while` keyword prove the pipeline optimized (not a
    // WHITESPACE_ONLY fallback, which would keep `while(x)a();`).
    assert!(
        flat.contains("for(;x;)"),
        "while was not canonicalized to `for` — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("while"),
        "`while` keyword still present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
