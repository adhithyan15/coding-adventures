//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/seasons.adj`) driven through the built CLI:
//! a native `table` of meteorological season → start month (Northern Hemisphere)
//! resolves a binding-query recall with the NOAA citation, and abstains on an
//! equinox (an astronomical instant, not a meteorological season) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsseason_{tag}_{}", std::process::id()));
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
fn seasons_recall_binds_start_month_with_citation() {
    let dir = scratch("seasons");
    // Copy the shipped earth-science table beside the entry program and import it.
    let src = facts_stdlib().join("earth-science/seasons.adj");
    std::fs::copy(&src, dir.join("seasons.adj")).expect("copy shipped seasons.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"seasons.adj\"\n\
         ? season_start_month(spring, $Month)\n\
         ? season_start_month(summer, $Month)\n\
         ? season_start_month(equinox, $Month)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Meteorological spring begins in March; summer begins in June — the recalled
    // start months, straight from the grounded rows.
    assert!(out.contains("\"Month\":\"march\""), "spring → march: {out}");
    assert!(out.contains("\"Month\":\"june\""), "summer → june: {out}");
    // The answer carries the NOAA/NCEI citation (authoritative, a .gov source) as
    // proof — both the locator URL and the honest trust tier.
    assert!(
        out.contains("ncei.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA source citation: {out}"
    );
    // An equinox is an astronomical instant, not a meteorological season — honest
    // abstention, never a fabricated month.
    assert!(out.contains("\"abstained\":true"), "equinox abstains: {out}");
}
