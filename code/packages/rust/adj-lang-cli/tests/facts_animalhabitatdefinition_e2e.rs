//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/animal-habitat-definition.adj`) driven through
//! the built CLI: a `rule` composing the already-shipped `animal_habitat`
//! table (`biology/animal-habitat.adj`) with the already-shipped `biome_type`
//! table (`biology/biome-type.adj`, a SAME-DIRECTORY import, the same shape
//! `heat-causes-phase-change.adj`/`force-causes-acceleration.adj` already
//! established) to DERIVE `animal_habitat_definition($Animal, $Description)`
//! -- the FIFTH `rule`-based CAUSAL-COMPOSITION fact in this loop's science
//! curriculum sweep, mirroring the discipline `heat-causes-phase-change.adj`,
//! `force-causes-acceleration.adj`, `earth-layer-matter-behavior.adj`, and
//! `measuring-tool-si-unit.adj` already established, applied here to the
//! "ecosystems" gap (ADJ-STDLIB-COVERAGE.md 5.1/5.2). 0 answer-time model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_animalhabitatdefinition_{tag}_{}", std::process::id()));
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

/// Copy all THREE shipped files, preserving their real relative directory
/// structure: `animal-habitat-definition.adj` (in `biology/`) imports
/// `animal-habitat.adj` and `biome-type.adj` (both same dir).
fn place_libs(dir: &Path) {
    let src = facts_stdlib();
    for rel in [
        "biology/animal-habitat.adj",
        "biology/biome-type.adj",
        "biology/animal-habitat-definition.adj",
    ] {
        let dst = dir.join(rel);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(src.join(rel), &dst).unwrap_or_else(|e| panic!("copy shipped {rel}: {e}"));
    }
}

#[test]
fn bactrian_camel_derives_desert_definition_with_dual_citations() {
    let dir = scratch("camel");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/animal-habitat-definition.adj\"\n\
         ? animal_habitat_definition(bactrian_camel, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains(
            "\"D\":\"dry_areas_where_rainfall_is_less_than_50_centimeters_20_inches_per_year\""
        ),
        "a bactrian camel's desert habitat is a dry area with less than 50cm/20in of rain per year: {out}"
    );
    // The derivation composes citations from BOTH sibling libraries:
    // National Geographic Kids (animal_habitat) AND National Geographic
    // Education (biome_type).
    assert!(
        out.contains("\"kind\":\"rule\"") && out.contains("\"kind\":\"fact\""),
        "the derived fact is DERIVED, not a direct table row -- both a rule step and fact steps appear: {out}"
    );
    assert!(
        out.contains("kids.nationalgeographic.com") && out.contains("education.nationalgeographic.org"),
        "carries citations from BOTH composed libraries (animal-habitat.adj and biome-type.adj): {out}"
    );
}

#[test]
fn grassland_definition_reverse_binds_to_giraffe() {
    let dir = scratch("reverse");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/animal-habitat-definition.adj\"\n\
         ? animal_habitat_definition($A, open_regions_dominated_by_grass_with_a_warm_dry_climate)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"A\":\"giraffe\""),
        "the only animal whose habitat is the grassland biome is the giraffe: {out}"
    );
}

#[test]
fn polar_bear_abstains_honestly_as_arctic_is_not_a_keyed_biome() {
    let dir = scratch("abstain");
    place_libs(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"biology/animal-habitat-definition.adj\"\n\
         ? animal_habitat_definition(polar_bear, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the polar bear's own habitat word is \"arctic\", not one of biome_type's four keyed biomes (desert/forest/grassland/tundra) -- honest abstention, never invented: {out}"
    );
}
