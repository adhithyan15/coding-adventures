//! End-to-end test for the nutrition FACTS library
//! (`adj-facts-stdlib/nutrition/vegetable-subgroup.adj`) driven through
//! the built CLI: a native `table` naming the USDA MyPlate vegetable
//! subgroup the SAME source span already states for each of the five
//! shipped vegetables -- a sibling to the already-shipped
//! `food-groups.adj` (which only sorts every vegetable into the coarse
//! `vegetables` bucket), decoding the finer subgroup classification
//! already sitting unused inside that table's own header quote. Resolves
//! binding-query recall (both directions) with the source's citation, and
//! covers the full five-vegetable domain with no abstention -- 0 model
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
    let dir = std::env::temp_dir().join(format!("adjcli_vegetablesubgroup_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("nutrition/vegetable-subgroup.adj");
    std::fs::copy(&src, dir.join("vegetable-subgroup.adj"))
        .expect("copy shipped vegetable-subgroup.adj");
}

#[test]
fn vegetable_subgroup_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vegetable-subgroup.adj\"\n\
         ? vegetable_subgroup(broccoli, $Subgroup)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"vegetable_subgroup(broccoli, dark_green)\""),
        "broccoli is a dark-green vegetable: {out}"
    );
    assert!(
        out.contains("myplate.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USDA MyPlate citation: {out}"
    );
}

#[test]
fn vegetable_subgroup_recalls_backward_from_a_bound_subgroup() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vegetable-subgroup.adj\"\n\
         ? vegetable_subgroup($Vegetable, red_and_orange)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"vegetable_subgroup(carrots, red_and_orange)\""),
        "carrots are a red-and-orange vegetable: {out}"
    );
}

#[test]
fn vegetable_subgroup_covers_the_full_domain_without_abstention() {
    let dir = scratch("full");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vegetable-subgroup.adj\"\n\
         ? vegetable_subgroup(corn, $Subgroup)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"vegetable_subgroup(corn, starchy)\""),
        "corn is a starchy vegetable: {out}"
    );
    assert!(
        !out.contains("\"abstained\":true"),
        "every shipped vegetable has a subgroup -- no abstention expected: {out}"
    );
}
