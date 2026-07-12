//! Integration test for the `tests/diff/simple-logical-assign/` fixture.
//!
//! Exercises an **ES2021 logical assignment operator** (`||=`, an
//! `AssignmentExpression` with `operator: LogicalOrEq`) end-to-end at SIMPLE —
//! the CLOC12.183 bridge of the `&&=` / `||=` / `??=` operators. These parse
//! fine but the bridge previously mapped them to an `InternalError`
//! ("unknown assignment operator"), dropping the whole file to WHITESPACE_ONLY.
//! The AST gained the three operator variants, the bridge maps the tokens, and
//! the emitter reprints them.
//!
//! The fixture is `x ||= 1 + 2;` compiled at SIMPLE. Two facts prove the whole
//! pipeline ran through the operator:
//!   1. the `||=` operator round-trips — proving the bridge mapped it to
//!      `LogicalOrEq` (not a WHITESPACE_ONLY fallback); and
//!   2. the RHS folds — `1 + 2` → `3` — proving the SIMPLE pipeline descended
//!      INTO the assignment's right-hand side. A WHITESPACE_ONLY fallback would
//!      leave `1+2` intact.
//! Before this bridge change the operator declined, dropping the file to
//! WHITESPACE_ONLY (`x||=1+2;`) and assertion (2) failed.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-logical-assign/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_logical_assign_folds_rhs() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-logical-assign/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let a = actual.replace(' ', "");
    // (1) the `||=` operator round-tripped.
    assert!(
        a.contains("||="),
        "logical-assign operator `||=` did not round-trip: {actual}"
    );
    // (2) the RHS folded — proving the pipeline descended INTO the assignment
    //     right-hand side (`1+2`→`3`). A WHITESPACE_ONLY fallback would leave the
    //     arithmetic intact.
    assert!(
        a.contains("||=3"),
        "logical-assign RHS did not fold to `3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
