//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/syllable-substitution.adj`) driven through the
//! built CLI: a native THREE-column `table` naming the one syllable
//! substitution ("suntan" -> "sunset", second syllable) walked through on
//! Reading Rockets' "Phonological and Phonemic Awareness: In Practice"
//! module -- the SEVENTH literacy sub-skill in this loop's curriculum sweep,
//! deliberately different from `phoneme-substitution.adj` (a single SOUND
//! swap) and `phoneme-deletion.adj` (a single sound removal): this grounds
//! substituting a whole SYLLABLE. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_syllablesub_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/syllable-substitution.adj");
    std::fs::copy(&src, dir.join("syllable-substitution.adj"))
        .expect("copy shipped syllable-substitution.adj");
}

#[test]
fn syllable_substitution_recall_binds_the_position_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-substitution.adj\"\n\
         ? syllable_substitution(suntan, sunset, $Position)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Position\":\"second\""),
        "suntan -> sunset changes the second syllable: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn syllable_substitution_reverse_binds_the_word_pair() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-substitution.adj\"\n\
         ? syllable_substitution($Orig, $New, second)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Orig\":\"suntan\"") && out.contains("\"New\":\"sunset\""),
        "the shipped second-syllable example is suntan -> sunset: {out}"
    );
}

#[test]
fn syllable_substitution_abstains_honestly_on_an_untabled_pair() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"syllable-substitution.adj\"\n\
         ? syllable_substitution(cat, hat, $Position)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "cat -> hat has no shipped row -- honest abstention, never invented: {out}"
    );
}
