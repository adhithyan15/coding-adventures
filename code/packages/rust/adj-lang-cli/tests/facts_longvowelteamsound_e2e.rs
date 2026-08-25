//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/long-vowel-team-sound.adj`) driven through
//! the built CLI: a native `table` naming four lessons of the University of
//! Florida Literacy Institute (UFLI) Foundations Toolbox's "Long Vowel
//! Teams Unit Resources (Lessons 84-88)" page and the single long vowel
//! sound each spelling represents. `ai`/`ay` share long_a_sound (lesson
//! 84), `ee`/`ea`/`ey` share long_e_sound (lesson 85), `oa`/`ow_long_o`/`oe`
//! share long_o_sound (lesson 86), and `ie`/`igh` share long_i_sound
//! (lesson 87) -- a genuine many-keys-to-one-sound shape. The lesson-86
//! "ow" row ships as the disambiguated atom `ow_long_o`, NOT a bare `ow`,
//! because the bare spelling "ow" already carries a DIFFERENT, genuinely
//! distinct source-cited sound in the sibling `diphthong-sound.adj` library
//! (the glided /ow/ diphthong in "cow"/"how", vs. this table's long-O
//! reading in "know"/"grow") -- a real heteronym-in-spelling the header's
//! own design note documents in full. Abstains honestly on a bare `ow`
//! against THIS table's predicate (this table asserts nothing about it) and
//! on `au` (a spelling UFLI's own broader scope and sequence tables under a
//! DIFFERENT unit, "Other Vowel Teams", lesson 93, not this cited Long
//! Vowel Teams page). 0 answer-time model calls.

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
        "adjcli_longvowelteamsound_{tag}_{}",
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
    let src = facts_stdlib().join("language/long-vowel-team-sound.adj");
    std::fs::copy(&src, dir.join("long-vowel-team-sound.adj"))
        .expect("copy shipped long-vowel-team-sound.adj");
}

/// Places BOTH this library and its sibling `diphthong-sound.adj`, to prove
/// the two "ow" senses coexist without conflict when both are in scope --
/// the exact scenario the header's design note is about.
fn place_lib_and_sibling(dir: &Path) {
    place_lib(dir);
    let sibling = facts_stdlib().join("language/diphthong-sound.adj");
    std::fs::copy(&sibling, dir.join("diphthong-sound.adj"))
        .expect("copy shipped diphthong-sound.adj");
}

#[test]
fn long_vowel_team_sound_recall_binds_the_sound_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"long-vowel-team-sound.adj\"\n\
         ? long_vowel_team_sound(ai, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Sound\":\"long_a_sound\""),
        "ai makes the long_a_sound: {out}"
    );
    assert!(
        out.contains("ufli.education.ufl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the UFLI citation at authoritative trust: {out}"
    );
}

#[test]
fn long_vowel_team_sound_reverse_binds_all_three_spellings_of_long_o_sound() {
    let dir = scratch("reverse_long_o");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"long-vowel-team-sound.adj\"\n\
         ? long_vowel_team_sound($Sp, long_o_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Lesson 86 pairs THREE spellings ("oa", "ow", "oe") with the same
    // long-O sound; "ow" ships as the disambiguated atom `ow_long_o`.
    assert!(out.contains("\"Sp\":\"oa\""), "oa carries long_o_sound: {out}");
    assert!(
        out.contains("\"Sp\":\"ow_long_o\""),
        "the disambiguated ow_long_o carries long_o_sound too: {out}"
    );
    assert!(out.contains("\"Sp\":\"oe\""), "oe carries long_o_sound too: {out}");
}

#[test]
fn long_vowel_team_sound_forward_igh_recalls_long_i_sound() {
    let dir = scratch("forward_igh");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"long-vowel-team-sound.adj\"\n\
         ? long_vowel_team_sound(igh, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Sound\":\"long_i_sound\""),
        "igh makes the long_i_sound: {out}"
    );
}

#[test]
fn long_vowel_team_sound_abstains_on_bare_ow_while_diphthong_sound_still_answers_it() {
    let dir = scratch("ow_heteronym");
    place_lib_and_sibling(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"long-vowel-team-sound.adj\"\n\
         import \"diphthong-sound.adj\"\n\
         ? long_vowel_team_sound(ow, $S)\n\
         ? diphthong_sound(ow, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // This table never asserts a bare `ow` row -- the disambiguated
    // `ow_long_o` atom carries the long-O sense instead -- so a bare-`ow`
    // query against THIS predicate must honestly abstain, never silently
    // reuse the sibling library's different, glided-diphthong sense of the
    // identical two letters.
    assert!(
        out.contains(
            "\"query\":\"long_vowel_team_sound(ow, S)\",\"answers\":[],\"abstained\":true"
        ),
        "a bare ow abstains against THIS table's predicate: {out}"
    );
    // The sibling library's own, already-shipped `ow` row is untouched and
    // still answers its own, genuinely different sound.
    assert!(
        out.contains("\"query\":\"diphthong_sound(ow, S)\"")
            && out.contains("\"S\":\"ow_sound\""),
        "diphthong_sound's own ow row still answers ow_sound, unaffected: {out}"
    );
}

#[test]
fn long_vowel_team_sound_abstains_honestly_on_a_different_ufli_unit() {
    let dir = scratch("abstain_au");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"long-vowel-team-sound.adj\"\n\
         ? long_vowel_team_sound(au, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "au is tabled by UFLI under a DIFFERENT unit (Other Vowel Teams, \
         lesson 93), not this cited Long Vowel Teams page -- honest \
         abstention, never invented: {out}"
    );
}
