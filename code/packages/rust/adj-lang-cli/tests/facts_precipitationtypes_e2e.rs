//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/precipitation-types.adj`) driven through the
//! built CLI: a native `table` of the common precipitation types → the defining
//! physical form resolves binding-query recalls (forward AND backward) with the
//! source's NOAA National Weather Service citation, and abstains on a word that
//! is not one of these precipitation types (fog) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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

#[test]
fn meteorology_precipitation_types_recall_binds_form_with_citation() {
    let dir = scratch("precipitationtypes");
    // Copy the shipped meteorology table beside the entry program and import it.
    let src = facts_stdlib().join("meteorology/precipitation-types.adj");
    std::fs::copy(&src, dir.join("precipitation-types.adj"))
        .expect("copy shipped precipitation-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-types.adj\"\n\
         ? precip_form(snow, $Form)\n\
         ? precip_form(sleet, $Form)\n\
         ? precip_form(freezing_rain, $Form)\n\
         ? precip_form($Precip, balls_of_ice)\n\
         ? precip_form(fog, $Form)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Snow is ice crystals, sleet reaches the ground as frozen rain drops,
    // freezing rain glazes to ice on contact — the recalled forms (forward
    // binds).
    assert!(
        out.contains("\"Form\":\"ice_crystals\""),
        "snow → ice_crystals: {out}"
    );
    assert!(
        out.contains("\"Form\":\"frozen_raindrops\""),
        "sleet → frozen_raindrops: {out}"
    );
    assert!(
        out.contains("\"Form\":\"glaze_of_ice\""),
        "freezing_rain → glaze_of_ice: {out}"
    );
    // The relation runs BACKWARD: bind the form `balls_of_ice`, recall its
    // precipitation type.
    assert!(
        out.contains("\"Precip\":\"hail\""),
        "balls_of_ice → hail (reverse recall): {out}"
    );
    // The answer carries the NOAA National Weather Service citation as its proof,
    // at the `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("weather.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Fog is a suspension of droplets, not one of the falling precipitation
    // types — honest abstention, never a fabricated form.
    assert!(out.contains("\"abstained\":true"), "fog abstains: {out}");
}
