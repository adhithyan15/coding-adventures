//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/initial-sound.adj`) driven through the built
//! CLI: a native `table` naming a word's beginning sound (phoneme), quoted
//! verbatim from Reading Rockets' canonical phoneme-identity example -- the
//! THIRD literacy sub-skill in this loop's curriculum sweep, deliberately
//! different in shape from `word-families.adj`'s rhyme derivation (RF.K.2.a)
//! and `syllable-count.adj`'s syllable recall (RF.K.2.b): this is initial-
//! sound isolation (RF.K.2.d). 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_initialsound_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/initial-sound.adj");
    std::fs::copy(&src, dir.join("initial-sound.adj"))
        .expect("copy shipped initial-sound.adj");
}

#[test]
fn initial_sound_recall_binds_the_sound_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"initial-sound.adj\"\n\
         ? initial_sound(bell, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"S\":\"b\""), "bell starts with /b/: {out}");
    assert!(
        out.contains("readingrockets.org") && out.contains("\"trust\":\"consensus\""),
        "carries the Reading Rockets citation: {out}"
    );
}

#[test]
fn initial_sound_reverse_binds_every_word_sharing_that_sound() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"initial-sound.adj\"\n\
         ? initial_sound($W, b)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    for w in ["bell", "bike", "boy"] {
        assert!(
            out.contains(&format!("\"W\":\"{w}\"")),
            "{w} starts with /b/: {out}"
        );
    }
}

#[test]
fn initial_sound_abstains_honestly_on_an_unshipped_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"initial-sound.adj\"\n\
         ? initial_sound(cup, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "\"cup\" has no shipped row -- honest abstention, never invented: {out}"
    );
}
