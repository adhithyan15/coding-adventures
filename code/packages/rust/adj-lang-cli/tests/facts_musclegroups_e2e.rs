//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/muscle-groups.adj`) driven through the built CLI:
//! a native `table` of skeletal muscle → body region resolves a binding-query
//! recall with the source's Wikipedia (consensus) citation, runs the relation
//! backward (region → every muscle in it, one-to-many), and abstains on a
//! non-muscle (the femur, a bone) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsmuscle_{tag}_{}", std::process::id()));
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
fn anatomy_muscle_groups_recall_binds_region_with_citation() {
    let dir = scratch("musclegroups");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/muscle-groups.adj");
    std::fs::copy(&src, dir.join("muscle-groups.adj")).expect("copy shipped muscle-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-groups.adj\"\n\
         ? muscle_region(biceps_brachii, $R)\n\
         ? muscle_region(deltoid, $R)\n\
         ? muscle_region(quadriceps, $R)\n\
         ? muscle_region($M, arm)\n\
         ? muscle_region(femur, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Each named muscle binds its body region — a single lowercase token echoed
    // verbatim from the source sentence.
    assert!(out.contains("\"R\":\"arm\""), "biceps_brachii → arm: {out}");
    assert!(out.contains("\"R\":\"shoulder\""), "deltoid → shoulder: {out}");
    assert!(out.contains("\"R\":\"thigh\""), "quadriceps → thigh: {out}");
    // The relation runs backward and is one-to-many: the region `arm` recalls
    // BOTH the biceps and the triceps.
    assert!(
        out.contains("\"M\":\"biceps_brachii\"") && out.contains("\"M\":\"triceps_brachii\""),
        "arm → biceps_brachii AND triceps_brachii (reverse recall): {out}"
    );
    // The answer carries the Wikipedia citation as its proof, at consensus trust.
    assert!(
        out.contains("en.wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // The femur is a bone, not a muscle — honest abstention, never a fabricated
    // location.
    assert!(out.contains("\"abstained\":true"), "unknown muscle abstains: {out}");
}
