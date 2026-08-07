//! End-to-end test for the earth-science CLOUD-TYPES facts library
//! (`adj-facts-stdlib/earth-science/cloud-types.adj`) driven through the built
//! CLI: a native `table` of cloud → altitude level resolves a binding-query
//! recall carrying the NOAA / National Weather Service citation, and abstains on
//! a word the page never assigns a level — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn earth_science_cloud_altitude_recall_binds_level_with_citation() {
    let dir = scratch("clouds");
    // Copy the shipped cloud-altitude table beside the entry program and import it.
    let src = facts_stdlib().join("earth-science/cloud-types.adj");
    std::fs::copy(&src, dir.join("cloud-types.adj")).expect("copy shipped cloud-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"cloud-types.adj\"\n\
         ? cloud_altitude(cirrus, $Level)\n\
         ? cloud_altitude(cumulus, $Level)\n\
         ? cloud_altitude(fog, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Cirrus is a high cloud; cumulus is a low cloud — the recalled levels.
    assert!(out.contains("\"Level\":\"high\""), "cirrus → high: {out}");
    assert!(out.contains("\"Level\":\"low\""), "cumulus → low: {out}");
    // The answer carries the NOAA / NWS (weather.gov) citation as its proof,
    // at the authoritative trust tier of a .gov primary source.
    assert!(
        out.contains("https://www.weather.gov/lmk/cloud_classification")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source locator + trust tier: {out}"
    );
    // Fog is never assigned a level on the page — honest abstention, never a
    // fabricated level.
    assert!(out.contains("\"abstained\":true"), "fog abstains: {out}");
}
