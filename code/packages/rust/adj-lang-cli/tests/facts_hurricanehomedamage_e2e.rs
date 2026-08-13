//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/hurricane-category-home-damage.adj`)
//! driven through the built CLI: a native `table` naming the SPECIFIC
//! well-built-home damage effect the NHC describes for each Saffir-Simpson
//! hurricane category -- a sibling to the already-shipped
//! `hurricane-categories.adj` (which only carries ONE generic damage word
//! per category), decoding spans already sitting unused inside that
//! table's own provenance block. Resolves binding-query recall (both
//! directions) with the source's citation, and abstains on a category that
//! is not one of the five Saffir-Simpson categories -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_hurricanehomedamage_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("meteorology/hurricane-category-home-damage.adj");
    std::fs::copy(&src, dir.join("hurricane-category-home-damage.adj"))
        .expect("copy shipped hurricane-category-home-damage.adj");
}

#[test]
fn hurricane_home_damage_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"hurricane-category-home-damage.adj\"\n\
         ? hurricane_home_damage(category_1, $HomeDamageEffect)\n\
         ? hurricane_home_damage(category_5, $HomeDamageEffect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"hurricane_home_damage(category_1, damage_to_roof_shingles_vinyl_siding_and_gutters)\""),
        "category_1 effect: {out}"
    );
    assert!(
        out.contains("\"term\":\"hurricane_home_damage(category_5, total_roof_failure_and_wall_collapse)\""),
        "category_5 effect: {out}"
    );
    assert!(
        out.contains("nhc.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NHC citation: {out}"
    );
}

#[test]
fn hurricane_home_damage_recalls_backward_from_a_bound_effect() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"hurricane-category-home-damage.adj\"\n\
         ? hurricane_home_damage($Category, total_roof_failure_and_wall_collapse)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Category\":\"category_5\""),
        "total roof failure and wall collapse names category_5: {out}"
    );
}

#[test]
fn hurricane_home_damage_abstains_honestly_on_category_6() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"hurricane-category-home-damage.adj\"\n\
         ? hurricane_home_damage(category_6, $HomeDamageEffect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "there is no category 6 on the Saffir-Simpson scale -- honest abstention: {out}"
    );
}
