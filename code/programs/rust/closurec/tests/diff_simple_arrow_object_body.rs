//! Integration test for the `tests/diff/simple-arrow-object-body/` fixture.
//!
//! Exercises a **parenthesised object-body arrow** `() => ({…})` end-to-end at
//! SIMPLE — the CLOC12.185 bridge extension. The grammar buckets the braces of
//! both a block body `=> {…}` and a parenthesised object body `=> ({…})` as an
//! `object_literal`; CLOC12.184 disambiguated them by the concise_body's leftmost
//! token (`{` = block, `(` = object expression). CLOC12.185 now BRIDGES the
//! `(`-leading object expression body (previously declined) — the emitter already
//! re-wraps an object arrow body in parens so it is never misread as a block.
//!
//! The fixture is `x = () => ({a: 1 + 2});` compiled at SIMPLE. Two facts prove
//! the whole pipeline ran through the object-body arrow:
//!   1. the object body round-trips **parenthesised** — `()=>({a:…})`, proving
//!      the bridge modelled it (not a WHITESPACE_ONLY fallback) AND the emitter
//!      kept the parens so it stays an object expression, not a block; and
//!   2. the value folds — `1 + 2` → `3` — proving the SIMPLE pipeline descended
//!      INTO the object property value. A WHITESPACE_ONLY fallback would leave
//!      `1+2` intact.
//! Before this change the object-body arrow DECLINED, dropping the file to
//! WHITESPACE_ONLY (`x=()=>({a:1+2});`) and assertion (2) failed.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-arrow-object-body/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_arrow_object_body_round_trips_and_folds() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-arrow-object-body/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let a = actual.replace(' ', "");
    // (1) the object body round-tripped PARENTHESISED — `()=>({a:...})`. The
    //     parens are load-bearing: `()=>{a:...}` would be read as a block.
    assert!(
        a.contains("()=>({a:"),
        "object-body arrow did not round-trip parenthesised: {actual}"
    );
    // (2) the property value folded — proving the pipeline descended INTO the
    //     object body (`1+2`→`3`). A WHITESPACE_ONLY fallback would leave the
    //     arithmetic intact.
    assert!(
        a.contains("({a:3})"),
        "object-body property did not fold to `3`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
