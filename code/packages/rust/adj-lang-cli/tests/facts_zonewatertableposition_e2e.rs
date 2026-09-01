//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/zone-water-table-position.adj`) driven
//! through the built CLI: a native `table` recording where each groundwater
//! zone sits relative to the water table, grounding the U.S. National Park
//! Service's "Speleothems" article.
//!
//! THIRD cave/karst library, and the first designed to COMPOSE with a
//! sibling. `karst-process-zone.adj` places a process in a zone; this table
//! places a zone relative to the water table; chained, they answer a
//! question neither answers alone.
//!
//! That chain is NOT exercised here. A query in this language is a single
//! term, so composition lives in a rule body -- see the sibling rule
//! library and `facts_karstprocesswatertableposition_e2e.rs`, which is
//! where the interesting property (the source's typicality hedge blocking
//! the cave-formation join) is asserted. This file covers the table on its
//! own: direct recall with its citation, backward recall, and abstention.
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
    let dir = std::env::temp_dir().join(format!("adjcli_factszonewtp_{tag}_{}", std::process::id()));
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

/// Place this table alone.
fn place(dir: &Path) {
    let src = facts_stdlib().join("earth-science/zone-water-table-position.adj");
    std::fs::copy(&src, dir.join("zone-water-table-position.adj"))
        .expect("copy shipped zone-water-table-position.adj");
}

#[test]
fn zone_water_table_position_places_the_zone_of_aeration_above_the_water_table() {
    let dir = scratch("aeration");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"zone-water-table-position.adj\"\n\
         ? zone_water_table_position(zone_of_aeration, $P)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"P\":\"above_the_water_table\""),
        "the zone of aeration is above the water table: {out}"
    );
    assert!(
        out.contains("until caves are above the water table in the zone of aeration"),
        "carries the grounding sentence verbatim: {out}"
    );
    assert!(
        out.contains("nps.gov/subjects/caves/speleothems.htm")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the NPS citation: {out}"
    );
}

#[test]
fn zone_water_table_position_runs_backward_from_the_position() {
    let dir = scratch("reverse");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"zone-water-table-position.adj\"\n\
         ? zone_water_table_position($Z, below_the_water_table)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("zone_water_table_position(zone_of_saturation, below_the_water_table)"),
        "the below-the-water-table zone is the zone of saturation: {out}"
    );
}

#[test]
fn zone_water_table_position_abstains_on_zones_this_source_never_names() {
    let dir = scratch("unnamed");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"zone-water-table-position.adj\"\n\
         ? zone_water_table_position(vadose_zone, $P)\n\
         ? zone_water_table_position(phreatic_zone, $P)\n\
         ? zone_water_table_position(capillary_fringe, $P)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The vadose and phreatic zones are the standard synonyms for these two
    // and a reader may well ask by those names; the capillary fringe is a
    // real subdivision a hydrology text would add. This source uses none of
    // the three words, so the table declines rather than answering from
    // general karst vocabulary.
    let abstained = out.matches("\"abstained\":true").count();
    assert_eq!(
        abstained, 3,
        "synonyms and unnamed subdivisions abstain rather than being inferred: {out}"
    );
}
