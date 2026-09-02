//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/karst-process-water-table-position.adj`)
//! driven through the built CLI: a `rule` that DERIVES where a karst
//! process happens relative to the water table by chaining two shipped
//! tables, grounding the U.S. National Park Service's "Speleothems"
//! article.
//!
//! No table states this relation. The rule body chains
//! `karst_process_zone` (process -> zone) with
//! `zone_water_table_position` (zone -> position), so the conclusion's
//! provenance is the composition of its premises'.
//!
//! THE LAST TWO TESTS ARE A MATCHED PAIR, AND THE SECOND IS THE POINT.
//! The identical chain that resolves for speleothem deposition must find
//! NOTHING for cave formation, because `karst_process_zone` binds the
//! hedged atom `zone_of_saturation_typically` -- the source says caves only
//! TYPICALLY form below the water table -- which does not unify with the
//! bare `zone_of_saturation` the position table keys on.
//!
//! That is the system working. Had the hedge been dropped when the atom was
//! authored, this rule would happily conclude "cave formation happens below
//! the water table" as a flat fact, which is exactly the claim the source
//! declines to make. A qualifier that survives direct recall but evaporates
//! the moment something reasons across two tables is not a qualifier at
//! all. If the abstention below ever turns into a binding, a tendency has
//! been promoted to a rule.
//!
//! 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factskpwtp_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

/// Place the rule library and both premise tables it imports.
fn place(dir: &Path) {
    for name in [
        "karst-process-water-table-position.adj",
        "karst-process-zone.adj",
        "zone-water-table-position.adj",
    ] {
        let src = facts_stdlib().join("earth-science").join(name);
        std::fs::copy(&src, dir.join(name))
            .unwrap_or_else(|e| panic!("copy shipped {name}: {e}"));
    }
}

/// Write a one-query program against the rule library.
fn case(dir: &Path, query: &str) -> PathBuf {
    let path = dir.join("case.adj");
    std::fs::write(
        &path,
        format!("import \"karst-process-water-table-position.adj\"\n? {query}\n"),
    )
    .unwrap();
    path
}

#[test]
fn the_rule_derives_that_speleothems_form_above_the_water_table() {
    let dir = scratch("derive");
    place(&dir);
    let program = case(
        &dir,
        "karst_process_water_table_position(speleothem_deposition, $P)",
    );

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Neither premise table answers this question alone.
    assert!(
        out.contains("\"P\":\"above_the_water_table\""),
        "the chain derives above_the_water_table: {out}"
    );
    assert!(
        out.contains("nps.gov/subjects/caves/speleothems.htm")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS citation: {out}"
    );
}

#[test]
fn the_derived_answer_shows_its_work() {
    let dir = scratch("proof");
    place(&dir);
    let program = case(
        &dir,
        "karst_process_water_table_position(speleothem_deposition, $P)",
    );

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // The proof trace is what makes a DERIVED answer auditable: a rule step
    // for the head, then one fact step per body goal naming the relation it
    // discharged. Without this, a composed answer is indistinguishable from
    // an asserted one.
    assert!(
        out.contains("\"kind\":\"rule\""),
        "the answer is derived, not stored: {out}"
    );
    assert!(
        out.contains("karst_process_zone(Process, Zone)")
            && out.contains("zone_water_table_position(Zone, Position)"),
        "both premise goals appear in the proof: {out}"
    );
}

#[test]
fn the_rule_runs_backward_from_the_position() {
    let dir = scratch("reverse");
    place(&dir);
    let program = case(
        &dir,
        "karst_process_water_table_position($P, above_the_water_table)",
    );

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"P\":\"speleothem_deposition\""),
        "the above-the-water-table process is speleothem deposition: {out}"
    );
}

#[test]
fn the_rule_abstains_on_processes_no_premise_places() {
    let dir = scratch("unplaced");
    place(&dir);
    let program = case(&dir, "karst_process_water_table_position(dissolution, $P)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // A derived relation cannot be better grounded than its premises.
    // `karst_process_zone` never places dissolution, so there is no zone to
    // chain through and the rule declines rather than reaching for general
    // karst knowledge.
    assert!(
        out.contains("\"abstained\":true"),
        "an unplaced process has no chain to follow: {out}"
    );
}

#[test]
fn the_typicality_hedge_blocks_the_cave_formation_join() {
    let dir = scratch("hedgejoin");
    place(&dir);
    let program = case(&dir, "karst_process_water_table_position(cave_formation, $P)");

    let (ok, out) = run(&program);
    assert!(ok, "cli should succeed: {out}");
    // THE POINT. The identical chain that resolved for deposition must find
    // nothing here. `karst_process_zone` binds the hedged atom
    // `zone_of_saturation_typically` -- caves only TYPICALLY form below the
    // water table -- which does not unify with the bare `zone_of_saturation`
    // the position table keys on. The hedge survives the join instead of
    // being discarded by it.
    assert!(
        out.contains("\"abstained\":true"),
        "the hedged atom must not join through to a flat position: {out}"
    );
    // Belt and braces: the unlicensed conclusion must not appear by any
    // route. The needle carries its closing quote, so a longer atom sharing
    // the prefix cannot satisfy it by accident -- and it is the BINDING
    // form, so the phrase "below the water table" appearing inside the
    // quoted source sentence cannot satisfy it either.
    assert!(
        !out.contains("\"P\":\"below_the_water_table\""),
        "must never conclude cave formation happens below the water table: {out}"
    );
}
