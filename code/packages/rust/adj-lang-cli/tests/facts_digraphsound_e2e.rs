//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/digraph-sound.adj`) driven through the built
//! CLI: a native `table` naming nine common consonant digraph lessons and
//! the single speech sound each represents, quoted verbatim from the
//! University of Florida Literacy Institute (UFLI) Foundations Toolbox's
//! "Digraphs Unit Resources (Lessons 42-53)" page. `th` carries TWO rows
//! (voiced and unvoiced), an honest one-key/many-values reflection of the
//! source's own lesson split. Abstains honestly on `qu`, a real digraph the
//! same UFLI scope-and-sequence covers elsewhere but not one of these nine
//! lessons. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_digraphsound_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/digraph-sound.adj");
    std::fs::copy(&src, dir.join("digraph-sound.adj")).expect("copy shipped digraph-sound.adj");
}

#[test]
fn digraph_sound_recall_binds_the_sound_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"digraph-sound.adj\"\n\
         ? digraph_sound(sh, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Sound\":\"sh_sound\""),
        "sh makes the sh_sound: {out}"
    );
    assert!(
        out.contains("ufli.education.ufl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the UFLI citation at authoritative trust: {out}"
    );
}

#[test]
fn digraph_sound_reverse_binds_the_digraph_for_that_sound() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"digraph-sound.adj\"\n\
         ? digraph_sound($D, ch_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"D\":\"ch\""),
        "the digraph that makes ch_sound is ch: {out}"
    );
}

#[test]
fn digraph_sound_th_recalls_both_voiced_and_unvoiced() {
    let dir = scratch("th_both");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"digraph-sound.adj\"\n\
         ? digraph_sound(th, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // th is a genuine one-key/many-values row pair -- the source splits it
    // into a voiced lesson (as in "this") and an unvoiced lesson (as in
    // "think"), so a forward recall yields BOTH sounds, not just one.
    assert!(
        out.contains("\"Sound\":\"th_voiced_sound\""),
        "th recalls th_voiced_sound: {out}"
    );
    assert!(
        out.contains("\"Sound\":\"th_unvoiced_sound\""),
        "th recalls th_unvoiced_sound: {out}"
    );
}

#[test]
fn digraph_sound_wh_and_ph_share_the_same_source_lesson() {
    let dir = scratch("wh_ph");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"digraph-sound.adj\"\n\
         ? digraph_sound(wh, $Sound)\n\
         ? digraph_sound(ph, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Sound\":\"w_sound\""),
        "wh makes the w_sound: {out}"
    );
    assert!(
        out.contains("\"Sound\":\"f_sound\""),
        "ph makes the f_sound: {out}"
    );
}

#[test]
fn digraph_sound_abstains_honestly_on_an_untabled_digraph() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"digraph-sound.adj\"\n\
         ? digraph_sound(qu, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "qu is a real digraph the same UFLI scope-and-sequence covers elsewhere, \
         but not one of these nine lessons -- honest abstention, never invented: {out}"
    );
}
