//! End-to-end test for the geometry FACTS library
//! (`adj-facts-stdlib/geometry/angle-pairs.adj`) driven through the built CLI:
//! a native `table` of pair-relationship → defining condition resolves a binding
//! query recall with the source's OpenStax / LibreTexts citation, runs the
//! relation backward (condition → relationship), and abstains on `acute` — a
//! SINGLE-angle type, not a pair relationship — with 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsanglepairs_{tag}_{}", std::process::id()));
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
fn geometry_angle_pairs_recall_binds_condition_with_citation_and_abstains_on_single_angle_type() {
    let dir = scratch("anglepairs");
    // Copy the shipped geometry table beside the entry program and import it.
    let src = facts_stdlib().join("geometry/angle-pairs.adj");
    std::fs::copy(&src, dir.join("angle-pairs.adj")).expect("copy shipped angle-pairs.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"angle-pairs.adj\"\n\
         ? angle_pair(complementary, $Cond)\n\
         ? angle_pair(supplementary, $Cond)\n\
         ? angle_pair(vertical, $Cond)\n\
         ? angle_pair(adjacent, $Cond)\n\
         ? angle_pair($Rel, sum_to_180)\n\
         ? angle_pair(acute, $Cond)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Each named pair relationship recalls the source's defining condition.
    assert!(out.contains("\"Cond\":\"sum_to_90\""), "complementary → sum_to_90: {out}");
    assert!(out.contains("\"Cond\":\"sum_to_180\""), "supplementary → sum_to_180: {out}");
    assert!(out.contains("\"Cond\":\"equal\""), "vertical → equal: {out}");
    assert!(
        out.contains("\"Cond\":\"share_side_and_vertex\""),
        "adjacent → share_side_and_vertex: {out}"
    );
    // The relation runs backward: the sum_to_180 condition recalls supplementary.
    assert!(
        out.contains("\"Rel\":\"supplementary\""),
        "sum_to_180 → supplementary (reverse recall): {out}"
    );
    // The complementary answer carries the OpenStax / LibreTexts citation + trust.
    assert!(
        out.contains("libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation with trust tier: {out}"
    );
    // `acute` is a single-angle TYPE (geometry/angle-types), NOT a pair
    // relationship — honest abstention, never a fabricated condition.
    assert!(
        out.contains("\"abstained\":true"),
        "acute (a single-angle type, not a pair relationship) abstains: {out}"
    );
}
