//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/flower-parts.adj`) driven through the built CLI:
//! a native `table` of flower parts → the function / role the source states
//! resolves binding-query recalls (forward AND backward) with the source's
//! University of Illinois Extension citation, and abstains on a word that is not
//! one of these flower parts (the leaf) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factfp_{tag}_{}", std::process::id()));
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
fn biology_flower_parts_recall_binds_function_with_citation() {
    let dir = scratch("flowerparts");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/flower-parts.adj");
    std::fs::copy(&src, dir.join("flower-parts.adj")).expect("copy shipped flower-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"flower-parts.adj\"\n\
         ? flower_part_function(petal, $Function)\n\
         ? flower_part_function(anther, $Function)\n\
         ? flower_part_function(ovary, $Function)\n\
         ? flower_part_function(stigma, $Function)\n\
         ? flower_part_function($Part, male)\n\
         ? flower_part_function(leaf, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The petals attract pollinators, the anthers carry the pollen, the ovary
    // contains the ovules, the stigma traps the pollen — the recalled functions
    // (forward binds).
    assert!(
        out.contains("\"Function\":\"attract_pollinators\""),
        "petal → attract_pollinators: {out}"
    );
    assert!(
        out.contains("\"Function\":\"carry_pollen\""),
        "anther → carry_pollen: {out}"
    );
    assert!(
        out.contains("\"Function\":\"contains_ovules\""),
        "ovary → contains_ovules: {out}"
    );
    assert!(
        out.contains("\"Function\":\"traps_pollen\""),
        "stigma → traps_pollen: {out}"
    );
    // The relation runs BACKWARD: bind the function `male`, recall its flower part.
    assert!(
        out.contains("\"Part\":\"stamen\""),
        "male → stamen (reverse recall): {out}"
    );
    // The answer carries the Illinois Extension citation as its proof, at the
    // `authoritative` trust tier for a .edu-primary botany/extension source.
    assert!(
        out.contains("web.extension.illinois.edu")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The leaf is a plant part, not a part of the flower — honest abstention,
    // never a fabricated function.
    assert!(out.contains("\"abstained\":true"), "leaf abstains: {out}");
}
