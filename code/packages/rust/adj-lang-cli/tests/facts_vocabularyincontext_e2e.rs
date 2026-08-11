//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/vocabulary-in-context.adj`) driven through
//! the built CLI: a native `table` naming three vocabulary words whose
//! meaning Reading Rockets' "Using Context Clues to Understand Word
//! Meanings" article teaches via a worked example sentence. The TENTH
//! literacy sub-skill library in this loop's sweep. 0 answer-time model
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
    let dir = std::env::temp_dir().join(format!("adjcli_vocabularyincontext_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/vocabulary-in-context.adj");
    std::fs::copy(&src, dir.join("vocabulary-in-context.adj"))
        .expect("copy shipped vocabulary-in-context.adj");
}

#[test]
fn vocabulary_in_context_recall_binds_the_meaning_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vocabulary-in-context.adj\"\n\
         ? vocabulary_in_context(ornithology, $Meaning)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Meaning\":\"scientific_study_of_birds\""),
        "ornithology is the scientific study of birds: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn vocabulary_in_context_reverse_binds_the_word_for_that_meaning() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vocabulary-in-context.adj\"\n\
         ? vocabulary_in_context($W, hidden_or_not_easily_seen)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"W\":\"inconspicuous\""),
        "inconspicuous means hidden or not easily seen: {out}"
    );
}

#[test]
fn vocabulary_in_context_abstains_honestly_on_an_undefined_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"vocabulary-in-context.adj\"\n\
         ? vocabulary_in_context(arboreal, $Meaning)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "arboreal is a real vocabulary word but not one this source defines -- honest abstention, never invented: {out}"
    );
}
