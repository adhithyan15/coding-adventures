//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/plant-need.adj`) driven through the built
//! CLI: a native `table` naming three photosynthesis inputs and the role
//! each plays, per Washington State University's "Ask Dr. Universe" science
//! outreach column. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_plantneed_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/plant-need.adj");
    std::fs::copy(&src, dir.join("plant-need.adj")).expect("copy shipped plant-need.adj");
}

#[test]
fn plant_need_recall_binds_the_role_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-need.adj\"\n\
         ? plant_need(sunlight, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Role\":\"excites_chlorophyll_electrons\""),
        "sunlight's role is to excite chlorophyll electrons: {out}"
    );
    assert!(
        out.contains("askdruniverse.wsu.edu") && out.contains("\"trust\":\"consensus\""),
        "carries the Ask Dr. Universe citation: {out}"
    );
}

#[test]
fn plant_need_reverse_binds_the_input_for_that_role() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-need.adj\"\n\
         ? plant_need($Need, combined_to_make_glucose)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Need\":\"carbon_dioxide\""),
        "the input combined to make glucose is carbon dioxide: {out}"
    );
}

#[test]
fn plant_need_abstains_honestly_on_an_untabled_input() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"plant-need.adj\"\n\
         ? plant_need(soil, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "soil is a real plant-growth input but has no shipped role in this table -- honest abstention, never invented: {out}"
    );
}
