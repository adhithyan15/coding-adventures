//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/author-purpose.adj`) driven through the
//! built CLI: a native `table` naming the three classic reasons an author
//! writes something, quoted verbatim from LiteracyIdeas' "The Author's
//! Purpose: Ultimate Guide for Teachers and Students" article. 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_author_purpose_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/author-purpose.adj");
    std::fs::copy(&src, dir.join("author-purpose.adj")).expect("copy shipped author-purpose.adj");
}

#[test]
fn author_purpose_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"author-purpose.adj\"\n\
         ? author_purpose(persuade, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"convince_the_reader_of_the_merits_of_a_particular_point_of_view\""),
        "persuade means convince_the_reader_of_the_merits_of_a_particular_point_of_view: {out}"
    );
    assert!(
        out.contains("literacyideas.com") && out.contains("\"trust\":\"consensus\""),
        "carries the LiteracyIdeas citation: {out}"
    );
}

#[test]
fn author_purpose_reverse_binds_the_purpose_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"author-purpose.adj\"\n\
         ? author_purpose($P, enlighten_the_readership_about_a_real_world_topic)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"P\":\"inform\""),
        "the shipped enlighten_the_readership_about_a_real_world_topic example is inform: {out}"
    );
}

#[test]
fn author_purpose_abstains_honestly_on_an_untabled_purpose() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"author-purpose.adj\"\n\
         ? author_purpose(describe, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "describe is a real purpose the source names, but its sentence uses a different pattern than the three tabled here -- honest abstention, never invented: {out}"
    );
}
