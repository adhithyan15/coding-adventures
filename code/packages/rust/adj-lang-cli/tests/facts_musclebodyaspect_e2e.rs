//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/muscle-body-aspect.adj`) driven through the
//! built CLI: a native `table` naming the anterior/posterior/ventral
//! spatial aspect of three named muscles, decoded from clauses already
//! sitting unused inside `muscle-groups.adj`'s own already-quoted
//! Wikipedia source sentences -- a sibling to that table. Resolves
//! binding-query recall (both directions) with the source's citation,
//! and abstains on a real, already-tabled muscle (quadriceps) whose own
//! quote uses "front of the thigh" rather than the word "anterior" --
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_musclebodyaspect_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/muscle-body-aspect.adj");
    std::fs::copy(&src, dir.join("muscle-body-aspect.adj"))
        .expect("copy shipped muscle-body-aspect.adj");
}

#[test]
fn muscle_body_aspect_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-body-aspect.adj\"\n\
         ? muscle_body_aspect(sartorius, $Aspect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"muscle_body_aspect(sartorius, anterior)\""),
        "the sartorius is on the anterior aspect: {out}"
    );
    assert!(
        out.contains("en.wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wikipedia citation: {out}"
    );
}

#[test]
fn muscle_body_aspect_recalls_backward_to_gastrocnemius() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-body-aspect.adj\"\n\
         ? muscle_body_aspect($Muscle, posterior)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"muscle_body_aspect(gastrocnemius, posterior)\""),
        "posterior recalls the gastrocnemius: {out}"
    );
}

#[test]
fn muscle_body_aspect_abstains_honestly_on_quadriceps() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-body-aspect.adj\"\n\
         ? muscle_body_aspect(quadriceps, $Aspect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "quadriceps shares the thigh region with sartorius but its own quote says 'front of the thigh', never the word 'anterior' -- honest abstention: {out}"
    );
}
