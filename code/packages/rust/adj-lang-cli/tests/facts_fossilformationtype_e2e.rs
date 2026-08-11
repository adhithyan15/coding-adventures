//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/fossil-formation-type.adj`) driven through
//! the built CLI: a native `table` naming three ways a fossil can form and
//! what each actually is, quoted verbatim from Ducksters' "Earth Science
//! for Kids: Fossils" page. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_fossil_formation_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geology/fossil-formation-type.adj");
    std::fs::copy(&src, dir.join("fossil-formation-type.adj"))
        .expect("copy shipped fossil-formation-type.adj");
}

#[test]
fn fossil_formation_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fossil-formation-type.adj\"\n\
         ? fossil_formation_type(amber, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"preserved_in_hardened_tree_sap\""),
        "amber means preserved_in_hardened_tree_sap: {out}"
    );
    assert!(
        out.contains("ducksters.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Ducksters citation: {out}"
    );
}

#[test]
fn fossil_formation_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fossil-formation-type.adj\"\n\
         ? fossil_formation_type($T, impression_of_a_living_organism)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"cast_or_mold\""),
        "the shipped impression_of_a_living_organism example is cast_or_mold: {out}"
    );
}

#[test]
fn fossil_formation_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"fossil-formation-type.adj\"\n\
         ? fossil_formation_type(freezing, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "freezing is a real preservation method the source covers but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
