//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/eye-part-property.adj`) driven through the
//! built CLI: a native `table` naming a descriptive PROPERTY of three eye
//! parts, decoded from parenthetical spans already sitting unused inside
//! `eye-parts.adj`'s own already-quoted NEI source sentences -- a sibling to
//! that table. Resolves binding-query recall (both directions) with the
//! source's citation, and abstains on a real, already-tabled eye part
//! (lens) whose own quote states no descriptive property, only a
//! function -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_eyepartproperty_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/eye-part-property.adj");
    std::fs::copy(&src, dir.join("eye-part-property.adj"))
        .expect("copy shipped eye-part-property.adj");
}

#[test]
fn eye_part_property_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"eye-part-property.adj\"\n\
         ? eye_part_property(iris, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"eye_part_property(iris, colored_part_of_eye)\""),
        "the iris is the colored part of the eye: {out}"
    );
    assert!(
        out.contains("nei.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NEI citation: {out}"
    );
}

#[test]
fn eye_part_property_recalls_backward() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"eye-part-property.adj\"\n\
         ? eye_part_property($Part, dome_shaped)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"eye_part_property(cornea, dome_shaped)\""),
        "backward recall should find cornea: {out}"
    );
}

#[test]
fn eye_part_property_abstains_honestly_on_lens() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"eye-part-property.adj\"\n\
         ? eye_part_property(lens, $Property)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "lens's own quote states no descriptive property -- honest abstention: {out}"
    );
}
