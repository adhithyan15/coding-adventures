//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/comparative-adjective-rule.adj`) driven
//! through the built CLI: a native `table` naming five common English
//! comparative-adjective formation rules and what each actually requires,
//! quoted verbatim from Grammarly's "What Are Comparative Adjectives?
//! Definition and Examples" article. A sibling to
//! `superlative-adjective-rule.adj` (which covers only "-est") and
//! distinct from `suffix-meaning.adj`'s agentive `_er_agentive` sense
//! (which is a suffix-meaning fact, not a comparative-degree grammar
//! rule) -- this is the first table anywhere in this stdlib naming the
//! comparative "-er" formation rules. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_comparative_adjective_rule_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/comparative-adjective-rule.adj");
    std::fs::copy(&src, dir.join("comparative-adjective-rule.adj"))
        .expect("copy shipped comparative-adjective-rule.adj");
}

#[test]
fn comparative_adjective_rule_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comparative-adjective-rule.adj\"\n\
         ? comparative_adjective_rule(adjective_ending_in_y, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"change_y_to_i_before_er\""),
        "adjective_ending_in_y means change_y_to_i_before_er: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn comparative_adjective_rule_reverse_binds_the_rule_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comparative-adjective-rule.adj\"\n\
         ? comparative_adjective_rule($R, add_er_suffix)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"R\":\"one_syllable_adjective\""),
        "the shipped add_er_suffix example is one_syllable_adjective: {out}"
    );
}

#[test]
fn comparative_adjective_rule_covers_the_silent_e_rule_distinct_from_est() {
    let dir = scratch("silent_e");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comparative-adjective-rule.adj\"\n\
         ? comparative_adjective_rule(one_syllable_adjective_ending_in_e, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"D\":\"add_r_only\""),
        "a one-syllable adjective already ending in -e just adds -r -- a rule that has no -est-formation counterpart in the sibling table: {out}"
    );
}

#[test]
fn comparative_adjective_rule_covers_the_two_syllable_er_ow_le_rule() {
    let dir = scratch("two_syllable");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comparative-adjective-rule.adj\"\n\
         ? comparative_adjective_rule($R, add_er_or_r_without_spelling_change)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"R\":\"two_syllable_adjective_ending_in_er_ow_or_le\""),
        "two-syllable adjectives ending in -er/-ow/-le add -er or -r without a spelling change: {out}"
    );
}

#[test]
fn comparative_adjective_rule_covers_the_cvc_doubling_rule() {
    let dir = scratch("cvc");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comparative-adjective-rule.adj\"\n\
         ? comparative_adjective_rule(one_syllable_consonant_vowel_consonant, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"D\":\"double_final_consonant_before_er\""),
        "big -> bigger, thin -> thinner -- the -er sibling of superlative-adjective-rule.adj's own double_final_consonant_before_est row: {out}"
    );
}

#[test]
fn comparative_adjective_rule_abstains_honestly_on_an_untabled_rule() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comparative-adjective-rule.adj\"\n\
         ? comparative_adjective_rule(two_syllable_adjective_not_ending_in_er_ow_le_or_y, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "long two-or-more-syllable adjectives use \"more\" instead of \"-er\" -- a real rule the same source covers, but its own supporting text is a bullet-list fragment rather than a clean quotable sentence, so it is deliberately not one of the five tabled here -- honest abstention, never invented: {out}"
    );
}
