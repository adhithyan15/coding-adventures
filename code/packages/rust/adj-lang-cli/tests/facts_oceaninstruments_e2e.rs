//! End-to-end test for the oceanography FACTS library
//! (`adj-facts-stdlib/oceanography/ocean-observing-instruments.adj`) driven
//! through the built CLI: a native `table` naming which quantity each of
//! three ocean-observing instruments measures, quoted verbatim from NOAA's
//! oceanservice.noaa.gov "facts" page series -- a THIRD "observation and
//! measurement" axis after `chemistry/measuring-tools.adj` (lab tools) and
//! `meteorology/weather-instruments.adj` (weather instruments). 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_oceaninstruments_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("oceanography/ocean-observing-instruments.adj");
    std::fs::copy(&src, dir.join("ocean-observing-instruments.adj"))
        .expect("copy shipped ocean-observing-instruments.adj");
}

#[test]
fn ocean_instrument_recall_binds_the_quantity_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-observing-instruments.adj\"\n\
         ? ocean_instrument(tide_gauge, $Q)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Q\":\"sea_level\""),
        "a tide gauge measures sea level: {out}"
    );
    assert!(
        out.contains("oceanservice.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA citation: {out}"
    );
}

#[test]
fn ocean_instrument_reverse_binds_the_instrument_for_that_quantity() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-observing-instruments.adj\"\n\
         ? ocean_instrument($I, underwater_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"I\":\"hydrophone\""),
        "a hydrophone measures underwater sound: {out}"
    );
}

#[test]
fn ocean_instrument_abstains_honestly_on_a_multi_quantity_instrument() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-observing-instruments.adj\"\n\
         ? ocean_instrument(ctd, $Q)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a CTD measures multiple quantities at once, deliberately not a row -- honest abstention, never invented: {out}"
    );
}
