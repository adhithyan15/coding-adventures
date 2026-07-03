//! End-to-end golden test for the GROUNDED context-precedence rulebook + worked legal example
//! (ADJ73 PR-B-3) through the built CLI binary.
//!
//! This proves the whole *lex superior* chain on real committed artifacts, at 0 answer-time
//! model calls:
//!
//!   1. `context-precedence.adj` declares the precedence order as GROUNDED facts
//!      (`relate outranks_context(ninth_circuit, district_court) source "<verbatim stare-decisis
//!      quote>" trust authoritative`) — each edge carrying its charter.
//!   2. `worked-legal-example.adj` `import`s that rulebook and sets the trap: two courts read a
//!      statute term differently; the district court's (narrow) reading is asserted at the HIGHEST
//!      (`mandatory`) tier, the circuit's (broad) reading at the lowest (`default`).
//!   3. The engine reads the grounded edge as a context-order edge (logic-engine 0.20) and applies
//!      context precedence BEFORE the tier: the circuit's broad reading GOVERNS, the district's
//!      narrow reading is DEFEATED — despite its higher tier.
//!
//! It also proves the precedence is AUDITABLE: a binding query over `outranks_context` recalls the
//! governing edge WITH its byte-quoted charter.

use std::path::PathBuf;
use std::process::Command;

/// The context-precedence data dir, relative to this crate's manifest:
/// `code/packages/rust/adj-lang-cli` → `code/specs/data/context-precedence`.
fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/context-precedence")
        .join(name)
}

/// Run the CLI on a committed `.adj` file; return (success, stdout).
fn run(name: &str) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(data(name))
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn higher_context_reading_governs_despite_lower_tier() {
    let (ok, s) = run("worked-legal-example.adj");
    assert!(ok, "cli should succeed: {s}");

    // The governing section exists (the query is a binding query).
    assert!(s.contains("\"governing\""), "has a governing section: {s}");

    // The Ninth Circuit's BROAD reading governs — and is decided in the ninth_circuit context.
    assert!(
        s.contains("means(navigable_waters, broad)"),
        "carries the broad reading: {s}"
    );
    assert!(
        s.contains("\"status\":\"governing\""),
        "the broad reading governs: {s}"
    );
    assert!(
        s.contains("\"context\":\"ninth_circuit\""),
        "the governing answer is decided in the ninth_circuit context: {s}"
    );

    // The district court's NARROW reading is defeated by the broad reading — even though it sits
    // at the higher `mandatory` tier (context is primary; lex superior beats the tier).
    assert!(
        s.contains("\"status\":\"defeated\""),
        "the narrow reading is defeated: {s}"
    );
    assert!(
        s.contains("\"context\":\"district_court\""),
        "the defeated answer is the district_court reading: {s}"
    );
    assert!(
        s.contains("\"defeated_by\":\"means(navigable_waters, broad)\""),
        "defeated by the broad reading: {s}"
    );
    assert!(
        s.contains("\"standing\":\"mandatory\""),
        "the defeated reading carried the highest tier, yet still lost on context: {s}"
    );

    // It is a clean override, not an unresolved split.
    assert!(s.contains("\"has_conflict\":false"), "no conflict: {s}");
}

#[test]
fn the_precedence_edge_is_auditable_with_its_charter() {
    // The precedence ORDER is itself queryable + cited: recall the grounded edge and confirm its
    // verbatim charter rides on it (the whole point of "context precedence is grounded").
    let dir = std::env::temp_dir().join(format!("adjcli_ctxaudit_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Copy the committed rulebook + its shared resolve module beside a tiny query case so the
    // import chain resolves locally. The grounded charter now rides on the canon-TAGGED edge
    // fact `outranks_context_by(…, lex_superior)` (the bare `outranks_context/2` is the resolved
    // order, derived); audit by recalling the tagged edge.
    for f in ["context-precedence.adj", "context-precedence-resolve.adj"] {
        std::fs::write(dir.join(f), std::fs::read_to_string(data(f)).unwrap()).unwrap();
    }
    std::fs::write(
        dir.join("audit.adj"),
        "import \"context-precedence.adj\"\n? outranks_context_by(ninth_circuit, district_court, $canon)\n",
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(dir.join("audit.adj"))
        .output()
        .expect("run adj-lang-cli");
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success(), "cli should succeed: {s}");

    // The tagged edge is recalled, bound to district_court + the lex_superior canon...
    assert!(
        s.contains("outranks_context_by(ninth_circuit, district_court, lex_superior)"),
        "recalls the grounded canon-tagged precedence edge: {s}"
    );
    // ...and its charter (the verbatim stare-decisis quote) is on the citation.
    assert!(
        s.contains("binding on all federal district courts within its circuit"),
        "the edge carries its byte-quoted charter for the audit trail: {s}"
    );
}
