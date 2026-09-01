//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/karst-process-zone.adj`) driven through
//! the built CLI: a native `table` recording which groundwater zone a named
//! karst process happens in, grounding the U.S. National Park Service's
//! "Speleothems" article.
//!
//! Sibling to `speleothem-growth-surface.adj`, on a different axis: that
//! one answers WHERE ON THE CAVE a speleothem grows, this one answers WHERE
//! RELATIVE TO THE WATER TABLE the process can happen at all. Both rows
//! come from a single source sentence.
//!
//! The assertion that matters most is the LAST one. The source hedges one
//! clause and not the other -- caves "typically" form below the water
//! table, whereas speleothem deposition "is not possible until" caves are
//! above it. The hedge therefore rides inside the atom
//! (`zone_of_saturation_typically`), so a query for the bare, unhedged
//! `zone_of_saturation` must find NOTHING. If that ever starts binding, a
//! tendency has been silently promoted to a rule.
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
    let dir = std::env::temp_dir().join(format!(
        "adjcli_factskarstzone_{tag}_{}",
        std::process::id()
    ));
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

fn place(dir: &Path) {
    let src = facts_stdlib().join("earth-science/karst-process-zone.adj");
    std::fs::copy(&src, dir.join("karst-process-zone.adj"))
        .expect("copy shipped karst-process-zone.adj");
}

#[test]
fn karst_process_zone_places_speleothem_deposition_above_the_water_table() {
    let dir = scratch("deposition");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"karst-process-zone.adj\"\n\
         ? karst_process_zone(speleothem_deposition, $Z)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Z\":\"zone_of_aeration\""),
        "deposition happens in the zone of aeration: {out}"
    );
    assert!(
        out.contains("the deposition of speleothems is not possible until caves are above the water table"),
        "carries the grounding sentence verbatim: {out}"
    );
    assert!(
        out.contains("nps.gov/subjects/caves/speleothems.htm")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS citation: {out}"
    );
}

#[test]
fn karst_process_zone_keeps_the_typicality_hedge_inside_the_atom() {
    let dir = scratch("hedge");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"karst-process-zone.adj\"\n\
         ? karst_process_zone(cave_formation, $Z)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The source says caves "typically" form below the water table. The
    // qualifier modifies this one value, so it lives in the atom -- the
    // same placement rule veto-override.adj applies to "in most cases".
    assert!(
        out.contains("\"Z\":\"zone_of_saturation_typically\""),
        "the tendency is carried in the atom: {out}"
    );
    assert!(
        !out.contains("\"Z\":\"zone_of_saturation\""),
        "must not drop the hedge and state a bare zone: {out}"
    );
}

#[test]
fn karst_process_zone_runs_backward_from_the_zone() {
    let dir = scratch("reverse");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"karst-process-zone.adj\"\n\
         ? karst_process_zone($P, zone_of_aeration)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("karst_process_zone(speleothem_deposition, zone_of_aeration)"),
        "the aeration-zone process is speleothem deposition: {out}"
    );
}

#[test]
fn karst_process_zone_abstains_on_the_unhedged_saturation_zone() {
    let dir = scratch("unhedged");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"karst-process-zone.adj\"\n\
         ? karst_process_zone($P, zone_of_saturation)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // THE POINT OF THE HEDGE PLACEMENT. Asking for the bare, unhedged zone
    // is asking "which process ALWAYS happens below the water table?" -- a
    // question this source does not answer, because it says caves form
    // there TYPICALLY. Because the qualifier rides in the atom rather than
    // being dropped, the absolute query correctly finds nothing. If this
    // ever binds, a tendency has been silently promoted to a rule.
    assert!(
        out.contains("\"abstained\":true"),
        "the unhedged zone is not a value of this relation: {out}"
    );
    assert!(
        !out.contains("karst_process_zone(cave_formation, zone_of_saturation)"),
        "must never assert cave formation as unconditionally below the water table: {out}"
    );
}

#[test]
fn karst_process_zone_abstains_on_processes_the_source_does_not_place() {
    let dir = scratch("unplaced");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"karst-process-zone.adj\"\n\
         ? karst_process_zone(dissolution, $Z)\n\
         ? karst_process_zone(speleogenesis, $Z)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Both are real karst processes, and a model asked where they happen
    // will answer from general knowledge. This source does not place either
    // relative to the water table, so the table declines rather than
    // importing an answer from outside its citation.
    let abstained_count = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained_count, 2,
        "unplaced karst processes abstain rather than being inferred: {out}"
    );
}
