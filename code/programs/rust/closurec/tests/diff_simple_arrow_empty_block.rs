//! Integration test for the `tests/diff/simple-arrow-empty-block/` fixture.
//!
//! Exercises the **empty-block arrow** `() => {}` end-to-end at SIMPLE — the
//! CLOC12.184 bridge fix. The grammar buckets the bare `{}` after `=>` as an
//! empty `object_literal`, but per the ES spec a `{` immediately after `=>`
//! ALWAYS opens a block body. `convert_arrow_function` now reinterprets a bare
//! empty object-literal concise body as an `ArrowBody::Block` with no
//! statements (distinguished from the parenthesised object body `() => ({})` by
//! the concise_body's leftmost token), instead of declining the whole file to
//! WHITESPACE_ONLY.
//!
//! The fixture is `x = () => {}; y = 1 + 2;` compiled at SIMPLE. Two facts prove
//! the whole pipeline ran:
//!   1. the `()=>{}` arrow round-trips — proving the bridge modelled it rather
//!      than declining; and
//!   2. the sibling folds — `1 + 2` → `3` — proving the SIMPLE pipeline ran over
//!      the whole program. Before this fix the arrow DECLINED, dropping the
//!      ENTIRE file to WHITESPACE_ONLY, so `y=1+2` would NOT have folded.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/simple-arrow-empty-block/flags.txt")
        .expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_arrow_empty_block_round_trips_and_folds() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-arrow-empty-block/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let a = actual.replace(' ', "");
    // (1) the empty-block arrow round-tripped as `()=>{}`.
    assert!(
        a.contains("()=>{}"),
        "empty-block arrow did not round-trip as `()=>{{}}`: {actual}"
    );
    // (2) the sibling folded — proving the whole program ran through the SIMPLE
    //     pipeline (`1+2`→`3`). A WHITESPACE_ONLY fallback (which the declining
    //     arrow would have forced for the ENTIRE file) would leave `1+2` intact.
    assert!(
        a.contains("y=3"),
        "sibling did not fold to `y=3` — whole file fell back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
