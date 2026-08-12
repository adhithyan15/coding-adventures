//! End-to-end test for the environment FACTS library
//! (`adj-facts-stdlib/environment/ecosystem-factor-type.adj`) driven
//! through the built CLI: a native `table` naming the two kinds of
//! ecosystem factor and what defines each, quoted verbatim from two
//! sibling National Geographic Education resource pages. 0 answer-time
//! model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_ecosystem_factor_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("environment/ecosystem-factor-type.adj");
    std::fs::copy(&src, dir.join("ecosystem-factor-type.adj")).expect("copy shipped ecosystem-factor-type.adj");
}

#[test]
fn ecosystem_factor_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ecosystem-factor-type.adj\"\n\
         ? ecosystem_factor_type(biotic, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"a_living_organism_that_shapes_its_environment\""),
        "biotic means a_living_organism_that_shapes_its_environment: {out}"
    );
    assert!(
        out.contains("nationalgeographic.org") && out.contains("\"trust\":\"consensus\""),
        "carries the National Geographic Education citation: {out}"
    );
}

#[test]
fn ecosystem_factor_type_reverse_binds_the_factor_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ecosystem-factor-type.adj\"\n\
         ? ecosystem_factor_type($F, a_non_living_part_of_an_ecosystem_that_shapes_its_environment)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"F\":\"abiotic\""),
        "the shipped a_non_living_part_of_an_ecosystem_that_shapes_its_environment example is abiotic: {out}"
    );
}

#[test]
fn ecosystem_factor_type_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ecosystem-factor-type.adj\"\n\
         ? ecosystem_factor_type(producer, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "producer is a real ecology term, but it names a food-chain role (see food-chain-roles.adj), not a biotic/abiotic factor type -- honest abstention, never invented: {out}"
    );
}
