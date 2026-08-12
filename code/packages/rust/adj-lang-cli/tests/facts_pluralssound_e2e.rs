//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/plural-s-sound.adj`) driven through the
//! built CLI: a native `table` naming three regular plural nouns and the
//! sound each one's -s/-es ending is pronounced with, per Speakspeak's
//! "Pronunciation of 's' and 'es' plural endings" article. The TWELFTH
//! literacy sub-skill library in this loop's sweep. 0 answer-time model
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
    let dir = std::env::temp_dir().join(format!("adjcli_pluralssound_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/plural-s-sound.adj");
    std::fs::copy(&src, dir.join("plural-s-sound.adj")).expect("copy shipped plural-s-sound.adj");
}

#[test]
fn plural_s_sound_recall_binds_the_sound_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"plural-s-sound.adj\"\n\
         ? plural_s_sound(hats, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Sound\":\"s_sound\""),
        "hats's plural ending is the /s/ sound: {out}"
    );
    assert!(
        out.contains("speakspeak.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Speakspeak citation: {out}"
    );
}

#[test]
fn plural_s_sound_reverse_binds_the_word_for_that_sound() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"plural-s-sound.adj\"\n\
         ? plural_s_sound($W, iz_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"W\":\"boxes\""),
        "boxes's plural ending is the /iz/ sound: {out}"
    );
}

#[test]
fn plural_s_sound_abstains_honestly_on_an_untabled_word() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"plural-s-sound.adj\"\n\
         ? plural_s_sound(cats, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "cats is also /s/-sounded but not one of the three tabled example words -- honest abstention, never invented: {out}"
    );
}
