//! End-to-end tests for the `--explain` renderer's ARGUMENT surface
//! (ADJ-ARGUMENT-IR ADR-6 / ADJ-REASON-MATH §E.8). This is the "explain" half of
//! "reason AND explain" for the `argument` construct: an `argument` desugars to
//! facts + rules (ADR-2), the engine derives its thesis by SLD over those, and
//! this surface projects that derivation back into prose a person reads — the
//! argument's grounded **premises** → the **connective** (inference rule) that
//! licensed each step → the derived **conclusion**.
//!
//! Where PR-E1/E2 rendered a differential's arithmetic and probabilistic
//! reasoning, a binding query (`? failed_by(axle, $M)`) is resolved by SLD
//! enumeration, not the differential — so its proof never flowed through the
//! `--explain` inference surface. These tests pin the new surface end to end on
//! the committed `axle-fatigue` worked example: the chain is rendered with the
//! DERIVED conclusion (not the still-open query variable), every line carries its
//! own provenance, and the render is deterministic (P4).

use std::path::{Path, PathBuf};
use std::process::Command;

/// The committed worked-example directory (shared with `argument_worked_example_e2e`).
fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../specs/data/adj-argument-ir")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rs4e3_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, src: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, src).unwrap();
    p
}

fn explain(program: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg("--explain")
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    assert!(
        out.status.success(),
        "--explain exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

// ---------------------------------------------------------------------------
// (1) The worked example's argument renders as premises → connective →
//     conclusion, with the DERIVED thesis and per-line provenance.
// ---------------------------------------------------------------------------

#[test]
fn renders_the_argument_chain_premises_connective_conclusion() {
    let adj = data_dir().join("axle-fatigue.adj");
    let s = explain(&adj);

    // The section header names the query the argument answers.
    assert!(
        s.contains("Argument for failed_by(axle, Mechanism):"),
        "argument section header: {s:?}"
    );
    // The CONCLUSION is the DERIVED thesis — `fatigue` bound from the query
    // variable — reached via an inference connective (not asserted, not the
    // still-open `$Mechanism`).
    assert!(
        s.contains("failed_by(axle, fatigue)  <= inference"),
        "the derived conclusion is rendered as an inference step: {s:?}"
    );
    // The intermediate sub-conclusion is itself an inference step (the two-step
    // chain: exceeds_endurance feeds failed_by).
    assert!(
        s.contains("exceeds_endurance(axle)  <= inference"),
        "the intermediate conclusion is an inference step: {s:?}"
    );
    // The grounded PREMISES are rendered as premises — the argument's leaves.
    assert!(
        s.contains("premise stress_amplitude(axle, 420)")
            && s.contains("premise endurance_limit(axle, 380)")
            && s.contains("premise shows(surface, beach_marks)")
            && s.contains("premise diagnostic_of(beach_marks, fatigue)"),
        "all four grounded premises are rendered: {s:?}"
    );
    // P2 — every line carries its own provenance: the premises and the
    // connectives all cite the source document at authoritative trust.
    assert!(
        s.contains("source \"axle failure report\" trust authoritative"),
        "lines carry their provenance: {s:?}"
    );
}

// ---------------------------------------------------------------------------
// (2) P4 — the argument explanation is deterministic across runs.
// ---------------------------------------------------------------------------

#[test]
fn the_argument_explanation_is_deterministic() {
    let adj = data_dir().join("axle-fatigue.adj");
    let a = explain(&adj);
    let b = explain(&adj);
    assert_eq!(a, b, "the same argument must render identical explanation text");
    assert!(
        a.contains("failed_by(axle, fatigue)  <= inference"),
        "non-trivial explanation: {a:?}"
    );
}

// ---------------------------------------------------------------------------
// (3) An unprovable binding query renders honest abstention — never a
//     fabricated chain (the argument surface's form of "I cannot commit").
// ---------------------------------------------------------------------------

#[test]
fn an_unprovable_query_renders_honest_abstention() {
    let dir = scratch("abstain");
    // A relate fact grounds one edge; the query asks for a DIFFERENT subject that
    // no fact or rule derives — so the search runs to completion with no proof.
    let prog = write(
        &dir,
        "case.adj",
        "relate causes(hepatitis_b, cirrhosis)\n\
         source \"ref\" trust authoritative\n\
         ? causes(measles, $Outcome)\n",
    );
    let s = explain(&prog);
    assert!(
        s.contains("Argument for causes(measles, Outcome):"),
        "the query still gets a section: {s:?}"
    );
    assert!(
        s.contains("abstained: no grounded chain derives this"),
        "an unprovable query abstains honestly, not a fabricated chain: {s:?}"
    );
}

// ---------------------------------------------------------------------------
// (4) A one-premise recall query is a degenerate argument: the grounding fact
//     IS the premise, rendered with its citation.
// ---------------------------------------------------------------------------

#[test]
fn a_recall_query_renders_its_grounding_fact_as_a_premise() {
    let dir = scratch("recall");
    let prog = write(
        &dir,
        "case.adj",
        "relate causes(hepatitis_b, cirrhosis)\n\
         source \"hepatology ref\" trust authoritative\n\
         ? causes(hepatitis_b, $Outcome)\n",
    );
    let s = explain(&prog);
    assert!(
        s.contains("premise causes(hepatitis_b, cirrhosis)")
            && s.contains("source \"hepatology ref\" trust authoritative"),
        "the grounding fact is rendered as a cited premise: {s:?}"
    );
}
