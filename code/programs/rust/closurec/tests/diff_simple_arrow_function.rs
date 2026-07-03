//! Integration test for the `tests/diff/simple-arrow-function/` fixture.
//!
//! Exercises CLOC12.152 — a concise-body **arrow function** (`x => expr`) now
//! flows through the full SIMPLE pipeline end-to-end (parser → typed-AST bridge
//! → passes → emitter) instead of declining at the bridge and falling back to
//! WHITESPACE_ONLY.
//!
//! The fixture puts a concise-body arrow in several positions — a single-param
//! RHS (`var f = n => 1+2`), a multi-param RHS (`var g = (a,b) => a+3*4`), and
//! a callback argument (`arr.map(x => x + (5-1))`) — each with a foldable body.
//! The expected output proves constant-fold ran *inside* those arrow bodies
//! (`1+2`→`3`, `3*4`→`12`, `5-1`→`4`), which a WHITESPACE_ONLY passthrough
//! would NOT do.
//!
//! Block-bodied arrows (`x => { return x; }`) are blocked on a grammar
//! limitation (CLOC12-gaps gap-156) and object-body arrows decline to avoid the
//! `() => {}` empty-block-vs-object ambiguity — neither appears here.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-arrow-function/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_arrow_function_fixture_matches_expected_stdout() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-arrow-function/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // Guard the *point* of the fixture: the arrow bodies were optimised, so the
    // folded constants must be present and the pre-fold expressions gone.
    // (A WHITESPACE_ONLY fallback would still contain `1+2` / `3*4` / `5-1`.)
    let a = actual.replace(' ', "");
    assert!(a.contains("=>3"), "1+2 not folded inside the arrow: {actual}");
    assert!(a.contains("+12"), "3*4 not folded inside the arrow: {actual}");
    assert!(a.contains("+4"), "5-1 not folded inside the arrow: {actual}");
    assert!(
        !a.contains("1+2") && !a.contains("3*4") && !a.contains("5-1"),
        "a pre-fold expression survived — did it fall back to WHITESPACE_ONLY? {actual}"
    );
}
