//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/end-punctuation-mark.adj`) driven through
//! the built CLI: a native `table` naming three end-punctuation marks and
//! what each actually does, quoted verbatim from Grammarly's "Punctuation:
//! The Best Guide to Using Punctuation Marks" article. 0 answer-time model
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
    let dir = std::env::temp_dir().join(format!("adjcli_end_punctuation_mark_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/end-punctuation-mark.adj");
    std::fs::copy(&src, dir.join("end-punctuation-mark.adj")).expect("copy shipped end-punctuation-mark.adj");
}

#[test]
fn end_punctuation_mark_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"end-punctuation-mark.adj\"\n\
         ? end_punctuation_mark(period, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"ends_a_declarative_sentence\""),
        "period means ends_a_declarative_sentence: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn end_punctuation_mark_reverse_binds_the_mark_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"end-punctuation-mark.adj\"\n\
         ? end_punctuation_mark($M, makes_sentences_exciting)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"M\":\"exclamation_point\""),
        "the shipped makes_sentences_exciting example is exclamation_point: {out}"
    );
}

#[test]
fn end_punctuation_mark_abstains_honestly_on_an_untabled_mark() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"end-punctuation-mark.adj\"\n\
         ? end_punctuation_mark(comma, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "comma is a real mark the source covers but belongs to a different category (mid-sentence pause, not end-of-sentence), not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
