//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/leaf-parts.adj`) driven through the built CLI:
//! a native `table` of leaf parts → the defining token / function the source
//! states resolves binding-query recalls (forward AND backward) with the
//! source's Colorado State University Extension citation, and abstains on a word
//! that is not one of these leaf parts (the root) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factlp_{tag}_{}", std::process::id()));
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
fn biology_leaf_parts_recall_binds_role_with_citation() {
    let dir = scratch("leafparts");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/leaf-parts.adj");
    std::fs::copy(&src, dir.join("leaf-parts.adj")).expect("copy shipped leaf-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"leaf-parts.adj\"\n\
         ? leaf_part_role(blade, $Role)\n\
         ? leaf_part_role(stomata, $Role)\n\
         ? leaf_part_role(cuticle, $Role)\n\
         ? leaf_part_role(veins, $Role)\n\
         ? leaf_part_role($Part, stalk)\n\
         ? leaf_part_role(root, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The blade is the flattened part, the stomata are for gas exchange, the
    // cuticle prevents water loss, the veins are xylem and phloem — the recalled
    // roles (forward binds).
    assert!(
        out.contains("\"Role\":\"flattened\""),
        "blade → flattened: {out}"
    );
    assert!(
        out.contains("\"Role\":\"gas_exchange\""),
        "stomata → gas_exchange: {out}"
    );
    assert!(
        out.contains("\"Role\":\"prevents_water_loss\""),
        "cuticle → prevents_water_loss: {out}"
    );
    assert!(
        out.contains("\"Role\":\"xylem_and_phloem\""),
        "veins → xylem_and_phloem: {out}"
    );
    // The relation runs BACKWARD: bind the role `stalk`, recall its leaf part.
    assert!(
        out.contains("\"Part\":\"petiole\""),
        "stalk → petiole (reverse recall): {out}"
    );
    // The answer carries the CSU Extension citation as its proof, at the
    // `authoritative` trust tier for a .edu-primary botany/extension source.
    assert!(
        out.contains("cmg.extension.colostate.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The root is a plant part, not a part of the leaf — honest abstention,
    // never a fabricated role.
    assert!(out.contains("\"abstained\":true"), "root abstains: {out}");
}
