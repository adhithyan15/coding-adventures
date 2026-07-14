//! End-to-end test for the geography oceans FACTS library
//! (`adj-facts-stdlib/geography/oceans.adj`): a native `table` of
//! ocean → size-rank resolves forward AND reverse binding queries with the NOAA
//! citation, and abstains on something that is not an ocean — 0 answer-time
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
fn geography_oceans_recall_binds_rank_forward_and_reverse() {
    let dir = scratch("oceans");
    let src = facts_stdlib().join("geography/oceans.adj");
    std::fs::copy(&src, dir.join("oceans.adj")).expect("copy shipped oceans.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"oceans.adj\"\n\
         ? ocean_size_rank(pacific, $R)\n\
         ? ocean_size_rank($Ocean, 2)\n\
         ? ocean_size_rank(mediterranean, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward: the Pacific is the largest ocean basin, so rank 1.
    assert!(out.contains("\"R\":\"1\""), "pacific → 1: {out}");
    // Reverse: the second largest basin is the Atlantic (binds the other column).
    assert!(out.contains("\"Ocean\":\"atlantic\""), "rank 2 → atlantic: {out}");
    // The answer carries the NOAA citation as its proof.
    assert!(
        out.contains("oceanservice.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA citation: {out}"
    );
    // The Mediterranean is a sea, not one of the five oceans — honest abstention.
    assert!(out.contains("\"abstained\":true"), "mediterranean abstains: {out}");
}
