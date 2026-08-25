//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/dolch-sight-word-level.adj`) driven through
//! the built CLI: a native `table` naming which of Edward W. Dolch's five
//! grade-banded reading levels (Pre-Primer, Primer, First Grade, Second
//! Grade, Third Grade -- University of Florida Literacy Institute's own
//! "Dolch High Frequency Word List Slides" deck) a common high-frequency
//! "sight word" is first taught at. Genuinely distinct in KIND from
//! `digraph-sound.adj`/`diphthong-sound.adj` (phonics: spelling -> sound):
//! this is whole-word recognition vocabulary (word -> reading-level band).
//! Ships 25 of the full 220-word Dolch list (the first five words of each
//! level, in the source deck's own listed order), mirroring
//! `food-groups.adj`'s "representative subset" convention. 0 answer-time
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
    let dir = std::env::temp_dir().join(format!("adjcli_dolchsightwordlevel_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/dolch-sight-word-level.adj");
    std::fs::copy(&src, dir.join("dolch-sight-word-level.adj"))
        .expect("copy shipped dolch-sight-word-level.adj");
}

#[test]
fn dolch_sight_word_level_recall_binds_the_level_with_citation() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(the, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Level\":\"pre_primer\""),
        "'the' is a Dolch Pre-Primer word: {out}"
    );
    assert!(
        out.contains("ufli.education.ufl.edu") && out.contains("\"trust\":\"authoritative\""),
        "carries the UFLI citation at authoritative trust: {out}"
    );
}

#[test]
fn dolch_sight_word_level_forward_would_recalls_second_grade() {
    let dir = scratch("forward_second_grade");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(would, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Level\":\"second_grade\""),
        "'would' is a Dolch Second Grade word, a genuinely different level: {out}"
    );
}

#[test]
fn dolch_sight_word_level_reverse_binds_all_five_pre_primer_words() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level($W, pre_primer)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The table ships all five of the Pre-Primer level's shipped words --
    // a genuine one-to-many reverse recall, the same shape
    // `food-groups.adj`'s `food_group($Food, dairy)` reverse query already
    // established in this stdlib.
    for w in ["the", "to", "and", "a", "i"] {
        assert!(
            out.contains(&format!("\"W\":\"{w}\"")),
            "{w} should be a bound Pre-Primer answer: {out}"
        );
    }
}

#[test]
fn dolch_sight_word_level_abstains_honestly_on_a_real_dolch_word_outside_the_shipped_subset() {
    let dir = scratch("abstain_scope");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(you, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "'you' is a REAL Dolch Pre-Primer word, but not one of this \
         table's shipped first-five-per-level subset -- honest abstention \
         on scope, never invented: {out}"
    );
}

#[test]
fn dolch_sight_word_level_abstains_honestly_on_a_non_dolch_word() {
    let dir = scratch("abstain_outside_source");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(elephant, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "'elephant' is not a Dolch service word at all -- honest \
         abstention, never invented: {out}"
    );
}
