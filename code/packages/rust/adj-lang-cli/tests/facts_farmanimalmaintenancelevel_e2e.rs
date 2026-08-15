//! End-to-end test for the agriculture FACTS library
//! (`adj-facts-stdlib/agriculture/farm-animal-maintenance-level.adj`)
//! driven through the built CLI: a native `table` naming the
//! husbandry-difficulty descriptor a source already states for a farm
//! animal, decoded from the LEADING clause of the SAME sentence
//! `farm-animals.adj` already carries as its own provenance envelope --
//! a sibling to that table (only the trailing "wool, meat, milk" clause
//! was ever decoded there). Resolves binding-query recall (both
//! directions) with the source's citation, and abstains on a real,
//! already-tabled animal (goat) whose own cited span states no
//! husbandry-difficulty descriptor -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_farmanimalmaintenancelevel_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("agriculture/farm-animal-maintenance-level.adj");
    std::fs::copy(&src, dir.join("farm-animal-maintenance-level.adj"))
        .expect("copy shipped farm-animal-maintenance-level.adj");
}

#[test]
fn farm_animal_maintenance_level_recalls_sheep_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-maintenance-level.adj\"\n\
         ? farm_animal_maintenance_level(sheep, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"farm_animal_maintenance_level(sheep, low_maintenance)\""),
        "sheep are low maintenance: {out}"
    );
    assert!(
        out.contains("cfsph.iastate.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the CFSPH citation: {out}"
    );
}

#[test]
fn farm_animal_maintenance_level_recalls_backward_to_sheep() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-maintenance-level.adj\"\n\
         ? farm_animal_maintenance_level($Animal, low_maintenance)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"farm_animal_maintenance_level(sheep, low_maintenance)\""),
        "low_maintenance names sheep: {out}"
    );
}

#[test]
fn farm_animal_maintenance_level_abstains_honestly_on_goat() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"farm-animal-maintenance-level.adj\"\n\
         ? farm_animal_maintenance_level(goat, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "goat's own cited span states no husbandry-difficulty descriptor -- honest abstention: {out}"
    );
}
