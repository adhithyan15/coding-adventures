//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/past-tense-ed-sound.adj`) driven through the
//! built CLI: a native `table` naming three regular past-tense verbs and
//! the sound each one's -ed ending is pronounced with, per 7ESL's
//! "Pronunciation of ED" article. The ELEVENTH literacy sub-skill library
//! in this loop's sweep. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_pasttenseedsound_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/past-tense-ed-sound.adj");
    std::fs::copy(&src, dir.join("past-tense-ed-sound.adj"))
        .expect("copy shipped past-tense-ed-sound.adj");
}

#[test]
fn past_tense_ed_sound_recall_binds_the_sound_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"past-tense-ed-sound.adj\"\n\
         ? past_tense_ed_sound(walked, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Sound\":\"t_sound\""),
        "walked's -ed ending is the /t/ sound: {out}"
    );
    assert!(
        out.contains("7esl.com") && out.contains("\"trust\":\"consensus\""),
        "carries the 7ESL citation: {out}"
    );
}

#[test]
fn past_tense_ed_sound_reverse_binds_the_word_for_that_sound() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"past-tense-ed-sound.adj\"\n\
         ? past_tense_ed_sound($W, id_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"W\":\"wanted\""),
        "wanted's -ed ending is the /id/ sound: {out}"
    );
}

#[test]
fn past_tense_ed_sound_abstains_honestly_on_an_untabled_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"past-tense-ed-sound.adj\"\n\
         ? past_tense_ed_sound(played, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "played is also /d/-sounded but not one of the three tabled example words -- honest abstention, never invented: {out}"
    );
}
