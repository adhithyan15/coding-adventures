//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/diphthong-sound.adj`) driven through the
//! built CLI: a native `table` naming the two diphthong lessons of the
//! University of Florida Literacy Institute (UFLI) Foundations Toolbox's
//! "Diphthongs and Silent Letters Units (Lessons 95-98)" page and the
//! single glided vowel sound each spelling represents. `oi`/`oy` share the
//! same sound (lesson 95) and `ou`/`ow` share the same sound (lesson 96) --
//! a genuine many-keys-to-one-sound shape, the mirror image of
//! `digraph-sound.adj`'s one-key-to-many-sounds `th` case. Abstains
//! honestly on `au`, a spelling UFLI's own broader scope and sequence
//! tables under a DIFFERENT unit ("Other Vowel Teams", lesson 93), not this
//! cited Diphthongs page. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_diphthongsound_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/diphthong-sound.adj");
    std::fs::copy(&src, dir.join("diphthong-sound.adj")).expect("copy shipped diphthong-sound.adj");
}

#[test]
fn diphthong_sound_recall_binds_the_sound_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"diphthong-sound.adj\"\n\
         ? diphthong_sound(oi, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Sound\":\"oi_sound\""),
        "oi makes the oi_sound: {out}"
    );
    assert!(
        out.contains("ufli.education.ufl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the UFLI citation at authoritative trust: {out}"
    );
}

#[test]
fn diphthong_sound_reverse_binds_both_spellings_of_oi_sound() {
    let dir = scratch("reverse_oi");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"diphthong-sound.adj\"\n\
         ? diphthong_sound($D, oi_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Many-keys-to-one-sound: the source's own lesson 95 pairs BOTH "oi"
    // and "oy" with the same /oi/ sound, so a backward recall on the
    // sound must yield both spellings, not just one.
    assert!(out.contains("\"D\":\"oi\""), "oi carries oi_sound: {out}");
    assert!(out.contains("\"D\":\"oy\""), "oy carries oi_sound too: {out}");
}

#[test]
fn diphthong_sound_reverse_binds_both_spellings_of_ow_sound() {
    let dir = scratch("reverse_ow");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"diphthong-sound.adj\"\n\
         ? diphthong_sound($D, ow_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Lesson 96 pairs BOTH "ou" and "ow" with the same /ow/ sound.
    assert!(out.contains("\"D\":\"ou\""), "ou carries ow_sound: {out}");
    assert!(out.contains("\"D\":\"ow\""), "ow carries ow_sound too: {out}");
}

#[test]
fn diphthong_sound_forward_ow_recalls_ow_sound() {
    let dir = scratch("forward_ow");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"diphthong-sound.adj\"\n\
         ? diphthong_sound(ow, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Sound\":\"ow_sound\""),
        "ow makes the ow_sound: {out}"
    );
}

#[test]
fn diphthong_sound_abstains_honestly_on_a_different_ufli_unit() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"diphthong-sound.adj\"\n\
         ? diphthong_sound(au, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "au is tabled by UFLI under a DIFFERENT unit (Other Vowel Teams, \
         lesson 93), not this cited Diphthongs page -- honest abstention, \
         never invented: {out}"
    );
}
