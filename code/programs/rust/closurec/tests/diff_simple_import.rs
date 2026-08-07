//! Integration test for the `tests/diff/simple-import/` fixture.
//!
//! Exercises ES-module `import` declarations end-to-end — the CLOC12.188 arc.
//! Before it, `import` landed in the parser bridge's *unsupported* bucket, so
//! any file with a top-level `import` DECLINED to WHITESPACE_ONLY (no
//! optimization at all). The arc landed in three PRs:
//!   - PR1: the `ImportDeclaration` AST node + emitter (`emit_import`) + pass
//!     traversal (atomic; the node was unreachable);
//!   - PR2: the parser-bridge flip (`convert_import_declaration`) that makes an
//!     `import` reach the AST, together with the renaming-soundness gate — an
//!     imported name aliases a *foreign module's* export, so renaming it (or
//!     colliding an unrelated local into it) is unsound; the rename /
//!     rename-globals / rename-properties passes therefore *decline to rename*
//!     when a module `import` is present (`program_contains_import_declaration`);
//!   - PR3 (this test): the closurec end-to-end proof.
//!
//! ## Fact 1 — SIMPLE: the `import` survives and the pipeline optimizes past it
//!
//! `import {a, b as c} from "y"; a(1 + 2);` at SIMPLE emits
//! `import{a,b as c}from"y";a(3);`. Two things prove the whole pipeline ran:
//!   1. the `import` round-trips (`import{a,b as c}from"y";`), proving the
//!      bridge modelled it — not a WHITESPACE_ONLY fallback; and
//!   2. the call argument folds — `1 + 2` → `3` — proving the SIMPLE pipeline
//!      ran the rest of the module. A WHITESPACE_ONLY fallback would leave
//!      `1+2` intact (it would emit the source verbatim).
//!
//! ## Fact 2 — ADVANCED: the soundness gate leaves an `import` program un-renamed
//!
//! `import {a} from "y"; function longFn(longParam){…} longFn(1);longFn(2);longFn(3);`
//! at ADVANCED keeps `longFn` and `longParam` verbatim, because the PR2 gate
//! disables the renaming passes whenever a module `import` is present. The
//! identical program WITHOUT the `import` renames all the way down to
//! `function b(a){…}` (the retained multi-use function and its param both get
//! short names), so the surviving long names are a direct, end-to-end
//! observation of the gate firing — not merely "nothing to rename".

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-import/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_import_round_trips_and_folds() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-import/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // (1) the `import` round-tripped — the bridge modelled it (not
    // WHITESPACE_ONLY). Checked on the raw output: the `b as c` alias carries a
    // required space, so we must NOT strip spaces here.
    let flat = actual.replace('\n', "");
    assert!(
        flat.contains("import{a,b as c}from\"y\";"),
        "`import` did not round-trip: {actual}"
    );
    // (2) the pipeline optimized the rest of the module: `1+2` folded to `3`.
    let a = actual.replace(' ', "");
    assert!(
        a.contains("a(3)"),
        "argument did not fold to `a(3)`: {actual}"
    );
    assert!(
        !a.contains("1+2"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}

#[test]
fn advanced_import_disables_renaming() {
    // ADVANCED on a program that contains a renameable multi-use function AND a
    // module `import`. The PR2 soundness gate must fire, leaving `longFn` and
    // `longParam` un-renamed.
    let out = Command::new(BINARY)
        .args([
            "--js",
            "tests/diff/simple-import/input/adv.js",
            "--compilation_level",
            "ADVANCED",
        ])
        .output()
        .expect("run closurec");

    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );

    let actual = String::from_utf8_lossy(&out.stdout);
    let a = actual.replace(' ', "");
    // The `import` bridged (not WHITESPACE_ONLY)…
    assert!(
        a.contains("import{a}from\"y\""),
        "`import` did not round-trip: {actual}"
    );
    // …and the gate left the otherwise-renameable names intact. (The identical
    // program without the `import` renames down to `function b(a){…}`.)
    assert!(
        a.contains("longFn") && a.contains("longParam"),
        "gate did not fire — names were renamed under an `import`: {actual}"
    );
}
