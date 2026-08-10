//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/syllable-count.adj`) driven through the built
//! CLI: a native `table` naming how many syllables each of four words has,
//! quoted verbatim from a Reading Rockets classroom-technique demonstration
//! -- the SECOND literacy sub-skill in this loop's curriculum sweep,
//! deliberately different in shape from `word-families.adj`'s rhyme-family
//! DERIVATION (this is a genuinely new phonological-awareness skill,
//! syllable segmentation RF.K.2.b, not another word family RF.K.2.a). 0
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
    let dir = std::env::temp_dir().join(format!("adjcli_syllablecount_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/syllable-count.adj");
    std::fs::copy(&src, dir.join("syllable-count.adj"))
        .expect("copy shipped syllable-count.adj");
}

#[test]
fn syllable_count_recall_binds_the_count_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-count.adj\"\n\
         ? syllable_count(peanut, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"N\":\"2\""), "peanut has 2 syllables: {out}");
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn syllable_count_reverse_binds_every_word_with_that_count() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-count.adj\"\n\
         ? syllable_count($W, 2)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for w in ["peanut", "pencil", "sunset", "laptop"] {
        assert!(
            out.contains(&format!("\"W\":\"{w}\"")),
            "{w} has 2 syllables: {out}"
        );
    }
}

#[test]
fn syllable_count_abstains_honestly_on_an_unshipped_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-count.adj\"\n\
         ? syllable_count(banana, $N)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"banana\" has no shipped row -- honest abstention, never invented: {out}"
    );
}
