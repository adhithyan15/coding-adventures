//! Golden tests for the worked adjudication examples (ADJ constraints track D).
//!
//! Each `.adj` file in `code/specs/data/adj-language-expansion/examples/`
//! exercises a slice of the whole stack — typed values, dimensions, `let`
//! arithmetic, predicate verdicts, constraint solving — end-to-end through the
//! CPU reasoner at **zero model calls**. These tests run the built CLI on each
//! file and assert the documented outcome, so the examples stay runnable as the
//! language evolves.

use std::path::PathBuf;
use std::process::Command;

/// The examples directory, relative to this crate's manifest:
/// `code/packages/rust/adj-lang-cli` → `code/specs/data/adj-language-expansion/examples`.
fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-language-expansion/examples")
        .join(name)
}

/// Run the CLI on an example file; return (success, stdout).
fn run_example(name: &str) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(example(name))
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn eligibility_fires_the_filing_threshold() {
    let (ok, s) = run_example("eligibility.adj");
    assert!(ok, "non-zero exit: {s}");
    assert!(s.contains("\"kind\":\"predicate\""), "{s}");
    assert!(s.contains("\"slot\":\"gross_income\""), "{s}");
    assert!(s.contains("\"threshold\":14600"), "{s}");
    assert!(s.contains("\"observed\":18000"), "{s}");
    assert!(s.contains("\"type\":\"determinate\""), "{s}");
    assert!(
        s.contains("\"leader\":\"required_to_file\""),
        "{s}"
    );
}

#[test]
fn debt_to_income_ratio_drives_eligibility() {
    let (ok, s) = run_example("debt_to_income.adj");
    assert!(ok, "non-zero exit: {s}");
    // dti = 1800/6000 = 0.3 (dimensionless), <= 0.43 fires.
    assert!(s.contains("\"slot\":\"dti\""), "{s}");
    assert!(s.contains("\"observed\":0.3"), "{s}");
    assert!(s.contains("\"leader\":\"mortgage_eligible\""), "{s}");
}

#[test]
fn proration_computes_and_clears_the_floor() {
    let (ok, s) = run_example("proration.adj");
    assert!(ok, "non-zero exit: {s}");
    // prorated = 12000 * 9 / 12 = 9000, >= 8000 fires.
    assert!(s.contains("\"slot\":\"prorated\""), "{s}");
    assert!(s.contains("\"observed\":9000"), "{s}");
    assert!(s.contains("\"leader\":\"senior_tier\""), "{s}");
}

#[test]
fn break_even_solves_for_the_unit_price() {
    let (ok, s) = run_example("break_even.adj");
    assert!(ok, "non-zero exit: {s}");
    assert!(s.contains("\"outcome\":\"solved\""), "{s}");
    assert!(s.contains("\"name\":\"p\",\"value\":8"), "{s}");
    assert!(s.contains("\"from_constraints\":[0]"), "{s}");
}

#[test]
fn grant_allocation_maximizes_impact_via_lp() {
    // The allocation LP: max 3·outreach + 2·training s.t. budget ≤ 10,
    // outreach ≤ 6, both ≥ 0 → optimum 26 at (6, 4), binding the budget and
    // the outreach cap. The engine solves it end-to-end at 0 model calls.
    let (ok, s) = run_example("grant_allocation.adj");
    assert!(ok, "non-zero exit: {s}");
    assert!(s.contains("\"optimize\":{"), "expected an optimize section: {s}");
    assert!(s.contains("\"outcome\":\"optimal\""), "{s}");
    assert!(s.contains("\"value\":26"), "expected optimum 26: {s}");
    assert!(s.contains("\"name\":\"outreach\",\"value\":6"), "{s}");
    assert!(s.contains("\"name\":\"training\",\"value\":4"), "{s}");
    assert!(s.contains("\"binding\":[0,1]"), "budget + cap bind: {s}");
}
