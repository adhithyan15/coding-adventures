//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/tooth-parts.adj`) driven through the built CLI:
//! a native `table` of the structural parts of a tooth → the role / description
//! the source states resolves binding-query recalls (forward AND backward) with
//! the source's NIH (MedlinePlus / NCBI Bookshelf) citation, and abstains on a
//! word that is not one of these tooth parts (the femur) — 0 model calls.
//!
//! This library is DISTINCT from anatomy/tooth-types (tooth TYPE → job); here
//! the key is a tooth PART (enamel, dentin, pulp, cementum, crown, root).

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
fn anatomy_tooth_parts_recall_binds_role_with_citation() {
    let dir = scratch("toothparts");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/tooth-parts.adj");
    std::fs::copy(&src, dir.join("tooth-parts.adj")).expect("copy shipped tooth-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"tooth-parts.adj\"\n\
         ? tooth_part_role(enamel, $Role)\n\
         ? tooth_part_role(dentin, $Role)\n\
         ? tooth_part_role(pulp, $Role)\n\
         ? tooth_part_role($Part, covers_roots)\n\
         ? tooth_part_role(femur, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Enamel is the outer surface of the crown, dentin sits just beneath the
    // enamel, and the pulp holds the blood vessels and nerves — the recalled
    // roles (forward binds).
    assert!(
        out.contains("\"Role\":\"outer_surface\""),
        "enamel → outer_surface: {out}"
    );
    assert!(
        out.contains("\"Role\":\"beneath_enamel\""),
        "dentin → beneath_enamel: {out}"
    );
    assert!(
        out.contains("\"Role\":\"blood_vessels_and_nerves\""),
        "pulp → blood_vessels_and_nerves: {out}"
    );
    // The relation runs BACKWARD: bind the role `covers_roots`, recall which
    // part it is.
    assert!(
        out.contains("\"Part\":\"cementum\""),
        "covers_roots → cementum (reverse recall): {out}"
    );
    // The answer carries the MedlinePlus (NIH / U.S. NLM) citation as its proof,
    // at the `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("medlineplus.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The femur is a leg bone, not a tooth part — honest abstention, never a
    // fabricated role.
    assert!(out.contains("\"abstained\":true"), "femur abstains: {out}");
}
