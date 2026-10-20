//! The VisiCalc web demos carry generated code, and it must be what the
//! generators produce from this package today.
//!
//! `code/programs/typescript/visicalc-html` and `visicalc-webcomp` open
//! straight from disk, so their compiled pieces are committed:
//!
//! | committed file                              | generated from                          |
//! |---------------------------------------------|-----------------------------------------|
//! | `visicalc-html/index.html` `<template>`s    | this package's FormulaBar and Grid, HTML |
//! | `visicalc-webcomp/vendor/{FormulaBar,Grid}.js` | the same, as Web Components           |
//! | `visicalc-html/vendor/spreadsheet-engine-wasm.js` | `spreadsheet-wasm`'s committed `.wasm` |
//!
//! They used to be regenerated and copied in by hand, and all of them had
//! drifted: the pages lacked the cell editor's geometry and name (#15048),
//! and the engine predated spreadsheet-core fixes such as #15541. Each demo's
//! script now writes its own files and has a `--check` mode; these tests run
//! those checks. They live here, not in the demos' BUILD files, because this
//! crate is rebuilt whenever `mosaic-compile` or `spreadsheet-wasm` changes
//! (see BUILD), which is exactly when the demos can go stale.
//!
//! The scripts are bash, so the tests are Unix-only; a failure message names
//! the command that regenerates the files.

#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

fn typescript_programs() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../typescript")
}

/// Run `program args...` from `dir` and fail with its output if it fails.
fn check(dir: &str, program: &str, args: &[&str]) {
    let cwd = typescript_programs().join(dir);
    let output = Command::new(program)
        .args(args)
        .current_dir(&cwd)
        .output()
        .unwrap_or_else(|err| panic!("run {program} {args:?} in {}: {err}", cwd.display()));
    assert!(
        output.status.success(),
        "{dir}: `{program} {}` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_html_demo_embeds_todays_fragments() {
    check("visicalc-html", "bash", &["scripts/build.sh", "--check"]);
}

#[test]
fn the_web_component_demo_ships_todays_elements() {
    check("visicalc-webcomp", "bash", &["scripts/build.sh", "--check"]);
}

#[test]
fn the_html_demo_embeds_the_committed_engine() {
    check("visicalc-html", "bash", &["scripts/bundle-wasm-engine.sh", "--check"]);
}

/// The infinite-grid smoke test drives the embedded engine through Node's
/// `vm`, so it needs only `node`. Without one there is nothing to run it with.
#[test]
fn the_embedded_engine_passes_the_infinite_grid_smoke_test() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("skipping: node is not installed");
        return;
    }
    check("visicalc-html", "node", &["scripts/verify-infinite.mjs"]);
}
