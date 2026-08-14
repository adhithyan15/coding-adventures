//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/tooth-part-property.adj`) driven through the
//! built CLI: a native `table` naming a descriptive property of two named
//! tooth parts, decoded from clauses already sitting unused inside
//! `tooth-parts.adj`'s own already-quoted MedlinePlus/StatPearls source
//! sentences -- a sibling to that table. Resolves binding-query recall
//! (both directions) with the source's citation, and abstains on a real,
//! already-tabled tooth part (enamel) whose own quote states only its
//! location, never a descriptive property -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_toothpartproperty_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/tooth-part-property.adj");
    std::fs::copy(&src, dir.join("tooth-part-property.adj"))
        .expect("copy shipped tooth-part-property.adj");
}

#[test]
fn tooth_part_property_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"tooth-part-property.adj\"\n\
         ? tooth_part_property(dentin, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"tooth_part_property(dentin, harder_than_bone)\""),
        "dentin is harder than bone: {out}"
    );
    assert!(
        out.contains("medlineplus.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the MedlinePlus citation: {out}"
    );
}

#[test]
fn tooth_part_property_recalls_backward_to_cementum() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"tooth-part-property.adj\"\n\
         ? tooth_part_property($Part, calcified_material)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"tooth_part_property(cementum, calcified_material)\""),
        "calcified_material recalls cementum: {out}"
    );
}

#[test]
fn tooth_part_property_abstains_honestly_on_enamel() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"tooth-part-property.adj\"\n\
         ? tooth_part_property(enamel, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "enamel is a real, already-tabled tooth part but its own quote states only its location, never a descriptive property -- honest abstention: {out}"
    );
}
