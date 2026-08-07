//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/gas-laws.adj`) driven through the built CLI:
//! a native `table` of the four named simple gas laws → the pair of quantities
//! each relates resolves binding-query recalls (forward AND backward) with the
//! source's Chemistry LibreTexts (CK-12) citation, and abstains on a name that
//! is not one of the four simple gas laws (Newton) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsg_{tag}_{}", std::process::id()));
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
fn chemistry_gas_laws_recall_binds_pair_with_citation() {
    let dir = scratch("gaslaws");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/gas-laws.adj");
    std::fs::copy(&src, dir.join("gas-laws.adj")).expect("copy shipped gas-laws.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"gas-laws.adj\"\n\
         ? gas_law_relates(boyle, $Pair)\n\
         ? gas_law_relates(charles, $Pair)\n\
         ? gas_law_relates(avogadro, $Pair)\n\
         ? gas_law_relates($Law, pressure_temperature)\n\
         ? gas_law_relates(newton, $Pair)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Boyle relates pressure and volume, Charles relates volume and temperature,
    // Avogadro relates volume and moles — the recalled pairs (forward binds).
    assert!(
        out.contains("\"Pair\":\"pressure_volume\""),
        "boyle → pressure_volume: {out}"
    );
    assert!(
        out.contains("\"Pair\":\"volume_temperature\""),
        "charles → volume_temperature: {out}"
    );
    assert!(
        out.contains("\"Pair\":\"volume_moles\""),
        "avogadro → volume_moles: {out}"
    );
    // The relation runs BACKWARD: bind the pair `pressure_temperature`, recall
    // which law names it.
    assert!(
        out.contains("\"Law\":\"gay_lussac\""),
        "pressure_temperature → gay_lussac (reverse recall): {out}"
    );
    // The answer carries the Chemistry LibreTexts citation as its proof, at the
    // `consensus` trust tier for an open teaching resource.
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // Newton's laws are mechanics, not one of the four simple gas laws — honest
    // abstention, never a fabricated relation.
    assert!(out.contains("\"abstained\":true"), "newton abstains: {out}");
}
