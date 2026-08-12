//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/part-of-speech.adj`) driven through the
//! built CLI: a native `table` naming five example words and their
//! grammatical part of speech, per Grammarly's "The 8 Parts of Speech"
//! article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_partofspeech_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/part-of-speech.adj");
    std::fs::copy(&src, dir.join("part-of-speech.adj")).expect("copy shipped part-of-speech.adj");
}

#[test]
fn part_of_speech_recall_binds_the_category_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"part-of-speech.adj\"\n\
         ? part_of_speech(queen, $Category)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Category\":\"noun\""),
        "queen is a noun: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn part_of_speech_reverse_binds_the_word_for_that_category() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"part-of-speech.adj\"\n\
         ? part_of_speech($Word, verb)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Word\":\"run\""),
        "the shipped verb example is run: {out}"
    );
}

#[test]
fn part_of_speech_recall_binds_a_newly_added_row_directly() {
    let dir = scratch("direct_new");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"part-of-speech.adj\"\n\
         ? part_of_speech(quietly, $Category)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Category\":\"adverb\""),
        "quietly is an adverb: {out}"
    );
}

#[test]
fn part_of_speech_reverse_binds_a_newly_added_row() {
    let dir = scratch("reverse_new");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"part-of-speech.adj\"\n\
         ? part_of_speech($Word, preposition)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Word\":\"against\""),
        "the shipped preposition example is against: {out}"
    );
}

#[test]
fn part_of_speech_abstains_honestly_on_an_untabled_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"part-of-speech.adj\"\n\
         ? part_of_speech(slowly, $Category)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "slowly is a real word (an adverb) but has no shipped category in this table -- honest abstention, never invented: {out}"
    );
}
