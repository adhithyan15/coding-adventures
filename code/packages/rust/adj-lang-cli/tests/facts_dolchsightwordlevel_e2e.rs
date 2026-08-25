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
//! (extend): completed the Third Grade level to its full 41 words the same
//! way. Round 4 (extend): completed the Primer level to its full 52 words,
//! the same zero-new-sourcing re-parse. Round 5 (extend): completes BOTH
//! remaining levels -- First Grade (full 41 words) and Second Grade (full
//! 46 words) -- in one round, after empirically confirming against the
//! real built CLI that First Grade's `from`/`when` (reserved-grammar-
//! keyword-shaped) atoms parse fine in `row(...)` position, and resolving
//! Second Grade's apostrophe-bearing "don't" using the SAME `dont` atom
//! convention `language/contraction.adj` already established. All 220 of
//! Dolch's own words are now shipped -- the full list, no more partial
//! levels. 0 answer-time model calls.

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
        "'would' is a Dolch Second Grade word, a genuinely different level \
         from Pre-Primer: {out}"
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
fn dolch_sight_word_level_forward_please_recalls_primer() {
    let dir = scratch("forward_primer_extension");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(please, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Level\":\"primer\""),
        "'please' is the 52nd (last) word of the now-completed Primer \
         level -- confirms this round's extension shipped correctly: {out}"
    );
}

#[test]
fn dolch_sight_word_level_reverse_binds_all_fifty_two_primer_words() {
    let dir = scratch("reverse_primer");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level($W, primer)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Primer is now a COMPLETE Dolch level (52/52 words) as of this round's
    // extension -- a genuine one-to-many reverse recall over the full
    // level, the same shape the Pre-Primer and Third Grade reverse tests
    // above already established, just carried to completion for this
    // third level.
    for w in [
        "he", "was", "that", "she", "on", "they", "but", "at", "with", "all", "there", "out",
        "be", "have", "am", "do", "did", "what", "so", "get", "like", "this", "will", "yes",
        "went", "are", "now", "no", "came", "ride", "into", "good", "want", "too", "pretty",
        "four", "saw", "well", "ran", "brown", "eat", "who", "new", "must", "black", "white",
        "soon", "our", "ate", "say", "under", "please",
    ] {
        assert!(
            out.contains(&format!("\"W\":\"{w}\"")),
            "{w} should be a bound Primer answer: {out}"
        );
    }
}

#[test]
fn dolch_sight_word_level_forward_thank_recalls_first_grade() {
    let dir = scratch("forward_first_grade_extension");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(thank, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Level\":\"first_grade\""),
        "'thank' is the 41st (last) word of the now-completed First Grade \
         level -- confirms this round's extension shipped correctly: {out}"
    );
}

#[test]
fn dolch_sight_word_level_forward_from_recalls_first_grade() {
    // `from` is one of the two reserved-grammar-keyword-shaped words this
    // loop's tracking issue flagged as an open question across three prior
    // rounds -- empirically confirmed this round to parse fine as a plain
    // atom in `row(...)` position and in query position, exactly like
    // `to`/`and`/`for`/`if` before it.
    let dir = scratch("forward_first_grade_from");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(from, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Level\":\"first_grade\""),
        "'from' should recall first_grade despite being adj-lang \
         reserved-keyword-shaped: {out}"
    );
}

#[test]
fn dolch_sight_word_level_forward_when_recalls_first_grade() {
    // `when` is the other reserved-grammar-keyword-shaped word (see
    // `dolch_sight_word_level_forward_from_recalls_first_grade` above).
    let dir = scratch("forward_first_grade_when");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(when, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Level\":\"first_grade\""),
        "'when' should recall first_grade despite being adj-lang \
         reserved-keyword-shaped: {out}"
    );
}

#[test]
fn dolch_sight_word_level_reverse_binds_all_forty_one_first_grade_words() {
    let dir = scratch("reverse_first_grade");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level($W, first_grade)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // First Grade is now a COMPLETE Dolch level (41/41 words) as of this
    // round's extension -- a genuine one-to-many reverse recall over the
    // full level, including the two reserved-keyword-shaped words `from`
    // and `when`.
    for w in [
        "of", "his", "had", "him", "her", "some", "as", "then", "could", "when", "were", "them",
        "ask", "an", "over", "just", "from", "any", "how", "know", "put", "take", "every", "old",
        "by", "after", "think", "let", "going", "walk", "again", "may", "stop", "fly", "round",
        "give", "once", "open", "has", "live", "thank",
    ] {
        assert!(
            out.contains(&format!("\"W\":\"{w}\"")),
            "{w} should be a bound First Grade answer: {out}"
        );
    }
}

#[test]
fn dolch_sight_word_level_forward_many_recalls_second_grade() {
    let dir = scratch("forward_second_grade_extension");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(many, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Level\":\"second_grade\""),
        "'many' is the 46th (last) word of the now-completed Second Grade \
         level -- confirms this round's extension shipped correctly: {out}"
    );
}

#[test]
fn dolch_sight_word_level_forward_dont_recalls_second_grade() {
    // The apostrophe-bearing "don't" this loop's tracking issue flagged as
    // an open question across three prior rounds -- resolved this round
    // using the SAME `dont` (no apostrophe) atom convention
    // `language/contraction.adj` already established for this exact word.
    let dir = scratch("forward_second_grade_apostrophe_word");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level(dont, $Level)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Level\":\"second_grade\""),
        "'dont' (standing for the deck's own \"don't\") should recall \
         second_grade: {out}"
    );
}

#[test]
fn dolch_sight_word_level_reverse_binds_all_forty_six_second_grade_words() {
    let dir = scratch("reverse_second_grade");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"dolch-sight-word-level.adj\"\n\
         ? dolch_sight_word_level($W, second_grade)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Second Grade is now a COMPLETE Dolch level (46/46 words) as of this
    // round's extension -- a genuine one-to-many reverse recall over the
    // full level, including the apostrophe-derived `dont` atom.
    for w in [
        "would", "very", "your", "its", "around", "dont", "right", "green", "their", "call",
        "sleep", "five", "wash", "or", "before", "been", "off", "cold", "tell", "work", "first",
        "does", "goes", "write", "always", "made", "gave", "us", "buy", "those", "use", "fast",
        "pull", "both", "sit", "which", "read", "why", "found", "because", "best", "upon",
        "these", "sing", "wish", "many",
    ] {
        assert!(
            out.contains(&format!("\"W\":\"{w}\"")),
            "{w} should be a bound Second Grade answer: {out}"
        );
    }
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
