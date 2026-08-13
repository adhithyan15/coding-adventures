//! End-to-end test for the geography FACTS library
//! (`adj-facts-stdlib/geography/ocean-deepest.adj`) driven through the
//! built CLI: a native `table` naming the depth superlative the SAME NOAA
//! source sentence already states for the Pacific -- a sibling to the
//! already-shipped `oceans.adj` (which only carries each ocean's size
//! rank, not a depth claim), decoding the depth-superlative half of a
//! sentence already sitting unused inside that table's own `source` field.
//! Resolves binding-query recall (both directions) with the source's
//! citation, and abstains on an ocean (atlantic) the cited span gives no
//! depth superlative for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_oceandeepest_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geography/ocean-deepest.adj");
    std::fs::copy(&src, dir.join("ocean-deepest.adj"))
        .expect("copy shipped ocean-deepest.adj");
}

#[test]
fn ocean_is_deepest_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-deepest.adj\"\n\
         ? ocean_is_deepest(pacific, $Superlative)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"ocean_is_deepest(pacific, deepest)\""),
        "pacific is deepest: {out}"
    );
    assert!(
        out.contains("oceanservice.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA citation: {out}"
    );
}

#[test]
fn ocean_is_deepest_recalls_backward_from_a_bound_superlative() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-deepest.adj\"\n\
         ? ocean_is_deepest($Ocean, deepest)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"ocean_is_deepest(pacific, deepest)\""),
        "deepest names the pacific: {out}"
    );
}

#[test]
fn ocean_is_deepest_abstains_honestly_on_atlantic() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ocean-deepest.adj\"\n\
         ? ocean_is_deepest(atlantic, $Superlative)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "atlantic has no depth superlative in the cited span -- honest abstention: {out}"
    );
}
