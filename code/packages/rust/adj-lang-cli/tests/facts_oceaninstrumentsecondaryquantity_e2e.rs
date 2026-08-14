//! End-to-end test for the oceanography FACTS library
//! (`adj-facts-stdlib/oceanography/ocean-instrument-secondary-quantity.adj`)
//! driven through the built CLI: a native `table` naming a SECOND quantity
//! sonar determines (orientation of the object), decoded from a span already
//! sitting unused inside the SAME NOAA "Sonar" quote
//! `ocean-observing-instruments.adj` already cites -- a sibling to that
//! table. Resolves binding-query recall (both directions) with the source's
//! citation, and abstains on an instrument (tide_gauge) whose cited span
//! names no second quantity -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_oceaninstrumentsecondaryquantity_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("oceanography/ocean-instrument-secondary-quantity.adj");
    std::fs::copy(&src, dir.join("ocean-instrument-secondary-quantity.adj"))
        .expect("copy shipped ocean-instrument-secondary-quantity.adj");
}

#[test]
fn ocean_instrument_secondary_quantity_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-instrument-secondary-quantity.adj\"\n\
         ? ocean_instrument_secondary_quantity(sonar, $SecondaryQuantity)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"ocean_instrument_secondary_quantity(sonar, orientation_of_object)\""),
        "sonar also determines orientation: {out}"
    );
    assert!(
        out.contains("oceanservice.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA citation: {out}"
    );
}

#[test]
fn ocean_instrument_secondary_quantity_recalls_backward_from_a_bound_quantity() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-instrument-secondary-quantity.adj\"\n\
         ? ocean_instrument_secondary_quantity($Instrument, orientation_of_object)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"ocean_instrument_secondary_quantity(sonar, orientation_of_object)\""),
        "orientation is determined by sonar: {out}"
    );
}

#[test]
fn ocean_instrument_secondary_quantity_abstains_honestly_on_the_tide_gauge() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-instrument-secondary-quantity.adj\"\n\
         ? ocean_instrument_secondary_quantity(tide_gauge, $SecondaryQuantity)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the cited tide-gauge span names no second quantity -- honest abstention: {out}"
    );
}
