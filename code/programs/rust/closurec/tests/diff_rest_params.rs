//! Integration test for the `tests/diff/rest-params/` fixture.
//!
//! Exercises ES rest parameters (`function f(...args){}`) end-to-end — the
//! CLOC12.190 arc. Before it, a rest parameter landed in the parser bridge's
//! *unsupported* bucket (`convert_formal_parameter` declined any `...`), so any
//! file with a rest parameter DECLINED to WHITESPACE_ONLY (no optimization at
//! all). The arc landed in three PRs:
//!   - PR1 (#8233): the `FunctionParam::RestElement` AST variant + emitter arm
//!     (`...name`) + the pass-traversal arms across scope-analyzer / rename /
//!     inline (atomic; the variant was unreachable);
//!   - PR2 (#8241): the parser-bridge flip (`convert_formal_parameter` maps an
//!     `ELLIPSIS` parameter to a `RestElement`, while a destructuring rest
//!     target `...[a,b]` still declines);
//!   - PR3 (this test): the closurec end-to-end proof.
//!
//! ## Fact — SIMPLE: the rest parameter survives and the pipeline optimizes
//!
//! `function f(...a){return a.length} g(f(1 + 2, 3));` at SIMPLE emits
//! `function f(...a){return a.length};g(f(3,3));`. Two things prove the whole
//! pipeline ran:
//!   1. the rest parameter round-trips (`function f(...a){…}`), proving the
//!      bridge modelled it — not a WHITESPACE_ONLY fallback; and
//!   2. the call argument folds — `1 + 2` → `3` (`f(3,3)`) — proving the SIMPLE
//!      pipeline ran the rest of the module. A WHITESPACE_ONLY fallback would
//!      leave `1+2` intact (it would emit the source verbatim, only stripping
//!      whitespace).
//!
//! The call is wrapped in `g(...)` so the single-use function `f` is *retained*
//! (an unknown `g` consumes its result), keeping the `...a` parameter visible in
//! the output rather than being inlined away.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/rest-params/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn rest_params_round_trip_and_fold() {
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
    let expected = std::fs::read_to_string("tests/diff/rest-params/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace(' ', "").replace('\n', "");
    // (1) the rest parameter round-tripped — the bridge modelled it (not
    // WHITESPACE_ONLY). The `...a` prefix must survive on the emitted function.
    // (Checked on space-stripped output, so no space between `function` and `f`.)
    assert!(
        flat.contains("f(...a){"),
        "rest parameter did not round-trip: {actual}"
    );
    // (2) the pipeline optimized the rest of the module: `1+2` folded to `3`.
    assert!(
        flat.contains("f(3,3)"),
        "argument did not fold to `f(3,3)`: {actual}"
    );
    assert!(
        !flat.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
