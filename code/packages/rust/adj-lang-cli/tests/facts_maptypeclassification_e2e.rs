//! End-to-end test for the geography FACTS library
//! (`adj-facts-stdlib/geography/map-type-classification.adj`) driven
//! through the built CLI: a native `table` naming the classification of a
//! topographic map, decoded from a clause already sitting unused inside
//! `map-type.adj`'s own already-quoted Geology.com source sentence -- a
//! sibling to that table. Resolves binding-query recall (both directions)
//! with the source's citation, and abstains on a real, already-tabled map
//! type (political) whose own quote never classifies it as a kind of map
//! -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_maptypeclassification_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geography/map-type-classification.adj");
    std::fs::copy(&src, dir.join("map-type-classification.adj"))
        .expect("copy shipped map-type-classification.adj");
}

#[test]
fn map_type_classification_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"map-type-classification.adj\"\n\
         ? map_type_classification(topographic, $Classification)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"map_type_classification(topographic, reference_map)\""),
        "topographic maps are classified as reference maps: {out}"
    );
    assert!(
        out.contains("geology.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Geology.com citation: {out}"
    );
}

#[test]
fn map_type_classification_recalls_backward_to_topographic() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"map-type-classification.adj\"\n\
         ? map_type_classification($Type, reference_map)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"map_type_classification(topographic, reference_map)\""),
        "reference_map recalls the topographic map type: {out}"
    );
}

#[test]
fn map_type_classification_abstains_honestly_on_political() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"map-type-classification.adj\"\n\
         ? map_type_classification(political, $Classification)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "political is a real, already-tabled map type but its own quote never classifies it as a kind of map -- honest abstention: {out}"
    );
}
