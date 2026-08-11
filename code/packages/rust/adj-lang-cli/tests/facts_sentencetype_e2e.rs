//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/sentence-type.adj`) driven through the built
//! CLI: a native `table` naming four example sentences and their
//! grammatical type, per Grammarly's "4 Types of Sentences" article. 0
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
    let dir = std::env::temp_dir().join(format!("adjcli_sentencetype_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/sentence-type.adj");
    std::fs::copy(&src, dir.join("sentence-type.adj")).expect("copy shipped sentence-type.adj");
}

#[test]
fn sentence_type_recall_binds_the_type_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sentence-type.adj\"\n\
         ? sentence_type(bears_dont_eat_when_they_hibernate, $Type)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Type\":\"declarative\""),
        "the bears sentence is declarative: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn sentence_type_reverse_binds_the_example_for_that_type() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sentence-type.adj\"\n\
         ? sentence_type($Example, imperative)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Example\":\"sweep_the_floor_before_you_mop_it\""),
        "the shipped imperative example is sweep_the_floor_before_you_mop_it: {out}"
    );
}

#[test]
fn sentence_type_abstains_honestly_on_an_untabled_sentence() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sentence-type.adj\"\n\
         ? sentence_type(the_cat_sat_on_the_mat, $Type)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the_cat_sat_on_the_mat is a real declarative sentence but has no shipped type in this table -- honest abstention, never invented: {out}"
    );
}
