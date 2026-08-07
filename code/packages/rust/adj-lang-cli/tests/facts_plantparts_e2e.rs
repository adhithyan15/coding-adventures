//! End-to-end test for the biology PLANT-PARTS facts library
//! (`adj-facts-stdlib/biology/plant-parts.adj`) driven through the built CLI:
//! a native `table` of plant-part → primary-function resolves binding-query
//! recalls (forward and backward) with the source's citation, and abstains on a
//! non-plant-part — 0 model calls.

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
fn biology_plant_parts_recall_binds_function_with_citation() {
    let dir = scratch("plantparts");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/plant-parts.adj");
    std::fs::copy(&src, dir.join("plant-parts.adj")).expect("copy shipped plant-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-parts.adj\"\n\
         ? plant_part_function(roots, $Function)\n\
         ? plant_part_function($Part, photosynthesis)\n\
         ? plant_part_function(mushroom, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The roots absorb water — the recalled forward binding.
    assert!(
        out.contains("\"Function\":\"absorb_water\""),
        "roots → absorb_water: {out}"
    );
    // The relation runs BACKWARD too: what does photosynthesis? — the leaves.
    assert!(
        out.contains("\"Part\":\"leaves\""),
        "photosynthesis ← leaves (reverse recall): {out}"
    );
    // The answer carries the UF/IFAS Extension citation, at the authoritative tier.
    assert!(
        out.contains("blogs.ifas.ufl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation with its locator + trust: {out}"
    );
    // A mushroom is a fungus, not a plant part — honest abstention, never a
    // fabricated function.
    assert!(out.contains("\"abstained\":true"), "mushroom abstains: {out}");
}
