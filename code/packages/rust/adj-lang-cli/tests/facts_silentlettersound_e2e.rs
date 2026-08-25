//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/silent-letter-sound.adj`) driven through the
//! built CLI: a native `table` naming the University of Florida Literacy
//! Institute (UFLI) Foundations Toolbox's sole "Silent Letters Unit" lesson
//! (lesson 98 of the "Diphthongs and Silent Letters Units (Lessons 95-98)"
//! page -- the same page `diphthong-sound.adj` cites for its own, distinct
//! lessons 95-96) and the single speech sound each of its three named
//! silent-letter consonant-cluster spellings actually represents: `kn` ->
//! n_sound, `wr` -> r_sound, `mb` -> m_sound. Abstains honestly on `gh`, a
//! real silent-letter pattern (as in "night") but not one of this UFLI
//! lesson's three named patterns. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "adjcli_silentlettersound_{tag}_{}",
        std::process::id()
    ));
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
    let src = facts_stdlib().join("language/silent-letter-sound.adj");
    std::fs::copy(&src, dir.join("silent-letter-sound.adj"))
        .expect("copy shipped silent-letter-sound.adj");
}

#[test]
fn silent_letter_sound_recall_binds_the_sound_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-letter-sound.adj\"\n\
         ? silent_letter_sound(kn, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Sound\":\"n_sound\""),
        "kn makes the n_sound: {out}"
    );
    assert!(
        out.contains("ufli.education.ufl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the UFLI citation at authoritative trust: {out}"
    );
}

#[test]
fn silent_letter_sound_reverse_binds_the_spelling_for_that_sound() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-letter-sound.adj\"\n\
         ? silent_letter_sound($Sp, r_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Sp\":\"wr\""),
        "the silent-letter spelling that makes r_sound is wr: {out}"
    );
}

#[test]
fn silent_letter_sound_mb_recalls_m_sound() {
    let dir = scratch("mb");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-letter-sound.adj\"\n\
         ? silent_letter_sound(mb, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Sound\":\"m_sound\""),
        "mb makes the m_sound: {out}"
    );
}

#[test]
fn silent_letter_sound_abstains_honestly_on_an_untabled_pattern() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"silent-letter-sound.adj\"\n\
         ? silent_letter_sound(gh, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "gh is a real silent-letter pattern (as in \"night\"), but not one of \
         this UFLI lesson's three named patterns -- honest abstention, never \
         invented: {out}"
    );
}
