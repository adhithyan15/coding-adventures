//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/skeleton-bones.adj`) driven through the built CLI:
//! a native `table` of common human bone → body region resolves binding-query
//! recalls (forward AND backward) with the source's NIH / MedlinePlus citation,
//! and abstains on a word that is not one of these bones (the spleen) — 0 model
//! calls.

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
fn biology_skeleton_bones_recall_binds_region_with_citation() {
    let dir = scratch("skeletonbones");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/skeleton-bones.adj");
    std::fs::copy(&src, dir.join("skeleton-bones.adj")).expect("copy shipped skeleton-bones.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"skeleton-bones.adj\"\n\
         ? bone_region(femur, $Region)\n\
         ? bone_region(patella, $Region)\n\
         ? bone_region(sternum, $Region)\n\
         ? bone_region($Bone, arm)\n\
         ? bone_region(spleen, $Region)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The femur is a leg bone, the patella is at the knee, the sternum is in the
    // chest — the recalled regions (forward binds).
    assert!(out.contains("\"Region\":\"leg\""), "femur → leg: {out}");
    assert!(out.contains("\"Region\":\"knee\""), "patella → knee: {out}");
    assert!(out.contains("\"Region\":\"chest\""), "sternum → chest: {out}");
    // The relation runs BACKWARD: bind the region `arm`, recall the arm bones.
    assert!(
        out.contains("\"Bone\":\"humerus\"")
            && out.contains("\"Bone\":\"radius\"")
            && out.contains("\"Bone\":\"ulna\""),
        "arm → humerus ; radius ; ulna (reverse recall): {out}"
    );
    // The answer carries the NIH / NLM MedlinePlus citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("medlineplus.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The spleen is an organ, not a bone — honest abstention, never a fabricated
    // region.
    assert!(out.contains("\"abstained\":true"), "spleen abstains: {out}");
}
