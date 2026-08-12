//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/phoneme-deletion.adj`) driven through the
//! built CLI: a native THREE-column `table` naming the one phoneme
//! deletion ("bike" -> "by", removing /k/) walked through on Reading
//! Rockets' "Phonological and Phonemic Awareness: In Practice" module --
//! the SIXTH literacy sub-skill in this loop's curriculum sweep,
//! deliberately different from `word-families.adj`, `syllable-count.adj`,
//! `onset-rime.adj`, `initial-sound.adj`, and `phoneme-substitution.adj`:
//! this grounds phoneme deletion, not substitution. 0 answer-time model
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
    let dir = std::env::temp_dir().join(format!("adjcli_phonemedel_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/phoneme-deletion.adj");
    std::fs::copy(&src, dir.join("phoneme-deletion.adj")).expect("copy shipped phoneme-deletion.adj");
}

#[test]
fn phoneme_deletion_recall_binds_the_new_word_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-deletion.adj\"\n\
         ? phoneme_deletion(bike, k, $New)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"New\":\"by\""),
        "removing bike's last sound /k/ gives by: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn phoneme_deletion_reverse_binds_the_original_word_and_sound() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-deletion.adj\"\n\
         ? phoneme_deletion($Orig, $Sound, by)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Orig\":\"bike\"") && out.contains("\"Sound\":\"k\""),
        "by came from bike by removing /k/: {out}"
    );
}

#[test]
fn phoneme_deletion_abstains_honestly_on_an_untabled_deletion() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-deletion.adj\"\n\
         ? phoneme_deletion(cat, c, $New)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "cat -> ? via removing /c/ has no shipped row -- honest abstention, never invented: {out}"
    );
}
