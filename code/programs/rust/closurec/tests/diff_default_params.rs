//! Integration test for the `tests/diff/default-params/` fixture.
//!
//! Exercises ES default parameters (`function f(a = expr){}`) end-to-end — the
//! CLOC12.191 arc. Before it, a default parameter landed in the parser bridge's
//! *unsupported* bucket (`convert_formal_parameter` declined the `EQUALS`
//! branch), so any file with a default parameter DECLINED to WHITESPACE_ONLY
//! (no optimization at all). The arc landed in three PRs:
//!   - PR1 (#8284): the `FunctionParam::AssignmentPattern` AST variant + emitter
//!     arm (`name=expr`) + the pass-traversal threading — the default's `right`
//!     is *live code* that constant-fold folds, the renamers rewrite, and inline
//!     declines around (atomic; the variant was unreachable);
//!   - PR2 (#8295): the parser-bridge flip (`convert_formal_parameter` maps a
//!     `NAME = assignment_expression` parameter to an `AssignmentPattern`, while
//!     a destructuring default `{x} = {}` still declines);
//!   - PR3 (this test): the closurec end-to-end proof.
//!
//! ## Fact — SIMPLE: the default folds and the pipeline optimizes
//!
//! `function f(a=1+2){return a} g(f());` at SIMPLE emits
//! `function f(a=3){return a};g(f());`. The proof that the whole pipeline ran —
//! and did NOT fall back to WHITESPACE_ONLY — is that the DEFAULT expression
//! itself folds: `a = 1 + 2` → `a = 3`. A WHITESPACE_ONLY fallback would emit
//! the source verbatim (only stripping whitespace), leaving `a=1+2` intact. This
//! is the CLOC12.191 headline: unlike a rest parameter (which binds only a
//! name), a default parameter carries a live expression that the optimizer walks
//! and folds exactly as it would a function body.
//!
//! The single-use function `f` is *retained* (an unknown `g` consumes its
//! result via `g(f())`), keeping the `a=3` default parameter visible in the
//! output rather than being inlined away.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/default-params/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn default_params_fold_and_optimize() {
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
    let expected = std::fs::read_to_string("tests/diff/default-params/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // The default expression folded: `a = 1 + 2` → `a = 3`. This is the proof
    // the file OPTIMIZED (not a WHITESPACE_ONLY fallback, which would keep
    // `a=1+2`). Checked on space-stripped output.
    assert!(
        flat.contains("f(a=3)"),
        "default did not fold to `a=3` — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
