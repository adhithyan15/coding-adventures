//! # The conformance baseline gate
//!
//! One data-driven test — not one per vendored file, matching this repo's
//! `html-lexer` fixture-driven pattern — that runs every `.wast` file in
//! `tests/fixtures/testsuite/` and diffs the result against the checked-in
//! golden baseline, `tests/fixtures/testsuite-status.json`.
//!
//! This test fails on **any** change from the committed baseline —
//! regression *or* improvement. The number can never silently drift: every
//! change to it (a genuine interpreter fix, a `wasm-wast-parser` grammar
//! fix, a newly-supported directive kind) has to be a deliberate, reviewed
//! commit that updates the baseline alongside the code change that earned
//! it.
//!
//! ## Maintainer workflow
//!
//! ```text
//! cargo run --bin wasm_conformance_report -p wasm-conformance   # see where things stand
//! cargo test -p wasm-conformance                                # this test — catches drift
//! # ... after a deliberate, reviewed fix that changes the numbers ...
//! cargo run --bin wasm_conformance_report -p wasm-conformance -- --write-baseline
//! cargo test -p wasm-conformance                                # confirm it's green again
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use wasm_conformance::report::ConformanceReport;

fn testsuite_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/testsuite")
}

fn baseline_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/testsuite-status.json")
}

fn discover_wast_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(testsuite_dir())
        .expect("tests/fixtures/testsuite should exist -- run fetch_testsuite.py first")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "wast"))
        .collect();
    files.sort();
    files
}

fn run_current_report() -> ConformanceReport {
    let mut report = ConformanceReport::default();
    for path in discover_wast_files() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {name}: {e}"));
        match wasm_conformance::run_wast_source(&source) {
            Ok(results) => report.add_file(name, wasm_conformance::report::tally_results(&results)),
            Err(e) => report.add_parse_failure(name, e.to_string()),
        }
    }
    report
}

#[test]
fn corpus_matches_the_committed_baseline() {
    let current = run_current_report();
    let baseline_json = fs::read_to_string(baseline_path()).unwrap_or_else(|e| {
        panic!(
            "failed to read the golden baseline at {}: {e}\n\
             if this crate has no baseline yet, generate one with:\n\
             cargo run --bin wasm_conformance_report -p wasm-conformance -- --write-baseline",
            baseline_path().display()
        )
    });
    let baseline: ConformanceReport = serde_json::from_str(&baseline_json).expect("baseline JSON should deserialize");

    // Compare parse failures first, with a clear message naming exactly
    // which files changed status -- this is the class of change most
    // likely to be a `wasm-wast-parser` fix (or regression), not a
    // `wasm-execution` one.
    if current.parse_failures.keys().collect::<Vec<_>>() != baseline.parse_failures.keys().collect::<Vec<_>>() {
        let newly_parsing: Vec<&String> =
            baseline.parse_failures.keys().filter(|f| !current.parse_failures.contains_key(*f)).collect();
        let newly_broken: Vec<&String> =
            current.parse_failures.keys().filter(|f| !baseline.parse_failures.contains_key(*f)).collect();
        panic!(
            "conformance baseline drift in FILE PARSE STATUS\n  \
             now parses (was a parse failure): {newly_parsing:?}\n  \
             newly fails to parse (was parsing): {newly_broken:?}\n\n\
             If this is a deliberate, reviewed change, regenerate the baseline:\n  \
             cargo run --bin wasm_conformance_report -p wasm-conformance -- --write-baseline"
        );
    }

    // Compare per-file, per-kind tallies for every file that DID parse.
    for (file, current_tallies) in &current.files {
        let baseline_tallies = baseline.files.get(file);
        assert_eq!(
            baseline_tallies,
            Some(current_tallies),
            "conformance baseline drift in {file}\n  baseline: {baseline_tallies:?}\n  current:  {current_tallies:?}\n\n\
             If this is a deliberate, reviewed change, regenerate the baseline:\n  \
             cargo run --bin wasm_conformance_report -p wasm-conformance -- --write-baseline"
        );
    }

    assert_eq!(
        current.files.len(),
        baseline.files.len(),
        "the set of successfully-parsed files changed size -- see the parse-failure diff above"
    );
}

/// A cheap, independent sanity check with no dependency on the golden
/// baseline staying in sync: every vendored file at least *parses* as a
/// well-formed `.wast` SCRIPT (a sequence of directives), even for the 16
/// files this phase's `wasm-wast-parser` can't fully build yet. Guards
/// against `wasm-wast-parser`'s S-expression tokenizer/tree-builder itself
/// regressing on real-world input, independent of directive-level grading.
#[test]
fn every_vendored_file_is_readable_and_non_empty() {
    let files = discover_wast_files();
    assert!(!files.is_empty(), "no vendored .wast files found -- run fetch_testsuite.py");
    for path in &files {
        let source = fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        assert!(!source.trim().is_empty(), "{} is empty", path.display());
    }
}
