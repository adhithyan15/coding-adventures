//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/sound-device-type.adj`) driven through the
//! built CLI: a native `table` naming two sound devices and what each
//! actually does, quoted verbatim from Grammarly's "20 Types of Figures
//! of Speech: Definitions and Examples" article. 0 answer-time model
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
    let dir = std::env::temp_dir().join(format!("adjcli_sound_device_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/sound-device-type.adj");
    std::fs::copy(&src, dir.join("sound-device-type.adj")).expect("copy shipped sound-device-type.adj");
}

#[test]
fn sound_device_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sound-device-type.adj\"\n\
         ? sound_device_type(onomatopoeia, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"is_when_a_word_imitates_the_natural_sound_of_a_thing\""),
        "onomatopoeia means is_when_a_word_imitates_the_natural_sound_of_a_thing: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn sound_device_type_reverse_binds_the_device_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sound-device-type.adj\"\n\
         ? sound_device_type($D, repeating_consonant_sounds_right_next_to_each_other)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"D\":\"alliteration\""),
        "the shipped repeating_consonant_sounds_right_next_to_each_other example is alliteration: {out}"
    );
}

#[test]
fn sound_device_type_abstains_honestly_on_an_untabled_device() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"sound-device-type.adj\"\n\
         ? sound_device_type(simile, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "simile is a real figure of speech the same page also covers, but it's already separately shipped as simile-meaning.adj, unlike the two tabled here -- honest abstention, never invented: {out}"
    );
}
