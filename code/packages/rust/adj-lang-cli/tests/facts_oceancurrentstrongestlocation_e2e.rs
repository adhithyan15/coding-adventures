//! End-to-end test for the oceanography FACTS library
//! (`adj-facts-stdlib/oceanography/ocean-current-strongest-location.adj`)
//! driven through the built CLI: a native `table` naming WHERE tidal
//! currents are strongest (near the shore and in bays and estuaries along
//! the coast), decoded from a span already sitting unused inside the SAME
//! NOAA "What is a current?" quote `ocean-current-drivers.adj` already
//! cites -- a sibling to that table. Resolves binding-query recall (both
//! directions) with the source's citation, and abstains on a current type
//! (thermohaline_circulation) whose cited span names no location -- 0
//! model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_oceancurrentstrongestlocation_{tag}_{}", std::process::id()));
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

fn place_lib(dir: &Path) {
    let src = facts_stdlib().join("oceanography/ocean-current-strongest-location.adj");
    std::fs::copy(&src, dir.join("ocean-current-strongest-location.adj"))
        .expect("copy shipped ocean-current-strongest-location.adj");
}

#[test]
fn ocean_current_strongest_location_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-current-strongest-location.adj\"\n\
         ? ocean_current_strongest_location(tidal_currents, $Location)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains(
            "\"term\":\"ocean_current_strongest_location(tidal_currents, near_the_shore_and_in_bays_and_estuaries_along_the_coast)\""
        ),
        "tidal currents are strongest near the shore and in bays and estuaries: {out}"
    );
    assert!(
        out.contains("oceanservice.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA citation: {out}"
    );
}

#[test]
fn ocean_current_strongest_location_recalls_backward_from_a_bound_location() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-current-strongest-location.adj\"\n\
         ? ocean_current_strongest_location($CurrentType, near_the_shore_and_in_bays_and_estuaries_along_the_coast)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains(
            "\"term\":\"ocean_current_strongest_location(tidal_currents, near_the_shore_and_in_bays_and_estuaries_along_the_coast)\""
        ),
        "the location names tidal currents: {out}"
    );
}

#[test]
fn ocean_current_strongest_location_abstains_honestly_on_thermohaline_circulation() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-current-strongest-location.adj\"\n\
         ? ocean_current_strongest_location(thermohaline_circulation, $Location)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the cited thermohaline-circulation span names no location -- honest abstention: {out}"
    );
}
