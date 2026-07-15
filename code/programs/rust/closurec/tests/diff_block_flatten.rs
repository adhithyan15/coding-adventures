//! Integration test for the `tests/diff/block-flatten/` fixture.
//!
//! Exercises redundant-`BlockStatement` flattening end-to-end — the CLOC12.194
//! arc. Before it, when an `if` with a statically-decidable condition folded to
//! its surviving branch, closurec left that branch's `{ … }` block intact
//! (`{ b(); }`); the reference Closure Compiler removes the redundant braces and
//! runs the statement directly (`b();`).
//!
//! The arc landed in two PRs:
//!   - PR1 (#8345): the flatten itself, in `closure-pass-fold-control-flow`
//!     (`fold_program` + `fold_block_statement`). A bare `{ … }` block at
//!     statement-list position with no block-scoped binding is spliced into the
//!     enclosing list; the soundness gate `block_is_scope_safe_to_hoist` keeps
//!     any block declaring `let`/`const`/`class`/a `function`. Verified
//!     byte-identical to the real Closure jar.
//!   - PR2 (this test): the closurec end-to-end proof.
//!
//! ## Fact — SIMPLE: the folded branch's block flattens away
//!
//! `if (2 > 3) { a(); } else { b(); }` at SIMPLE emits `b();`. Two passes
//! compose: `constant-fold` turns `2 > 3` into `false`, `fold-control-flow`
//! keeps the `else` branch, and then the redundant `{ b(); }` block flattens to
//! `b();`. The proof the whole pipeline ran (not a WHITESPACE_ONLY fallback,
//! which would keep the entire `if`/`else` verbatim) is that the output is the
//! single bare statement `b();` — no `if`, and crucially no surrounding block
//! braces (`{b()`).

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw = std::fs::read_to_string("tests/diff/block-flatten/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn redundant_block_flattens_after_branch_fold() {
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
    let expected = std::fs::read_to_string("tests/diff/block-flatten/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    let flat = actual.replace([' ', '\n'], "");
    // The `else` branch was kept AND its redundant block flattened: `b();`, not
    // `{b()}`. The absence of `{b(` is the proof the block braces were removed
    // (CLOC12.194); the presence of `b()` proves the pipeline optimized rather
    // than falling back to WHITESPACE_ONLY (which keeps the whole `if`/`else`).
    assert!(
        flat.contains("b()"),
        "kept `else` branch missing — pipeline may have fallen back to WHITESPACE_ONLY: {actual}"
    );
    assert!(
        !flat.contains("{b("),
        "redundant block braces still present — block did not flatten: {actual}"
    );
    assert!(
        !flat.contains("if("),
        "the dead `if` survived — constant-fold/fold-control-flow did not run: {actual}"
    );
}
