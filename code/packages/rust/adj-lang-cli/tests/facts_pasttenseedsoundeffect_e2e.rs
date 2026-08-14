//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/past-tense-ed-sound-effect.adj`) driven
//! through the built CLI: a native `table` naming the pronunciation effect
//! the /id/ past-tense -ed sound has, decoded from a span already sitting
//! unused inside the SAME 7ESL quote `past-tense-ed-sound.adj`'s own header
//! already reproduces -- a sibling to that table. Resolves binding-query
//! recall (both directions) with the source's citation, and abstains on a
//! real, already-tabled -ed sound (t_sound) whose own rule states no effect
//! -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_pasttenseedsoundeffect_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/past-tense-ed-sound-effect.adj");
    std::fs::copy(&src, dir.join("past-tense-ed-sound-effect.adj"))
        .expect("copy shipped past-tense-ed-sound-effect.adj");
}

#[test]
fn past_tense_ed_sound_effect_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"past-tense-ed-sound-effect.adj\"\n\
         ? past_tense_ed_sound_effect(id_sound, $Effect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"past_tense_ed_sound_effect(id_sound, adds_a_whole_syllable)\""),
        "the /id/ sound adds a whole syllable to a word: {out}"
    );
    assert!(
        out.contains("7esl.com") && out.contains("\"trust\":\"consensus\""),
        "carries the 7ESL citation: {out}"
    );
}

#[test]
fn past_tense_ed_sound_effect_recalls_backward_from_a_bound_effect() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"past-tense-ed-sound-effect.adj\"\n\
         ? past_tense_ed_sound_effect($Sound, adds_a_whole_syllable)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"past_tense_ed_sound_effect(id_sound, adds_a_whole_syllable)\""),
        "the effect names id_sound: {out}"
    );
}

#[test]
fn past_tense_ed_sound_effect_abstains_honestly_on_t_sound() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"past-tense-ed-sound-effect.adj\"\n\
         ? past_tense_ed_sound_effect(t_sound, $Effect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "t_sound's own rule states no effect -- honest abstention: {out}"
    );
}
