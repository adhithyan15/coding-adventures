//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/determiner-type.adj`) driven through the
//! built CLI: a native `table` naming three determiner types and what
//! each actually does, quoted verbatim from Grammarly's "What Are
//! Determiners?" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_determiner_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/determiner-type.adj");
    std::fs::copy(&src, dir.join("determiner-type.adj")).expect("copy shipped determiner-type.adj");
}

#[test]
fn determiner_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"determiner-type.adj\"\n\
         ? determiner_type(distributive_determiner, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"refers_to_a_group_or_individual_parts_within_a_group\""),
        "distributive_determiner means refers_to_a_group_or_individual_parts_within_a_group: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn determiner_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"determiner-type.adj\"\n\
         ? determiner_type($T, communicates_the_placement_of_a_noun_in_space_or_time)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"demonstrative_determiner\""),
        "the shipped communicates_the_placement_of_a_noun_in_space_or_time example is demonstrative_determiner: {out}"
    );
}

#[test]
fn determiner_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"determiner-type.adj\"\n\
         ? determiner_type(possessive_determiner, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "possessive_determiner is a real category the source covers but its sentence bundles two facts plus an example list, not one of the three clean single-fact types tabled here -- honest abstention, never invented: {out}"
    );
}
