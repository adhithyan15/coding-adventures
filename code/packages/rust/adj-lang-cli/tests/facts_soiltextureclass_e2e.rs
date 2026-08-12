//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/soil-texture-class.adj`) driven
//! through the built CLI: a native `table` naming the three soil
//! particle-size separates (clay/silt/sand) and the diameter range that
//! defines each, quoted verbatim from Wikipedia's "Soil texture" article.
//! 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_soil_texture_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("earth-science/soil-texture-class.adj");
    std::fs::copy(&src, dir.join("soil-texture-class.adj")).expect("copy shipped soil-texture-class.adj");
}

#[test]
fn soil_texture_class_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"soil-texture-class.adj\"\n\
         ? soil_texture_class(clay, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"less_than_two_thousandths_of_a_millimeter_in_diameter\""),
        "clay means less_than_two_thousandths_of_a_millimeter_in_diameter: {out}"
    );
    assert!(
        out.contains("en.wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wikipedia citation: {out}"
    );
}

#[test]
fn soil_texture_class_reverse_binds_the_class_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"soil-texture-class.adj\"\n\
         ? soil_texture_class($C, larger_than_five_hundredths_of_a_millimeter_in_diameter)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"C\":\"sand\""),
        "the shipped larger_than_five_hundredths_of_a_millimeter_in_diameter example is sand: {out}"
    );
}

#[test]
fn soil_texture_class_abstains_honestly_on_an_untabled_class() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"soil-texture-class.adj\"\n\
         ? soil_texture_class(loam, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "loam is a real soil-texture term, but a composite mix of sand/silt/clay rather than one of the three particle-size separates -- honest abstention, never invented: {out}"
    );
}
