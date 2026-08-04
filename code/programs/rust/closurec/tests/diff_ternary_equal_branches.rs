//! Integration test for the `tests/diff/ternary-equal-branches/` fixture.
//!
//! Exercises the ternary equal-branch collapse end-to-end — the
//! `closure-pass-constant-fold` 0.101.0 arc.
//!
//! ## Why this fold is sound
//!
//! `t ? X : X` selects `X` no matter how `t` decides, because BOTH arms are
//! the same expression — the branch on `t` is dead. The only behaviour the
//! rewrite must not drop is `t`'s own evaluation, so the fold fires only when
//! `t` is side-effect-free (an identifier, literal, or member read; a call or
//! assignment is not). The reference Closure Compiler performs exactly this at
//! `SIMPLE`, and for an impure test instead builds the comma sequence
//! `(t, X)` — a larger transform this pass deliberately declines (leaving the
//! ternary intact, which is sound).
//!
//! ## Fact — SIMPLE: `g(a ? b : b)` → `g(b)`
//!
//! `g(a ? b : b);` at SIMPLE emits `g(b);`: the test `a` is a side-effect-free
//! identifier and both arms are the identifier `b`, so the ternary collapses to
//! `b`. A WHITESPACE_ONLY fallback emits `g(a?b:b);` verbatim (only stripping
//! whitespace), so the presence of `g(b)` and the absence of the `?`/`:` prove
//! the optimization pipeline ran. Verified byte-identical to the real Closure
//! jar across the fold's truth table (`a?b:b`, `a?1:1`, `a?b.c:b.c`,
//! `a?b():b()`, `a.p?b:b`).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/ternary-equal-branches/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn ternary_with_equal_branches_collapses_to_the_branch() {
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
    let expected = std::fs::read_to_string("tests/diff/ternary-equal-branches/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // The ternary collapsed: `a?b:b` → `b`. This is the proof the file
    // OPTIMIZED (not a WHITESPACE_ONLY fallback, which keeps `a?b:b`). Checked
    // on space-stripped output.
    assert!(
        flat.contains("g(b)"),
        "ternary did not collapse to `b` — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains('?'),
        "the `?:` survived — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
