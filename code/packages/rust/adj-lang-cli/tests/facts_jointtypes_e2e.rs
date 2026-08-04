//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/joint-types.adj`) driven through the built CLI:
//! a native `table` of synovial joint type → representative example resolves a
//! binding-query recall with the source's StatPearls (NIH/NLM) citation, runs
//! the relation backward (joint type → examples, and example → joint type), and
//! abstains on a non-synovial joint (a skull suture) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsjoint_{tag}_{}", std::process::id()));
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
fn anatomy_joint_types_recall_binds_example_with_citation() {
    let dir = scratch("jointtypes");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/joint-types.adj");
    std::fs::copy(&src, dir.join("joint-types.adj")).expect("copy shipped joint-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"joint-types.adj\"\n\
         ? joint_example(hinge, $Ex)\n\
         ? joint_example(saddle, $Ex)\n\
         ? joint_example(ball_and_socket, $Ex)\n\
         ? joint_example($T, elbow)\n\
         ? joint_example(suture, $Ex)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The stock hinge is the elbow; the saddle joint is at the base of the thumb.
    assert!(out.contains("\"Ex\":\"elbow\""), "hinge → elbow: {out}");
    assert!(out.contains("\"Ex\":\"thumb\""), "saddle → thumb: {out}");
    // ball_and_socket is one-to-many: the source names both the hip and shoulder.
    assert!(out.contains("\"Ex\":\"hip\""), "ball_and_socket → hip: {out}");
    assert!(
        out.contains("\"Ex\":\"shoulder\""),
        "ball_and_socket → shoulder: {out}"
    );
    // The relation runs backward: the elbow recalls the hinge shape.
    assert!(
        out.contains("\"T\":\"hinge\""),
        "elbow → hinge (reverse recall): {out}"
    );
    // The answer carries the StatPearls (NIH/NLM) citation as its proof.
    assert!(
        out.contains("ncbi.nlm.nih.gov/books/NBK507893")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // A skull suture is an immovable joint, not a synovial type — honest
    // abstention, never a fabricated example.
    assert!(
        out.contains("\"abstained\":true"),
        "unknown joint type abstains: {out}"
    );
}
