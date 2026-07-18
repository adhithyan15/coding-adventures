//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/hand-bones.adj`) driven through the built CLI:
//! a native `table` of the three groups of hand bones → the part of the hand
//! each occupies resolves binding-query recalls (forward AND backward) with the
//! source's InformedHealth.org / NCBI Bookshelf citation, and abstains on a
//! word that is not one of the three hand-bone groups (the femur, a leg bone) —
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factst_{tag}_{}", std::process::id()));
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
fn anatomy_hand_bones_recall_binds_region_with_citation() {
    let dir = scratch("handbones");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/hand-bones.adj");
    std::fs::copy(&src, dir.join("hand-bones.adj")).expect("copy shipped hand-bones.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"hand-bones.adj\"\n\
         ? hand_bone_region(carpals, $Region)\n\
         ? hand_bone_region(metacarpals, $Region)\n\
         ? hand_bone_region(phalanges, $Region)\n\
         ? hand_bone_region($Group, fingers)\n\
         ? hand_bone_region(femur, $Region)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The carpals sit at the base of the hand, the metacarpals across the middle
    // of the hand, the phalanges are the finger bones — the recalled regions
    // (forward binds), each in the source's own words.
    assert!(
        out.contains("\"Region\":\"base_of_hand\""),
        "carpals → base_of_hand: {out}"
    );
    assert!(
        out.contains("\"Region\":\"middle_of_hand\""),
        "metacarpals → middle_of_hand: {out}"
    );
    assert!(
        out.contains("\"Region\":\"fingers\""),
        "phalanges → fingers: {out}"
    );
    // The relation runs BACKWARD: bind the region `fingers`, recall its group.
    assert!(
        out.contains("\"Group\":\"phalanges\""),
        "fingers → phalanges (reverse recall): {out}"
    );
    // The answer carries the InformedHealth.org / NCBI Bookshelf citation as its
    // proof, at the `consensus` trust tier for a teaching summary.
    assert!(
        out.contains("ncbi.nlm.nih.gov/books/NBK279362") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // The femur is a leg bone, not one of the three hand-bone groups — honest
    // abstention, never a fabricated region.
    assert!(out.contains("\"abstained\":true"), "femur abstains: {out}");
}
