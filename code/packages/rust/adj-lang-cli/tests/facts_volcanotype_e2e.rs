//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/volcano-type.adj`) driven through the built
//! CLI: a native `table` naming three types of volcano and what each
//! actually is, quoted verbatim from USGS's "About Volcanoes" page. 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_volcano_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geology/volcano-type.adj");
    std::fs::copy(&src, dir.join("volcano-type.adj")).expect("copy shipped volcano-type.adj");
}

#[test]
fn volcano_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"volcano-type.adj\"\n\
         ? volcano_type(shield_volcano, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"built_almost_entirely_of_fluid_lava_flows\""),
        "shield_volcano means built_almost_entirely_of_fluid_lava_flows: {out}"
    );
    assert!(
        out.contains("usgs.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS citation: {out}"
    );
}

#[test]
fn volcano_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"volcano-type.adj\"\n\
         ? volcano_type($T, is_the_simplest_type_of_volcano)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"cinder_cone\""),
        "the shipped is_the_simplest_type_of_volcano example is cinder_cone: {out}"
    );
}

#[test]
fn volcano_type_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"volcano-type.adj\"\n\
         ? volcano_type(lava_dome, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "lava_dome is a real term the source names but explicitly disclaims as technically not a volcano type, not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
