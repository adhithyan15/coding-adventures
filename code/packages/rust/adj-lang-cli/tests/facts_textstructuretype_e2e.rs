//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/text-structure-type.adj`) driven through the
//! built CLI: a native `table` naming three ways a nonfiction text
//! organizes its information, quoted verbatim from Reading Rockets'
//! "Teaching Text Structure" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_text_structure_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/text-structure-type.adj");
    std::fs::copy(&src, dir.join("text-structure-type.adj"))
        .expect("copy shipped text-structure-type.adj");
}

#[test]
fn text_structure_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"text-structure-type.adj\"\n\
         ? text_structure_type(cause_and_effect, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"tells_why_something_happened_and_what_happened\""),
        "cause_and_effect means tells_why_something_happened_and_what_happened: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn text_structure_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"text-structure-type.adj\"\n\
         ? text_structure_type($T, describes_a_topic_to_give_the_reader_a_mental_picture)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"description\""),
        "the shipped describes_a_topic_to_give_the_reader_a_mental_picture example is description: {out}"
    );
}

#[test]
fn text_structure_type_abstains_honestly_on_an_untabled_structure() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"text-structure-type.adj\"\n\
         ? text_structure_type(sequence, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "sequence is a real text structure the source names, but its sentence joins two distinct functions with 'or' rather than stating one clean fact -- honest abstention, never invented: {out}"
    );
}
