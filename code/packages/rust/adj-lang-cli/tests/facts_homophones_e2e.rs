//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/homophones.adj`) driven through the built
//! CLI: a native `table` naming three common words and a homophone of each,
//! per the English Wiktionary entries for those words -- a sibling library
//! to `opposites.adj`/`synonyms.adj`. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_homophones_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/homophones.adj");
    std::fs::copy(&src, dir.join("homophones.adj")).expect("copy shipped homophones.adj");
}

#[test]
fn homophone_recall_binds_the_sound_alike_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"homophones.adj\"\n\
         ? homophone(there, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Word\":\"their\""),
        "a homophone of there is their: {out}"
    );
    assert!(
        out.contains("en.wiktionary.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Wiktionary citation: {out}"
    );
}

#[test]
fn homophone_reverse_binds_the_word_for_that_sound_alike() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"homophones.adj\"\n\
         ? homophone($Word, too)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Word\":\"to\""),
        "too's shipped homophone pairing is to: {out}"
    );
}

#[test]
fn homophone_abstains_honestly_on_an_untabled_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"homophones.adj\"\n\
         ? homophone(here, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "here is a real word with a real homophone (hear) but has no shipped homophone in this table -- honest abstention, never invented: {out}"
    );
}
