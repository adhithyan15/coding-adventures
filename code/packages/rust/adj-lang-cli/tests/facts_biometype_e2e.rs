//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/biome-type.adj`) driven through the built
//! CLI: a native `table` naming four major biomes and what defines each,
//! quoted verbatim from National Geographic Education's "The Five Major
//! Types of Biomes" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_biome_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/biome-type.adj");
    std::fs::copy(&src, dir.join("biome-type.adj")).expect("copy shipped biome-type.adj");
}

#[test]
fn biome_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biome-type.adj\"\n\
         ? biome_type(desert, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"dry_areas_where_rainfall_is_less_than_50_centimeters_20_inches_per_year\""),
        "desert means dry_areas_where_rainfall_is_less_than_50_centimeters_20_inches_per_year: {out}"
    );
    assert!(
        out.contains("nationalgeographic.org") && out.contains("\"trust\":\"consensus\""),
        "carries the National Geographic Education citation: {out}"
    );
}

#[test]
fn biome_type_reverse_binds_the_biome_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biome-type.adj\"\n\
         ? biome_type($B, open_regions_dominated_by_grass_with_a_warm_dry_climate)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"B\":\"grassland\""),
        "the shipped open_regions_dominated_by_grass_with_a_warm_dry_climate example is grassland: {out}"
    );
}

#[test]
fn biome_type_abstains_honestly_on_an_untabled_biome() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biome-type.adj\"\n\
         ? biome_type(aquatic, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "aquatic is the source's fifth major biome, but its own section defers to freshwater/marine sub-categories rather than stating a single clean defining sentence, unlike the four tabled here -- honest abstention, never invented: {out}"
    );
}
