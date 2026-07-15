//! Integration test for the `tests/diff/template-literal-fold/` fixture.
//!
//! Exercises the CLOC12.197 template-literal → string fold end-to-end. A
//! no-substitution template literal is a compile-time-known string, so
//! `closure-pass-constant-fold` collapses it to a plain string literal, matching
//! the reference Closure Compiler.
//!
//! ## Fact — SIMPLE: a no-sub template folds to a string
//!
//! `g(`hi`);` at SIMPLE emits `g("hi");` — the backtick template becomes a
//! double-quoted string literal. The proof the optimization pipeline ran (not a
//! WHITESPACE_ONLY fallback, which would keep `` `hi` `` verbatim) is the
//! presence of `"hi"` and the absence of any backtick.
//!
//! *Substituted* templates (`` `a${x}b` ``) do not yet parse in closurec's
//! grammar, so this fixture uses the no-substitution form — the only shape the
//! fold can currently receive end-to-end.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/template-literal-fold/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn no_substitution_template_folds_to_string() {
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
    let expected = std::fs::read_to_string("tests/diff/template-literal-fold/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // The template folded to a string: `"hi"` present, no backtick left.
    assert!(
        flat.contains("\"hi\""),
        "template did not fold to a string — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains('`'),
        "a backtick template survived — the CLOC12.197 fold did not run: {actual}"
    );
}
