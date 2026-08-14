//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/lung-size-comparison.adj`) driven through
//! the built CLI: a native `table` naming three size/shape comparisons of
//! the right lung relative to the left, decoded from a clause already
//! sitting unused inside `lung-lobes.adj`'s own already-quoted NCI SEER
//! source sentence -- a sibling to that table. Resolves binding-query
//! recall (both directions, including a 3-answer forward recall) with
//! the source's citation, and abstains on a real, already-tabled lung
//! (left_lung) whose own quote states only a lobe count, never a
//! comparative size descriptor -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_lungsizecomparison_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/lung-size-comparison.adj");
    std::fs::copy(&src, dir.join("lung-size-comparison.adj"))
        .expect("copy shipped lung-size-comparison.adj");
}

#[test]
fn lung_size_comparison_recalls_forward_all_three_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"lung-size-comparison.adj\"\n\
         ? lung_size_comparison(right_lung, $Comparison)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    for comparison in ["shorter", "broader", "greater_volume"] {
        assert!(
            out.contains(&format!("\"term\":\"lung_size_comparison(right_lung, {comparison})\"")),
            "the right lung is {comparison} than the left lung: {out}"
        );
    }
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NCI SEER citation: {out}"
    );
}

#[test]
fn lung_size_comparison_recalls_backward_to_right_lung() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"lung-size-comparison.adj\"\n\
         ? lung_size_comparison($Lung, broader)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"lung_size_comparison(right_lung, broader)\""),
        "broader recalls the right lung: {out}"
    );
}

#[test]
fn lung_size_comparison_abstains_honestly_on_left_lung() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"lung-size-comparison.adj\"\n\
         ? lung_size_comparison(left_lung, $Comparison)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "left_lung's own quote states only a lobe count, never a comparative size descriptor -- honest abstention: {out}"
    );
}
