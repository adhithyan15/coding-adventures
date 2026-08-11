//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/animal-habitat.adj`) driven through the built
//! CLI: a native `table` naming three animals and the biome each lives in,
//! per National Geographic -- a sibling library to `animal-homes.adj` (built
//! structures) covering a genuinely different axis (biome/environment type).
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
    let dir = std::env::temp_dir().join(format!("adjcli_animalhabitat_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/animal-habitat.adj");
    std::fs::copy(&src, dir.join("animal-habitat.adj")).expect("copy shipped animal-habitat.adj");
}

#[test]
fn animal_habitat_recall_binds_the_biome_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-habitat.adj\"\n\
         ? animal_habitat(polar_bear, $Biome)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Biome\":\"arctic\""),
        "a polar bear's habitat is the arctic: {out}"
    );
    assert!(
        out.contains("kids.nationalgeographic.com") && out.contains("\"trust\":\"consensus\""),
        "carries the National Geographic citation: {out}"
    );
}

#[test]
fn animal_habitat_reverse_binds_the_animal_for_that_biome() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-habitat.adj\"\n\
         ? animal_habitat($Animal, desert)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Animal\":\"bactrian_camel\""),
        "the desert-dwelling animal shipped is the bactrian camel: {out}"
    );
}

#[test]
fn animal_habitat_abstains_honestly_on_an_untabled_animal() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"animal-habitat.adj\"\n\
         ? animal_habitat(dog, $Biome)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "dog is a real animal but has no shipped habitat in this table -- honest abstention, never invented: {out}"
    );
}
