//! Integration test for the `tests/diff/simple-export/` fixture.
//!
//! Exercises ES-module `export` declarations end-to-end — the CLOC12.189 arc.
//! Before it, `export` landed in the parser bridge's *unsupported* bucket, so
//! any file with a top-level `export` DECLINED to WHITESPACE_ONLY (no
//! optimization at all). The arc landed in three PRs:
//!   - PR1: the `ExportNamedDeclaration` / `ExportDefaultDeclaration` /
//!     `ExportAllDeclaration` AST nodes + emitter (`emit_export_named` /
//!     `emit_export_default` / `emit_export_all`) + pass traversal (atomic; the
//!     nodes were unreachable);
//!   - PR2: the parser-bridge flip (`convert_export_declaration`) that makes an
//!     `export` reach the AST, together with the renaming-soundness gate — an
//!     `export`ed name is part of the module's *public* interface, so renaming
//!     it would break a foreign importer; the rename / rename-globals /
//!     rename-properties passes therefore *decline to rename* when a module
//!     `export` is present (`program_contains_export_declaration`);
//!   - PR3 (this test): the closurec end-to-end proof.
//!
//! ## Fact 1 — SIMPLE: the `export` survives and the pipeline optimizes past it
//!
//! `export { a, b as c } from "y"; d(4 + 5);` at SIMPLE emits
//! `export{a,b as c}from"y";d(9);`. Two things prove the whole pipeline ran:
//!   1. the re-`export` round-trips (`export{a,b as c}from"y";`), proving the
//!      bridge modelled it — not a WHITESPACE_ONLY fallback; and
//!   2. the trailing call argument folds — `4 + 5` → `9` — proving the SIMPLE
//!      pipeline ran the rest of the module. A WHITESPACE_ONLY fallback would
//!      leave `4+5` intact (it would emit the source verbatim).
//!
//! ## Fact 2 — ADVANCED: the soundness gate leaves an `export` program un-renamed
//!
//! `export { a } from "y"; function longFn(longParam){…} longFn(1);longFn(2);longFn(3);`
//! at ADVANCED keeps `longFn` and `longParam` verbatim, because the PR2 gate
//! disables the renaming passes whenever a module `export` is present. The
//! identical program WITHOUT the `export` renames all the way down to
//! `function b(a){…}` (the retained multi-use function and its param both get
//! short names), so the surviving long names are a direct, end-to-end
//! observation of the gate firing — not merely "nothing to rename".

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

fn read_flags() -> Vec<String> {
    let raw =
        std::fs::read_to_string("tests/diff/simple-export/flags.txt").expect("read flags.txt");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn simple_export_round_trips_and_folds() {
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
    let expected = std::fs::read_to_string("tests/diff/simple-export/expected.stdout")
        .expect("read expected.stdout");
    assert_eq!(
        actual.trim_end_matches('\n'),
        expected.trim_end_matches('\n'),
        "mismatch.\nactual:\n{actual}\nexpected:\n{expected}",
    );

    // (1) the re-`export` round-tripped — the bridge modelled it (not
    // WHITESPACE_ONLY). Checked on the raw output: the `b as c` alias carries a
    // required space, so we must NOT strip spaces here.
    let flat = actual.replace('\n', "");
    assert!(
        flat.contains("export{a,b as c}from\"y\";"),
        "`export` did not round-trip: {actual}"
    );
    // (2) the pipeline optimized the rest of the module: `4+5` folded to `9`.
    let a = actual.replace(' ', "");
    assert!(
        a.contains("d(9)"),
        "argument did not fold to `d(9)`: {actual}"
    );
    assert!(
        !a.contains("4+5"),
        "unfolded arithmetic present — pipeline fell back to WHITESPACE_ONLY: {actual}"
    );
}

#[test]
fn advanced_export_disables_renaming() {
    // ADVANCED on a program that contains a renameable multi-use function AND a
    // module `export`. The PR2 soundness gate must fire, leaving `longFn` and
    // `longParam` un-renamed.
    let out = Command::new(BINARY)
        .args([
            "--js",
            "tests/diff/simple-export/input/adv.js",
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
    // The `export` bridged (not WHITESPACE_ONLY)…
    assert!(
        a.contains("export{a}from\"y\""),
        "`export` did not round-trip: {actual}"
    );
    // …and the gate left the otherwise-renameable names intact. (The identical
    // program without the `export` renames down to `function b(a){…}`.)
    assert!(
        a.contains("longFn") && a.contains("longParam"),
        "gate did not fire — names were renamed under an `export`: {actual}"
    );
}
