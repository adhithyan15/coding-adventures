//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/phoneme-blending.adj`) driven through the
//! built CLI: a native FOUR-column `table` naming the one phoneme blend
//! (/s/, /o/, /p/ -> "soap") walked through on Reading Rockets'
//! "Phonological and Phonemic Awareness: In Practice" module -- the NINTH
//! literacy sub-skill in this loop's curriculum sweep, deliberately the
//! OPPOSITE direction from `phoneme-deletion.adj` and
//! `phoneme-substitution.adj`: this grounds phoneme blending (combining
//! separate sounds into a word) rather than decomposing one. 0 answer-time
//! model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_phonemeblend_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/phoneme-blending.adj");
    std::fs::copy(&src, dir.join("phoneme-blending.adj"))
        .expect("copy shipped phoneme-blending.adj");
}

#[test]
fn phoneme_blending_recall_binds_the_word_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-blending.adj\"\n\
         ? phoneme_blending(s, o, p, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Word\":\"soap\""),
        "blending /s/, /o/, /p/ gives soap: {out}"
    );
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn phoneme_blending_reverse_binds_the_three_sounds() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-blending.adj\"\n\
         ? phoneme_blending($S1, $S2, $S3, soap)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"S1\":\"s\"") && out.contains("\"S2\":\"o\"") && out.contains("\"S3\":\"p\""),
        "soap blends from /s/, /o/, /p/: {out}"
    );
}

#[test]
fn phoneme_blending_abstains_honestly_on_an_untabled_blend() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"phoneme-blending.adj\"\n\
         ? phoneme_blending(b, a, t, $Word)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "b/a/t -> ? has no shipped row -- honest abstention, never invented: {out}"
    );
}
