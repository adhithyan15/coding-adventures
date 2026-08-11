//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/onset-rime.adj`) driven through the built
//! CLI: a native THREE-column `table` naming each of two words' onset/rime
//! split, quoted verbatim from Reading Rockets' "Tuning In to the Sounds in
//! Words" article -- the FOURTH literacy sub-skill in this loop's curriculum
//! sweep, deliberately different in shape from `word-families.adj`'s rhyme
//! derivation (RF.K.2.a), `syllable-count.adj`'s syllable recall
//! (RF.K.2.b), and `initial-sound.adj`'s beginning-sound recall (RF.K.2.d):
//! this grounds onset/rime blending and segmenting (RF.K.2.c). 0
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
    let dir = std::env::temp_dir().join(format!("adjcli_onsetrime_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/onset-rime.adj");
    std::fs::copy(&src, dir.join("onset-rime.adj")).expect("copy shipped onset-rime.adj");
}

#[test]
fn onset_rime_recall_segments_the_word_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"onset-rime.adj\"\n\
         ? onset_rime(sleep, $Onset, $Rime)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Onset\":\"sl\"") && out.contains("\"Rime\":\"eep\""),
        "sleep splits into sl + eep: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn onset_rime_reverse_blends_the_parts_into_the_word() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"onset-rime.adj\"\n\
         ? onset_rime($Word, bl, ast)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Word\":\"blast\""),
        "bl + ast blends into blast: {out}"
    );
}

#[test]
fn onset_rime_abstains_honestly_on_an_unshipped_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"onset-rime.adj\"\n\
         ? onset_rime(cat, $Onset, $Rime)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"cat\" has no shipped row -- honest abstention, never invented: {out}"
    );
}
