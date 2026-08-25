//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/dolch-sight-word-level.adj`) driven through
//! the built CLI: a native `table` naming which of Edward W. Dolch's five
//! grade-banded reading levels (Pre-Primer, Primer, First Grade, Second
//! Grade, Third Grade -- University of Florida Literacy Institute's own
//! "Dolch High Frequency Word List Slides" deck) a common high-frequency
//! "sight word" is first taught at. Genuinely distinct in KIND from
//! `digraph-sound.adj`/`diphthong-sound.adj` (phonics: spelling -> sound):
//! this is whole-word recognition vocabulary (word -> reading-level band).
//! Round 2 (extend): completed the Pre-Primer level to its full 40 words
//! (re-fetched and re-parsed the SAME cited UFLI slide deck). Round 3
//! (extend): completes the Third Grade level to its full 41 words the same
//! way -- Primer/First Grade/Second Grade still ship only their first five
//! words each -- 96 of the full 220-word Dolch list total, mirroring
//! `food-groups.adj`'s "representative subset" convention for the three
//! still-partial levels. 0 answer-time model calls.

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
fn dolch_sight_word_level_forward_funny_recalls_pre_primer() {
    let dir = scratch("forward_pre_primer_extension");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(funny, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Level\":\"pre_primer\""),
        "'funny' is the 40th (last) word of the now-completed Pre-Primer \
         level -- confirms this round's extension shipped correctly: {out}"
    );
}

#[test]
fn dolch_sight_word_level_reverse_binds_all_forty_pre_primer_words() {
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
    // Pre-Primer is now a COMPLETE Dolch level (40/40 words) as of this
    // round's extension -- a genuine one-to-many reverse recall over the
    // full level, the same shape `food-groups.adj`'s
    // `food_group($Food, dairy)` reverse query already established in this
    // stdlib, just carried all the way to completion for this one level.
    for w in [
        "the", "to", "and", "a", "i", "you", "it", "in", "said", "for", "up", "look", "is", "go",
        "we", "little", "down", "can", "see", "not", "one", "my", "me", "big", "come", "blue",
        "red", "where", "jump", "away", "here", "help", "make", "yellow", "two", "play", "run",
        "find", "three", "funny",
    ] {
        assert!(
            out.contains(&format!("\"W\":\"{w}\"")),
            "{w} should be a bound Pre-Primer answer: {out}"
        );
    }
}

#[test]
fn dolch_sight_word_level_forward_laugh_recalls_third_grade() {
    let dir = scratch("forward_third_grade_extension");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(laugh, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Level\":\"third_grade\""),
        "'laugh' is the 41st (last) word of the now-completed Third Grade \
         level -- confirms this round's extension shipped correctly: {out}"
    );
}

#[test]
fn dolch_sight_word_level_reverse_binds_all_forty_one_third_grade_words() {
    let dir = scratch("reverse_third_grade");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level($W, third_grade)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Third Grade is now a COMPLETE Dolch level (41/41 words) as of this
    // round's extension -- a genuine one-to-many reverse recall over the
    // full level, the same shape the Pre-Primer reverse test above already
    // established, just carried to completion for this second level.
    for w in [
        "if", "long", "about", "got", "six", "never", "seven", "eight", "today", "myself",
        "much", "keep", "try", "start", "ten", "bring", "drink", "only", "better", "hold",
        "warm", "full", "done", "light", "pick", "hurt", "cut", "kind", "fall", "carry",
        "small", "own", "show", "hot", "far", "draw", "clean", "grow", "together", "shall",
        "laugh",
    ] {
        assert!(
            out.contains(&format!("\"W\":\"{w}\"")),
            "{w} should be a bound Third Grade answer: {out}"
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
         ? dolch_sight_word_level(they, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "'they' is a REAL Dolch Primer word (its sixth), but Primer still \
         ships only its first-five subset (unlike the now-completed \
         Pre-Primer level) -- honest abstention on scope, never invented: \
         {out}"
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
