//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/blood-vessels.adj`) driven through the built CLI:
//! a native `table` of the three main blood-vessel types → the defining
//! function each performs resolves binding-query recalls (forward AND backward)
//! with the source's NCI SEER Training Modules citation, and abstains on a word
//! that is not one of the three main blood vessels (a lymphatic vessel) — 0
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsbv_{tag}_{}", std::process::id()));
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
fn biology_blood_vessels_recall_binds_function_with_citation() {
    let dir = scratch("bloodvessels");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/blood-vessels.adj");
    std::fs::copy(&src, dir.join("blood-vessels.adj")).expect("copy shipped blood-vessels.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"blood-vessels.adj\"\n\
         ? vessel_function(artery, $Function)\n\
         ? vessel_function(vein, $Function)\n\
         ? vessel_function(capillary, $Function)\n\
         ? vessel_function($Vessel, toward_heart)\n\
         ? vessel_function(lymphatic, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Arteries carry blood away from the heart, veins carry blood toward the
    // heart, and capillaries are where the exchange of materials happens — the
    // recalled functions (forward binds).
    assert!(
        out.contains("\"Function\":\"away_from_heart\""),
        "artery → away_from_heart: {out}"
    );
    assert!(
        out.contains("\"Function\":\"toward_heart\""),
        "vein → toward_heart: {out}"
    );
    assert!(
        out.contains("\"Function\":\"exchange_of_materials\""),
        "capillary → exchange_of_materials: {out}"
    );
    // The relation runs BACKWARD: bind the function `toward_heart`, recall its
    // vessel type.
    assert!(
        out.contains("\"Vessel\":\"vein\""),
        "toward_heart → vein (reverse recall): {out}"
    );
    // The answer carries the NCI SEER Training Modules citation as its proof, at
    // the `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A lymphatic vessel is not one of the three main blood vessels — honest
    // abstention, never a fabricated function.
    assert!(
        out.contains("\"abstained\":true"),
        "lymphatic abstains: {out}"
    );
}
