//! End-to-end test for the geography continents FACTS library
//! (`adj-facts-stdlib/geography/continents.adj`): a native `table` of
//! continent → size-rank resolves forward AND reverse binding queries with the
//! National Geographic citation, and abstains on something that is not a
//! continent — 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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

#[test]
fn geography_continents_recall_binds_rank_forward_and_reverse() {
    let dir = scratch("continents");
    let src = facts_stdlib().join("geography/continents.adj");
    std::fs::copy(&src, dir.join("continents.adj")).expect("copy shipped continents.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"continents.adj\"\n\
         ? continent_size_rank(asia, $R)\n\
         ? continent_size_rank($Continent, 3)\n\
         ? continent_size_rank(greenland, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward: Asia is the largest continent, so rank 1.
    assert!(out.contains("\"R\":\"1\""), "asia → 1: {out}");
    // Reverse: the third largest continent is North America (binds the other column).
    assert!(
        out.contains("\"Continent\":\"north_america\""),
        "rank 3 → north_america: {out}"
    );
    // The answer carries the National Geographic citation as its proof.
    assert!(
        out.contains("education.nationalgeographic.org") && out.contains("\"trust\":\"consensus\""),
        "carries the National Geographic citation: {out}"
    );
    // Greenland is the world's largest island, not a continent — honest abstention.
    assert!(out.contains("\"abstained\":true"), "greenland abstains: {out}");
}
