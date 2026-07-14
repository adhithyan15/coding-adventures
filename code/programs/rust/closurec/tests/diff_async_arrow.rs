//! Integration test for the `tests/diff/async-arrow/` fixture.
//!
//! Exercises ES async arrow functions (`async () => expr`) end-to-end — the
//! CLOC12.192 arc. Before it, an async arrow landed in the parser bridge's
//! *unsupported* bucket (`async_arrow_function` was in the decline list), so any
//! file with one DECLINED to WHITESPACE_ONLY (no optimization at all). The arc
//! landed in two PRs — a bridge-only enable, because the AST
//! (`ArrowFunctionExpression.is_async`) and emitter (prints `async`) already
//! modelled async arrows:
//!   - PR1 (#8318): the parser-bridge enable. The grammar rule
//!     `async_arrow_function = "async" arrow_parameters ARROW concise_body` is
//!     the plain arrow shape plus a leading `async` literal, so
//!     `convert_arrow_function` (now taking an `is_async` flag) handles it
//!     unchanged and just sets the flag; the node is dispatched there instead of
//!     declining.
//!   - PR2 (this test): the closurec end-to-end proof.
//!
//! ## Fact — SIMPLE: the async arrow body folds and the pipeline optimizes
//!
//! `var f=async()=>1+2; g(f);` at SIMPLE emits `var f=async()=>3;g(f);`. The
//! proof that the whole pipeline ran — and did NOT fall back to WHITESPACE_ONLY
//! — is that the async arrow's concise body folds: `1 + 2` → `3`. A
//! WHITESPACE_ONLY fallback would emit the source verbatim (only stripping
//! whitespace), leaving `async()=>1+2` intact. The `async` keyword round-trips,
//! proving the bridge modelled the async flag rather than dropping it.
//!
//! `g(f)` consumes the arrow-valued `f` so the binding is retained in the
//! output rather than being removed as unused.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/async-arrow/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn async_arrow_folds_and_optimizes() {
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
    let expected = std::fs::read_to_string("tests/diff/async-arrow/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // The async arrow's concise body folded: `1 + 2` → `3`. This is the proof
    // the file OPTIMIZED (not a WHITESPACE_ONLY fallback, which would keep
    // `async()=>1+2`). The `async` keyword round-trips too. Checked on
    // space-stripped output.
    assert!(
        flat.contains("async()=>3"),
        "async arrow body did not fold to `3` — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}
