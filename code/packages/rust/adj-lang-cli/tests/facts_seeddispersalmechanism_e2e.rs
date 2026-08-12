//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/seed-dispersal-mechanism.adj`) driven through
//! the built CLI: a native `table` naming four seed-dispersal mechanisms
//! and how each actually works, quoted verbatim from Wikipedia's "Seed
//! dispersal" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_seed_dispersal_mechanism_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/seed-dispersal-mechanism.adj");
    std::fs::copy(&src, dir.join("seed-dispersal-mechanism.adj"))
        .expect("copy shipped seed-dispersal-mechanism.adj");
}

#[test]
fn seed_dispersal_mechanism_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"seed-dispersal-mechanism.adj\"\n\
         ? seed_dispersal_mechanism(barochory, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"uses_gravity_as_a_simple_means_of_seed_dispersal\""),
        "barochory means uses_gravity_as_a_simple_means_of_seed_dispersal: {out}"
    );
    assert!(
        out.contains("en.wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wikipedia citation: {out}"
    );
}

#[test]
fn seed_dispersal_mechanism_reverse_binds_the_mechanism_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"seed-dispersal-mechanism.adj\"\n\
         ? seed_dispersal_mechanism($M, seed_is_forcefully_ejected_by_explosive_dehiscence_of_the_fruit)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"M\":\"ballochory\""),
        "the shipped seed_is_forcefully_ejected_by_explosive_dehiscence_of_the_fruit example is ballochory: {out}"
    );
}

#[test]
fn seed_dispersal_mechanism_abstains_honestly_on_an_untabled_mechanism() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"seed-dispersal-mechanism.adj\"\n\
         ? seed_dispersal_mechanism(hydrochory, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "hydrochory is a real dispersal mechanism the source names, but every candidate sentence checked either conflates the mechanism with dispersal distance or is qualified by a following sentence rather than standing alone -- honest abstention, never invented: {out}"
    );
}
