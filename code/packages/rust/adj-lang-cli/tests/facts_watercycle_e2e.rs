//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/water-cycle.adj`) driven through the built
//! CLI: a native `table` of water-cycle stage → step number resolves a
//! binding-query recall with the USGS citation, and abstains on the Sun (which
//! DRIVES the cycle but is not one of its stages) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factswc_{tag}_{}", std::process::id()));
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
fn water_cycle_recall_binds_step_number_with_citation() {
    let dir = scratch("watercycle");
    // Copy the shipped earth-science table beside the entry program and import it.
    let src = facts_stdlib().join("earth-science/water-cycle.adj");
    std::fs::copy(&src, dir.join("water-cycle.adj")).expect("copy shipped water-cycle.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"water-cycle.adj\"\n\
         ? water_cycle_stage(evaporation, $N)\n\
         ? water_cycle_stage(precipitation, $N)\n\
         ? water_cycle_stage(sun, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Evaporation is the first step of the cycle; precipitation is the third —
    // the recalled step numbers, straight from the grounded rows.
    assert!(out.contains("\"N\":\"1\""), "evaporation → 1: {out}");
    assert!(out.contains("\"N\":\"3\""), "precipitation → 3: {out}");
    // The answer carries the USGS citation (authoritative, a .gov source) as proof.
    assert!(
        out.contains("water.usgs.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS source citation: {out}"
    );
    // The Sun drives the cycle but is not a stage — honest abstention, never a
    // fabricated step number.
    assert!(out.contains("\"abstained\":true"), "sun abstains: {out}");
}
