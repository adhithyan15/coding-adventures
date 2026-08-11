//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/rainforest-layer.adj`) driven through the
//! built CLI: a native `table` naming the four rainforest layers and a
//! one-fact description of each, per National Geographic Education's "Rain
//! Forest" entry. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rainforestlayer_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/rainforest-layer.adj");
    std::fs::copy(&src, dir.join("rainforest-layer.adj")).expect("copy shipped rainforest-layer.adj");
}

#[test]
fn rainforest_layer_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rainforest-layer.adj\"\n\
         ? rainforest_layer(emergent, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"tallest_trees_dominate_skyline\""),
        "the emergent layer is the tallest-trees layer: {out}"
    );
    assert!(
        out.contains("nationalgeographic.org") && out.contains("\"trust\":\"consensus\""),
        "carries the National Geographic Education citation: {out}"
    );
}

#[test]
fn rainforest_layer_reverse_binds_the_layer_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rainforest-layer.adj\"\n\
         ? rainforest_layer($L, deep_treetop_vegetation_layer)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"L\":\"canopy\""),
        "the canopy is the deep treetop vegetation layer: {out}"
    );
}

#[test]
fn rainforest_layer_abstains_honestly_on_an_untabled_layer() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rainforest-layer.adj\"\n\
         ? rainforest_layer(soil_layer, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "soil layer is not one of the four named rainforest layers -- honest abstention, never invented: {out}"
    );
}
