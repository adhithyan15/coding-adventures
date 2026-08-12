//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/point-of-view.adj`) driven through the built
//! CLI: a native `table` naming the three narrative perspectives a story
//! can be told from, quoted verbatim from Grammarly's "What Is Point of
//! View in Writing, and How Does It Work?" article. 0 answer-time model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_point_of_view_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/point-of-view.adj");
    std::fs::copy(&src, dir.join("point-of-view.adj")).expect("copy shipped point-of-view.adj");
}

#[test]
fn point_of_view_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"point-of-view.adj\"\n\
         ? point_of_view(first_person, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"the_reader_accesses_the_story_through_one_person\""),
        "first_person means the_reader_accesses_the_story_through_one_person: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn point_of_view_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"point-of-view.adj\"\n\
         ? point_of_view($T, uses_the_pronoun_you)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"second_person\""),
        "the shipped uses_the_pronoun_you example is second_person: {out}"
    );
}

#[test]
fn point_of_view_abstains_honestly_on_an_untabled_subtype() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"point-of-view.adj\"\n\
         ? point_of_view(third_person_omniscient, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "third_person_omniscient is a real subtype the same source defines, but not a fourth peer point of view -- honest abstention, never invented: {out}"
    );
}
