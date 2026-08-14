//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/subordinating-conjunction-relationship-type.adj`)
//! driven through the built CLI: a native `table` naming each relationship
//! kind a subordinating conjunction can signal, decoded from a span already
//! sitting unused inside the SAME Grammarly quote `conjunction-type.adj`'s
//! own header already reproduces -- a sibling to that table. Resolves
//! binding-query recall (both directions, including a 4-answer backward
//! recall) with the source's citation, and abstains both on a relationship
//! type the cited sentence does not name and on a real, already-tabled but
//! different conjunction category -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "adjcli_subordinatingconjunctionrelationshiptype_{tag}_{}",
        std::process::id()
    ));
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
    let src = facts_stdlib().join("language/subordinating-conjunction-relationship-type.adj");
    std::fs::copy(&src, dir.join("subordinating-conjunction-relationship-type.adj"))
        .expect("copy shipped subordinating-conjunction-relationship-type.adj");
}

#[test]
fn subordinating_conjunction_relationship_type_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"subordinating-conjunction-relationship-type.adj\"\n\
         ? subordinating_conjunction_relationship_type(cause_and_effect, $ConjunctionType)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains(
            "\"term\":\"subordinating_conjunction_relationship_type(cause_and_effect, subordinating_conjunction)\""
        ),
        "cause_and_effect is signaled by a subordinating conjunction: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn subordinating_conjunction_relationship_type_recalls_backward_all_four() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"subordinating-conjunction-relationship-type.adj\"\n\
         ? subordinating_conjunction_relationship_type($RelationshipType, subordinating_conjunction)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for relationship in ["cause_and_effect", "comparison", "contrast", "time"] {
        assert!(
            out.contains(&format!(
                "\"term\":\"subordinating_conjunction_relationship_type({relationship}, subordinating_conjunction)\""
            )),
            "backward recall should include {relationship}: {out}"
        );
    }
}

#[test]
fn subordinating_conjunction_relationship_type_abstains_honestly_on_unnamed_relationship() {
    let dir = scratch("abstain_relationship");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"subordinating-conjunction-relationship-type.adj\"\n\
         ? subordinating_conjunction_relationship_type(concession, $ConjunctionType)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "concession is a real relationship subordinating conjunctions can express, but not one this cited sentence names -- honest abstention: {out}"
    );
}

#[test]
fn subordinating_conjunction_relationship_type_abstains_honestly_on_coordinating_conjunction() {
    let dir = scratch("abstain_conjunction");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"subordinating-conjunction-relationship-type.adj\"\n\
         ? subordinating_conjunction_relationship_type($RelationshipType, coordinating_conjunction)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "coordinating_conjunction is a real, already-tabled conjunction category, but not the one this table covers -- honest abstention: {out}"
    );
}
