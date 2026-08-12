//! End-to-end test for the geography FACTS library
//! (`adj-facts-stdlib/geography/map-type.adj`) driven through the built
//! CLI: a native `table` naming three types of map and what each actually
//! shows, quoted verbatim from Geology.com's "Types of Maps" article. 0
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
    let dir = std::env::temp_dir().join(format!("adjcli_map_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geography/map-type.adj");
    std::fs::copy(&src, dir.join("map-type.adj")).expect("copy shipped map-type.adj");
}

#[test]
fn map_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"map-type.adj\"\n\
         ? map_type(physical, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"shows_the_natural_landscape_features_of_earth\""),
        "physical means shows_the_natural_landscape_features_of_earth: {out}"
    );
    assert!(
        out.contains("geology.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Geology.com citation: {out}"
    );
}

#[test]
fn map_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"map-type.adj\"\n\
         ? map_type($T, shows_the_shape_of_earths_surface)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"topographic\""),
        "the shipped shows_the_shape_of_earths_surface example is topographic: {out}"
    );
}

#[test]
fn map_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"map-type.adj\"\n\
         ? map_type(weather, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "weather maps are a real category the source covers, but its own section never states a single complete defining sentence, not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
