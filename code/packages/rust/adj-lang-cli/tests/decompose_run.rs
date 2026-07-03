//! Deterministic golden tests over the **committed** decompositions from the
//! live decompose->solve demonstration (ADJ constraints track D2).
//!
//! The live run (`code/specs/data/adj-constraints-decompose-run/run.py`) calls a
//! local model ONCE per case to turn messy prose into an adj-lang program, then
//! the engine solves it. The model is non-deterministic and Ollama is not in CI,
//! so the *model* half is not tested here. What IS tested — deterministically,
//! with no model — is that the **engine is a pure function of the committed
//! `.adj`**: re-running `adj-lang-cli` on each committed decomposition reproduces
//! the recorded outcome. This is the load-bearing claim: the model's
//! non-determinism is fully quarantined to the `.adj`; the answer is the engine's.
//!
//! Three of the four recorded decompositions solve to their gold value; the
//! fourth (`workshop_break_even`) is the honest-failure case — the model wrote an
//! inequality ("cover cost") where an equality ("break even") was needed, and the
//! engine reports `unsupported` rather than inventing a number. We assert that
//! too: the engine never fabricates an answer for a mis-decomposed problem.

use std::path::PathBuf;
use std::process::Command;

/// A committed decomposition, relative to this crate's manifest:
/// `code/packages/rust/adj-lang-cli` -> `code/specs/data/adj-constraints-decompose-run/results`.
fn committed(id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-constraints-decompose-run/results")
        .join(format!("{id}.adj"))
}

/// Run the CLI on a committed `.adj`; return (success, stdout).
fn solve(id: &str) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(committed(id))
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn relief_allocation_decomposition_solves_to_44() {
    // The model maximized 4·meals + 3·shelter under the budget + meals cap.
    let (ok, s) = solve("relief_allocation");
    assert!(ok, "non-zero exit: {s}");
    assert!(s.contains("\"optimize\":{"), "{s}");
    assert!(s.contains("\"outcome\":\"optimal\""), "{s}");
    assert!(s.contains("\"value\":44"), "expected optimum 44: {s}");
}

#[test]
fn production_cost_decomposition_solves_to_980() {
    // The model minimized 5·A + 8·B under the contract minimums.
    let (ok, s) = solve("production_cost");
    assert!(ok, "non-zero exit: {s}");
    assert!(s.contains("\"outcome\":\"optimal\""), "{s}");
    assert!(s.contains("\"value\":980"), "expected optimum 980: {s}");
}

#[test]
fn schedule_feasibility_decomposition_is_unsat() {
    // The model encoded the schedule constraints; the engine proves the
    // contradiction (design ≥ 28 but design ≤ 25 from the build deadline).
    let (ok, s) = solve("schedule_feasibility");
    assert!(ok, "non-zero exit: {s}");
    assert!(s.contains("\"check\":{"), "{s}");
    assert!(s.contains("\"outcome\":\"unsat\""), "expected unsat: {s}");
}

#[test]
fn break_even_misdecomposition_is_unsupported_not_fabricated() {
    // Honest-failure case: the model wrote an inequality where an equality was
    // needed. The engine REFUSES to guess — `unsupported`, never a fake value.
    let (ok, s) = solve("workshop_break_even");
    assert!(ok, "non-zero exit: {s}");
    assert!(s.contains("\"outcome\":\"unsupported\""), "{s}");
    // Crucially: no fabricated solved value.
    assert!(!s.contains("\"outcome\":\"solved\""), "must not fabricate: {s}");
}
