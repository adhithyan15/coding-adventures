//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/joint-formed-by.adj`) driven through the
//! built CLI: a native `table` naming the actual bones that meet to form
//! three synovial-joint types, decoded from spans already sitting unused
//! inside `joint-types.adj`'s own already-quoted StatPearls sentences --
//! a sibling to that table. Resolves binding-query recall (both
//! directions, including a 2-answer forward recall), and abstains on a
//! real, already-tabled joint type (hinge) whose own quote names only an
//! example joint, never its forming bones -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_jointformedby_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/joint-formed-by.adj");
    std::fs::copy(&src, dir.join("joint-formed-by.adj")).expect("copy shipped joint-formed-by.adj");
}

#[test]
fn joint_formed_by_recalls_forward_both_pivot_bones_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"joint-formed-by.adj\"\n\
         ? joint_formed_by(pivot, $Bone)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    for bone in ["atlas", "axis"] {
        assert!(
            out.contains(&format!("\"term\":\"joint_formed_by(pivot, {bone})\"")),
            "the pivot joint is formed by {bone}: {out}"
        );
    }
    assert!(
        out.contains("ncbi.nlm.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the StatPearls citation: {out}"
    );
}

#[test]
fn joint_formed_by_recalls_backward_to_saddle() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"joint-formed-by.adj\"\n\
         ? joint_formed_by($Type, trapezium)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"joint_formed_by(saddle, trapezium)\""),
        "trapezium helps form the saddle joint: {out}"
    );
}

#[test]
fn joint_formed_by_abstains_honestly_on_hinge() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"joint-formed-by.adj\"\n\
         ? joint_formed_by(hinge, $Bone)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "hinge is a real, already-tabled joint type but its own quote names no forming bones -- honest abstention: {out}"
    );
}
