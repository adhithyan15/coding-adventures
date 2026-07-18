//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/foot-bones.adj`) driven through the built CLI:
//! a native `table` of foot-bone group → the region of the foot it occupies
//! resolves a binding-query recall with the source's citation, runs the
//! relation backward (region → bone group), and abstains on something that is
//! not a foot-bone group — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsfoot_{tag}_{}", std::process::id()));
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
fn anatomy_foot_bones_recall_binds_region_with_citation() {
    let dir = scratch("footbones");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/foot-bones.adj");
    std::fs::copy(&src, dir.join("foot-bones.adj")).expect("copy shipped foot-bones.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"foot-bones.adj\"\n\
         ? foot_region(tarsals, $Region)\n\
         ? foot_region(metatarsals, $Region)\n\
         ? foot_region($Group, toes)\n\
         ? foot_region(carpals, $Region)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The tarsals form the heel and the ankle at the back of the foot; the
    // metatarsals are the long bones of the midfoot — the recalled regions,
    // each a plain lowercase atom copied from the source's own wording.
    assert!(out.contains("\"Region\":\"heel_and_ankle\""), "tarsals → heel_and_ankle: {out}");
    assert!(out.contains("\"Region\":\"midfoot\""), "metatarsals → midfoot: {out}");
    // The relation runs backward: the region `toes` recalls the group of bones
    // that make up the toes — the phalanges.
    assert!(out.contains("\"Group\":\"phalanges\""), "toes → phalanges (reverse recall): {out}");
    // The answer carries the Wikipedia citation as its proof, at the honest
    // `consensus` trust tier of a tertiary encyclopedia source.
    assert!(
        out.contains("en.wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // "carpals" are the wrist bones, not a foot-bone group — honest
    // abstention, never a fabricated region.
    assert!(out.contains("\"abstained\":true"), "unknown foot-bone group abstains: {out}");
}
