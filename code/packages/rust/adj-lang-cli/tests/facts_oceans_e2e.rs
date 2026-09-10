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
    // The answer carries the NOAA citation as its proof — pinned as TEXT, not
    // by host and trust tier alone.
    //
    // This assertion used to be `contains("oceanservice.noaa.gov") &&
    // contains(trust)`, which says nothing about what the `source` field
    // CONTAINS. It was satisfied by the welded two-sentence string installment
    // 4l removed — a string that occurs nowhere on the page, because its two
    // sentences sit 805 characters apart with an image caption between them —
    // exactly as happily as by the real spans that replaced it.
    //
    // The needle is the whole citation object as the serialiser actually emits
    // it, taken from a real run rather than from memory of the format, and it
    // CLOSES on the corroborations `]`. That bounds four things at once: the
    // source text, the locator, the trust tier, and the corroboration SET — so
    // a fabricated `cites` cannot be appended without reddening (#14735).
    //
    // Its string appears FOUR times in this test's output: the serialiser
    // echoes each citation in `citations` and in `steps` — two sections per
    // answer — and the program above issues three queries of which two bind.
    // That was checked BEFORE trusting the needle: none of the four is an
    // independent copy, because mutating the `.adj` moves all four together.
    // Where a duplicate CAN drift — a rule inlining a copy of a composed
    // library's citation — a needle constrains the copy and not the library,
    // which is #14745.
    assert!(
        out.contains(
            "\"source\":\"The Pacific Ocean is the largest and deepest of the world ocean basins.\",\"locator\":\"https://oceanservice.noaa.gov/facts/biggestocean.html\",\"trust\":\"authoritative\",\"corroborations\":[{\"source\":\"The Atlantic basin is the second largest basin, followed by the Indian Ocean basin, the Southern Ocean, and finally the Arctic Ocean basin.\",\"locator\":\"https://oceanservice.noaa.gov/facts/biggestocean.html\"}]"
        ),
        "carries the NOAA citation, its corroboration, and nothing else: {out}"
    );
    // The Mediterranean is a sea, not one of the five oceans — honest abstention.
    assert!(out.contains("\"abstained\":true"), "mediterranean abstains: {out}");
}
