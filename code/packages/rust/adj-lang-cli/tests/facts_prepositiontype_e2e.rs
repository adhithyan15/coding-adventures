//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/preposition-type.adj`) driven through the
//! built CLI: a native `table` naming three preposition types and what
//! each actually shows, quoted verbatim from Grammarly's "Prepositions:
//! Definition, Types, and Examples" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_preposition_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/preposition-type.adj");
    std::fs::copy(&src, dir.join("preposition-type.adj")).expect("copy shipped preposition-type.adj");
}

#[test]
fn preposition_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"preposition-type.adj\"\n\
         ? preposition_type(preposition_of_time, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"shows_when_something_happened_or_will_happen\""),
        "preposition_of_time means shows_when_something_happened_or_will_happen: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn preposition_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"preposition-type.adj\"\n\
         ? preposition_type($T, shows_how_something_is_moving_or_which_way_its_going)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"preposition_of_direction\""),
        "the shipped shows_how_something_is_moving_or_which_way_its_going example is preposition_of_direction: {out}"
    );
}

#[test]
fn preposition_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"preposition-type.adj\"\n\
         ? preposition_type(preposition_of_manner_cause_or_purpose, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "preposition_of_manner_cause_or_purpose is a real category the source covers but bundles three distinct functions, not one of the three clean single-concept types tabled here -- honest abstention, never invented: {out}"
    );
}
