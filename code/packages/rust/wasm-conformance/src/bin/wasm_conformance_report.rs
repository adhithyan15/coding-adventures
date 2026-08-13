//! # wasm_conformance_report
//!
//! The actual day-to-day deliverable of this crate: walk every vendored
//! `.wast` file, run it, and print a per-file table plus an aggregate line
//! per directive kind — "where does `wasm-execution` actually stand,"
//! answerable on demand.
//!
//! ```text
//! cargo run --bin wasm_conformance_report -p wasm-conformance
//! cargo run --bin wasm_conformance_report -p wasm-conformance -- --write-baseline
//! ```
//!
//! `--write-baseline` regenerates `tests/fixtures/testsuite-status.json`,
//! the golden manifest `tests/testsuite_conformance.rs` diffs every
//! `cargo test` run against. Only run it after a deliberate, reviewed
//! change — see that test's own doc comment.

use std::fs;
use std::path::{Path, PathBuf};
use wasm_conformance::report::{ConformanceReport, DirectiveKind, Tally};

fn testsuite_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/testsuite")
}

fn baseline_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/testsuite-status.json")
}

/// Every `.wast` file in the vendored corpus, sorted for deterministic
/// output — `fs::read_dir` does not guarantee an order.
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

fn run_report() -> ConformanceReport {
    let mut report = ConformanceReport::default();
    for path in discover_wast_files() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {name}: {e}"));
        match wasm_conformance::run_wast_source(&source) {
            Ok(results) => report.add_file(name, wasm_conformance::report::tally_results(&results)),
            Err(e) => {
                eprintln!("{name}: FAILED TO PARSE SCRIPT: {e}");
                report.add_parse_failure(name, e.to_string());
            }
        }
    }
    report
}

fn print_tally_row(label: &str, tally: &Tally) {
    let total = tally.total();
    if total == 0 {
        return;
    }
    let graded = tally.graded();
    let pct = if graded == 0 { 0.0 } else { 100.0 * tally.pass as f64 / graded as f64 };
    if tally.not_yet_supported > 0 && graded == 0 {
        println!("    {label:<20} {}/{} (not yet supported)", tally.pass, total);
    } else {
        println!(
            "    {label:<20} {}/{} ({pct:.1}%){}",
            tally.pass,
            graded,
            if tally.not_yet_supported > 0 { format!(", {} not yet supported", tally.not_yet_supported) } else { String::new() }
        );
    }
}

fn print_report(report: &ConformanceReport) {
    if !report.parse_failures.is_empty() {
        println!("== files that failed to parse ({}) ==", report.parse_failures.len());
        for (file, error) in &report.parse_failures {
            println!("    {file}: {error}");
        }
        println!();
    }
    for (file, tallies) in &report.files {
        println!("{file}");
        for kind in DirectiveKind::ALL {
            if let Some(tally) = tallies.get(kind.label()) {
                print_tally_row(kind.label(), tally);
            }
        }
    }
    println!("\n== aggregate ==");
    println!(
        "    {} files parsed, {} failed to parse",
        report.files.len(),
        report.parse_failures.len()
    );
    for kind in DirectiveKind::ALL {
        if let Some(tally) = report.aggregate.get(kind.label()) {
            print_tally_row(kind.label(), tally);
        }
    }
}

fn main() {
    let report = run_report();
    print_report(&report);

    if std::env::args().any(|a| a == "--write-baseline") {
        let json = serde_json::to_string_pretty(&report).expect("report should serialize");
        fs::write(baseline_path(), json + "\n").expect("failed to write baseline");
        println!("\nwrote {}", baseline_path().display());
    }
}
