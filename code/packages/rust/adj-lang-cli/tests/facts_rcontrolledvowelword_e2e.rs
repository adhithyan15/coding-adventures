//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/r-controlled-vowel-word.adj`) driven
//! through the built CLI: a native `table` naming which r-controlled
//! vowel digraph appears in each of five example words, quoted verbatim
//! from the University of Florida Literacy Institute (UFLI)'s phonics
//! foundations toolbox. The THIRD literacy slice in this loop's sweep to
//! move beyond CCSS RF.K.2 (following `compound-word-spelling-example.adj`
//! and `silent-e-word.adj`'s precedent). 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rcontrolledvowelword_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/r-controlled-vowel-word.adj");
    std::fs::copy(&src, dir.join("r-controlled-vowel-word.adj"))
        .expect("copy shipped r-controlled-vowel-word.adj");
}

#[test]
fn r_controlled_vowel_word_recall_binds_the_pattern_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"r-controlled-vowel-word.adj\"\n\
         ? r_controlled_vowel_word(barn, $Pattern)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Pattern\":\"ar\""),
        "barn follows the ar r-controlled pattern: {out}"
    );
    assert!(
        out.contains("ufl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the UFLI citation: {out}"
    );
}

#[test]
fn r_controlled_vowel_word_reverse_binds_the_word_for_that_pattern() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"r-controlled-vowel-word.adj\"\n\
         ? r_controlled_vowel_word($W, er)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"W\":\"fern\""),
        "fern follows the er r-controlled pattern: {out}"
    );
}

#[test]
fn r_controlled_vowel_word_abstains_honestly_on_an_uncited_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"r-controlled-vowel-word.adj\"\n\
         ? r_controlled_vowel_word(star, $Pattern)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "star is a real ar-pattern word but not one this source names -- honest abstention, never invented: {out}"
    );
}
