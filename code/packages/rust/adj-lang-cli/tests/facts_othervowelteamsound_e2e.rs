//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/other-vowel-team-sound.adj`) driven through
//! the built CLI: a native `table` naming five lessons of the University of
//! Florida Literacy Institute (UFLI) Foundations Toolbox's "Other Vowel
//! Teams Unit Resources (Lessons 89-94)" page and the single sound each
//! spelling represents -- the fifth UFLI phonics unit shipped in this
//! stdlib, and the direct sequel to `long-vowel-team-sound.adj`'s own
//! Long Vowel Teams unit (84-88). `u`/`oo` share short_oo_sound (lesson 89);
//! `oo` ALSO carries long_u_sound (lesson 90), the same one-key/many-values
//! shape `digraph-sound.adj`'s own "th" row established; `ew`/`ui`/`ue`
//! join `oo` on long_u_sound (lesson 91); `au`/`aw`/`augh` share aw_sound
//! (lesson 93); and lesson 94 gives two short-vowel exceptions, `ea /ĕ/`
//! and `a /ŏ/`. The lesson-94 "ea" row ships as the disambiguated atom
//! `ea_short_e`, NOT a bare `ea`, because the bare spelling "ea" already
//! carries a DIFFERENT, genuinely distinct source-cited sound in the
//! sibling `long-vowel-team-sound.adj` library (the steady long-E reading
//! in "team"/"rain", vs. this table's short-E reading in "bread"/"head") --
//! a real heteronym-in-spelling the header's own design note documents in
//! full, resolved the identical way that table's own "ow"/"ow_long_o" row
//! was. Abstains honestly on a bare `ea` against THIS table's predicate
//! (this table asserts nothing about it) and on `ey` (a spelling UFLI's own
//! broader scope and sequence tables under the separate Long Vowel Teams
//! unit, lesson 85, not this cited Other Vowel Teams page). 0 answer-time
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
    let dir = std::env::temp_dir().join(format!(
        "adjcli_othervowelteamsound_{tag}_{}",
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
    let src = facts_stdlib().join("language/other-vowel-team-sound.adj");
    std::fs::copy(&src, dir.join("other-vowel-team-sound.adj"))
        .expect("copy shipped other-vowel-team-sound.adj");
}

/// Places BOTH this library and its sibling `long-vowel-team-sound.adj`, to
/// prove the two "ea" senses coexist without conflict when both are in
/// scope -- the exact scenario the header's design note is about.
fn place_lib_and_sibling(dir: &Path) {
    place_lib(dir);
    let sibling = facts_stdlib().join("language/long-vowel-team-sound.adj");
    std::fs::copy(&sibling, dir.join("long-vowel-team-sound.adj"))
        .expect("copy shipped long-vowel-team-sound.adj");
}

#[test]
fn other_vowel_team_sound_recall_binds_the_sound_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"other-vowel-team-sound.adj\"\n\
         ? other_vowel_team_sound(au, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Sound\":\"aw_sound\""),
        "au makes the aw_sound: {out}"
    );
    assert!(
        out.contains("ufli.education.ufl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the UFLI citation at authoritative trust: {out}"
    );
}

#[test]
fn other_vowel_team_sound_reverse_binds_all_four_spellings_of_long_u_sound() {
    let dir = scratch("reverse_long_u");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"other-vowel-team-sound.adj\"\n\
         ? other_vowel_team_sound($Sp, long_u_sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Lesson 90 ("oo") and lesson 91 ("ew", "ui", "ue") pair FOUR spellings
    // with the same source-notated long-U sound.
    assert!(out.contains("\"Sp\":\"oo\""), "oo carries long_u_sound: {out}");
    assert!(out.contains("\"Sp\":\"ew\""), "ew carries long_u_sound too: {out}");
    assert!(out.contains("\"Sp\":\"ui\""), "ui carries long_u_sound too: {out}");
    assert!(out.contains("\"Sp\":\"ue\""), "ue carries long_u_sound too: {out}");
}

#[test]
fn other_vowel_team_sound_oo_carries_both_lesson89_and_lesson90_sounds() {
    let dir = scratch("oo_two_senses");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"other-vowel-team-sound.adj\"\n\
         ? other_vowel_team_sound(oo, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The SAME one-key/many-values shape digraph-sound.adj's own "th" row
    // established: "oo" forward-recalls BOTH lesson 89's short sound and
    // lesson 90's long sound, an honest reflection of the source's own
    // two-lesson split.
    assert!(
        out.contains("\"Sound\":\"short_oo_sound\""),
        "oo carries short_oo_sound (lesson 89): {out}"
    );
    assert!(
        out.contains("\"Sound\":\"long_u_sound\""),
        "oo ALSO carries long_u_sound (lesson 90): {out}"
    );
}

#[test]
fn other_vowel_team_sound_abstains_on_bare_ea_while_long_vowel_team_sound_still_answers_it() {
    let dir = scratch("ea_heteronym");
    place_lib_and_sibling(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"other-vowel-team-sound.adj\"\n\
         import \"long-vowel-team-sound.adj\"\n\
         ? other_vowel_team_sound(ea, $S)\n\
         ? long_vowel_team_sound(ea, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // This table never asserts a bare `ea` row -- the disambiguated
    // `ea_short_e` atom carries the short-E sense instead -- so a bare-`ea`
    // query against THIS predicate must honestly abstain, never silently
    // reuse the sibling library's different, long-E sense of the identical
    // two letters.
    assert!(
        out.contains(
            "\"query\":\"other_vowel_team_sound(ea, S)\",\"answers\":[],\"abstained\":true"
        ),
        "a bare ea abstains against THIS table's predicate: {out}"
    );
    // The sibling library's own, already-shipped `ea` row is untouched and
    // still answers its own, genuinely different sound.
    assert!(
        out.contains("\"query\":\"long_vowel_team_sound(ea, S)\"")
            && out.contains("\"S\":\"long_e_sound\""),
        "long_vowel_team_sound's own ea row still answers long_e_sound, unaffected: {out}"
    );
}

#[test]
fn other_vowel_team_sound_ea_short_e_recalls_short_e_sound_directly() {
    let dir = scratch("ea_short_e_direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"other-vowel-team-sound.adj\"\n\
         ? other_vowel_team_sound(ea_short_e, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Sound\":\"short_e_sound\""),
        "the disambiguated ea_short_e atom carries short_e_sound: {out}"
    );
}

#[test]
fn other_vowel_team_sound_abstains_honestly_on_a_different_ufli_unit() {
    let dir = scratch("abstain_ey");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"other-vowel-team-sound.adj\"\n\
         ? other_vowel_team_sound(ey, $Sound)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "ey is tabled by UFLI under a DIFFERENT unit (Long Vowel Teams, \
         lesson 85), not this cited Other Vowel Teams page -- honest \
         abstention, never invented: {out}"
    );
}
