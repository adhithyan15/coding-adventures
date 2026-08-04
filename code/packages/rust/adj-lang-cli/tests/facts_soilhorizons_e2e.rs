//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/soil-horizons.adj`) driven through the built
//! CLI: a native `table` of soil master horizon → what it is made of resolves a
//! binding-query recall with the UNL passel citation, and abstains on `humus`
//! (a component of soil organic matter, not one of the master horizons) — 0
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
    let dir = std::env::temp_dir().join(format!("adjcli_factssh_{tag}_{}", std::process::id()));
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
fn soil_horizons_recall_binds_material_with_citation() {
    let dir = scratch("soilhorizons");
    // Copy the shipped earth-science table beside the entry program and import it.
    let src = facts_stdlib().join("earth-science/soil-horizons.adj");
    std::fs::copy(&src, dir.join("soil-horizons.adj")).expect("copy shipped soil-horizons.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"soil-horizons.adj\"\n\
         ? horizon_material(o, $Material)\n\
         ? horizon_material(c, $Material)\n\
         ? horizon_material($Horizon, bedrock)\n\
         ? horizon_material(humus, $Material)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The O horizon is organic matter; the C horizon is parent material — the
    // recalled materials, straight from the grounded rows.
    assert!(
        out.contains("\"Material\":\"organic_matter\""),
        "o → organic_matter: {out}"
    );
    assert!(
        out.contains("\"Material\":\"parent_material\""),
        "c → parent_material: {out}"
    );
    // Reverse bind: which horizon is bedrock? The R layer.
    assert!(out.contains("\"Horizon\":\"r\""), "bedrock → r (reverse bind): {out}");
    // The answer carries the UNL passel citation (authoritative, a university
    // soil-science teaching resource) as proof.
    assert!(
        out.contains("passel2.unl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the UNL passel source citation: {out}"
    );
    // Humus is a component of soil organic matter, not a master horizon — honest
    // abstention, never a fabricated material.
    assert!(out.contains("\"abstained\":true"), "humus abstains: {out}");
}
