//! End-to-end golden test for the GROUNDED conflict-resolution META-RULES (ADJ73 PR-B-4) through
//! the built CLI binary, on the committed artifacts.
//!
//! PR-B-3 asserted lex-superior edges as facts. PR-B-4 makes precedence DERIVED: a grounded
//! meta-rule `rule { head: outranks_context($H, $L) when: reverses($H, $L) }` (citing the
//! overruling doctrine) turns a primitive grounded `reverses` fact into a precedence edge, which
//! the engine (logic-engine >= 0.21) reads as a context-order edge. These tests prove the derived
//! edge drives `lex superior` end-to-end on the two worked examples, at 0 answer-time model calls:
//!
//!   * worked-appeal-example.adj — a Supreme Court reversal flips a (now-reversed) Ninth Circuit
//!     reading that sits at the HIGHEST tier; the SCOTUS reading governs via the DERIVED edge.
//!   * worked-supersession-example.adj — lex posterior: the 2024 guideline edition supersedes the
//!     2004 one, so the current recommendation governs the legacy one.

use std::path::PathBuf;
use std::process::Command;

/// The context-precedence data dir, relative to this crate's manifest.
fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/context-precedence")
        .join(name)
}

fn run(name: &str) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(data(name))
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn appeal_metarule_derives_the_edge_that_governs() {
    let (ok, s) = run("worked-appeal-example.adj");
    assert!(ok, "cli should succeed: {s}");
    assert!(s.contains("\"governing\""), "has a governing section: {s}");

    // The Supreme Court (narrow) reading governs, decided in the scotus_2023 context — purely
    // because the appeal-status meta-rule DERIVED outranks_context(scotus_2023, ninth_circuit_2019)
    // from the grounded `reverses` fact (no edge was asserted).
    assert!(
        s.contains("means(scope, narrow)"),
        "carries the SCOTUS reading: {s}"
    );
    assert!(
        s.contains("\"status\":\"governing\""),
        "the SCOTUS reading governs: {s}"
    );
    assert!(
        s.contains("\"context\":\"scotus_2023\""),
        "governed in the scotus_2023 context: {s}"
    );
    // The reversed Ninth Circuit reading is defeated despite its `mandatory` tier.
    assert!(
        s.contains("\"status\":\"defeated\""),
        "the reversed reading is defeated: {s}"
    );
    assert!(
        s.contains("\"context\":\"ninth_circuit_2019\""),
        "the defeated reading is the reversed Ninth Circuit one: {s}"
    );
    assert!(
        s.contains("\"defeated_by\":\"means(scope, narrow)\""),
        "defeated by the SCOTUS reading: {s}"
    );
    assert!(
        s.contains("\"standing\":\"mandatory\""),
        "the defeated reading carried the highest tier, yet lost on derived context: {s}"
    );
    assert!(
        s.contains("\"has_conflict\":false"),
        "clean override, not a split: {s}"
    );
}

#[test]
fn lex_posterior_metarule_picks_the_newer_edition() {
    let (ok, s) = run("worked-supersession-example.adj");
    assert!(ok, "cli should succeed: {s}");
    // The 2024 (current) recommendation governs the 2004 (legacy) one via the derived edge.
    assert!(
        s.contains("recommendation(empiric_regimen, current)"),
        "carries the current recommendation: {s}"
    );
    assert!(
        s.contains("\"status\":\"governing\""),
        "current recommendation governs: {s}"
    );
    assert!(
        s.contains("\"context\":\"idsa_2024\""),
        "governed in the idsa_2024 context: {s}"
    );
    assert!(
        s.contains("\"defeated_by\":\"recommendation(empiric_regimen, current)\""),
        "the legacy recommendation is defeated by the current one: {s}"
    );
    assert!(
        s.contains("\"context\":\"idsa_2004\""),
        "the defeated one is the 2004 edition: {s}"
    );
    assert!(
        s.contains("\"has_conflict\":false"),
        "clean supersession, not a split: {s}"
    );
}
