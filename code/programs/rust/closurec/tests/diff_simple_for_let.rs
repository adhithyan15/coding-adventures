//! Integration test for the `tests/diff/simple-for-let/` fixture.
//!
//! Exercises a **lexical (`let`/`const`) init in a C-style `for` header** —
//! `for (let i = 0; …)` — end-to-end at SIMPLE, the CLOC12.186 bridge fix. The
//! grammar inlines the lexical declaration into the for-header (the `let`/`const`
//! keyword is a direct token of the `for_statement`, the bindings a bare
//! `binding_list` node). `convert_for_statement` previously only handled a `var`
//! init (`variable_declaration_list`) and routed the `binding_list` into
//! `convert_expression`, raising an InternalError that declined the whole file to
//! WHITESPACE_ONLY. The `ForInit::VariableDeclaration` AST variant already carries
//! the var/let/const kind, so this is a pure bridge fix.
//!
//! The fixture is `for (let i = 0; i < 1 + 2; i++) x();` compiled at SIMPLE.
//! Two facts prove the whole pipeline ran through the `for (let …)`:
//!   1. the loop round-trips with a `let` init — `for(let i=0;…`, proving the
//!      bridge modelled it (not a WHITESPACE_ONLY fallback); and
//!   2. the test folds — `1 + 2` → `3` — proving the SIMPLE pipeline descended
//!      INTO the loop header. A WHITESPACE_ONLY fallback would leave `1+2` intact.
//! Before this fix the `for (let …)` DECLINED, dropping the file to
//! WHITESPACE_ONLY (`for(let i=0;i<1+2;i++)x();`) and assertion (2) failed.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-for-let/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_for_let_round_trips_and_folds() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-for-let/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let a = actual.replace(' ', "");
    // (1) the loop round-tripped with a `let` init.
    assert!(
        a.contains("for(leti=0;"),
        "for-let did not round-trip with a `let` init: {actual}"
    );
    // (2) the test folded — proving the pipeline descended INTO the loop header
    //     (`1+2`→`3`). A WHITESPACE_ONLY fallback would leave the arithmetic
    //     intact.
    assert!(
        a.contains("i<3;"),
        "loop test did not fold to `i<3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
