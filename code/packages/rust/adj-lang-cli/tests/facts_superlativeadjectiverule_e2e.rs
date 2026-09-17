//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/superlative-adjective-rule.adj`) driven
//! through the built CLI: a native `table` naming three common English
//! superlative-adjective formation rules and what each actually requires,
//! quoted verbatim from Grammarly's "What Are Superlative Adjectives?
//! Definition and Examples" article. 0 answer-time model calls.
//!
//! The rows carry the provenance their sentences support (RS-5e, #14986).
//! Rules 1 and 2 each have one sentence naming both the spelling pattern and
//! the action, so each is that row's `source`. Rule 3's action sentence names
//! its pattern only as "the y", so that row `cites` the pattern sentence and
//! the action sentence, and the framing envelope stays primary. The envelope
//! used to be rule 1's own sentence, the primary source of all three answers.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_superlative_adjective_rule_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/superlative-adjective-rule.adj");
    std::fs::copy(&src, dir.join("superlative-adjective-rule.adj"))
        .expect("copy shipped superlative-adjective-rule.adj");
}

const LOCATOR: &str = "https://www.grammarly.com/blog/parts-of-speech/superlative-adjectives/";
const ENVELOPE: &str = "You can make any adjective into a superlative.";
const OLD_ENVELOPE: &str = "If an adjective has only one syllable, most of the time you can simply add the suffix \u{2013}est to the end of the word without changing the spelling.";
const RULE_3_PATTERN: &str = "Adjectives with either one or two syllables have special spelling rules if they end in -y.";

/// (rule, description, "source" or "cites", that row's sentences in page order)
const ROWS: [(&str, &str, &str, &[&str]); 3] = [
    ("one_syllable_adjective", "add_est_suffix", "source", &[OLD_ENVELOPE]),
    (
        "one_syllable_consonant_vowel_consonant",
        "double_final_consonant_before_est",
        "source",
        &["If the last three letters of a one-syllable adjective follow a consonant-vowel-consonant format, like big or thin, you have to double the last consonant and then add \u{2013}est."],
    ),
    (
        "adjective_ending_in_y",
        "change_y_to_i_before_est",
        "cites",
        &[RULE_3_PATTERN, "To make the superlative, first change the y into an i and then add \u{2013}est."],
    ),
];

/// The whole citation run a row's answer must carry, for its shape.
fn row_citation(kind: &str, spans: &[&str]) -> String {
    if kind == "source" {
        format!("\"source\":\"{}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[]", spans[0])
    } else {
        let corr: Vec<String> = spans
            .iter()
            .map(|s| format!("{{\"source\":\"{s}\",\"locator\":\"{LOCATOR}\"}}"))
            .collect();
        format!(
            "\"source\":\"{ENVELOPE}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[{}]",
            corr.join(",")
        )
    }
}

fn row(rule: &str) -> (&'static str, &'static str, &'static str, &'static [&'static str]) {
    *ROWS.iter().find(|r| r.0 == rule).expect("known rule")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("language/superlative-adjective-rule.adj"))
        .expect("read shipped superlative-adjective-rule.adj");
    adj[adj.find("table superlative_adjective_rule").expect("table")..].to_string()
}

#[test]
fn superlative_adjective_rule_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"superlative-adjective-rule.adj\"\n\
         ? superlative_adjective_rule(adjective_ending_in_y, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"change_y_to_i_before_est\""),
        "adjective_ending_in_y means change_y_to_i_before_est: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("grammarly.com") && contains(trust)` (#15209).
    let (_, _, kind, spans) = row("adjective_ending_in_y");
    assert!(
        out.contains(&row_citation(kind, spans)),
        "the envelope is primary and both rule 3 sentences corroborate, in page order: {out}"
    );
}

#[test]
fn superlative_adjective_rule_reverse_binds_the_rule_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"superlative-adjective-rule.adj\"\n\
         ? superlative_adjective_rule($R, add_est_suffix)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"R\":\"one_syllable_adjective\""),
        "the shipped add_est_suffix example is one_syllable_adjective: {out}"
    );
    let (_, _, kind, spans) = row("one_syllable_adjective");
    assert!(out.contains(&row_citation(kind, spans)), "carries the rule 1 citation, whole: {out}");
}

#[test]
fn superlative_adjective_rule_abstains_honestly_on_an_untabled_rule() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"superlative-adjective-rule.adj\"\n\
         ? superlative_adjective_rule(three_or_more_syllable_adjective, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "three_or_more_syllable_adjective is a real rule the source covers (use \"most\" instead) but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}

#[test]
fn every_rule_answer_carries_the_citation_shape_its_sentences_support() {
    for (rule, desc, kind, spans) in ROWS {
        let dir = scratch(&format!("row_{desc}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"superlative-adjective-rule.adj\"\n? superlative_adjective_rule($R, {desc})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {rule}: {out}");
        assert!(out.contains(&format!("\"R\":\"{rule}\"")), "{desc} binds {rule}: {out}");
        assert!(out.contains(&row_citation(kind, spans)), "{rule} ({kind}): the measured citation shape: {out}");
        for (other, _, _, other_spans) in ROWS {
            if other != rule {
                for s in other_spans.iter() {
                    assert!(!out.contains(s), "the {other} sentence must not reach {rule}: {out}");
                }
            }
        }
        if kind == "source" {
            assert!(!out.contains(ENVELOPE), "the envelope is not primary for a source row ({rule}): {out}");
        }
    }
}

#[test]
fn rule_3_carries_the_pattern_sentences_own_hyphen() {
    // The pattern sentence writes "-y" with HYPHEN-MINUS; the section heading
    // above it writes "–y" with an EN DASH. The row carries the sentence's own
    // character, and the heading's form appears nowhere in the table.
    assert!(RULE_3_PATTERN.ends_with("end in -y."), "the pattern sentence ends in HYPHEN-MINUS y");
    assert!(!RULE_3_PATTERN.contains('\u{2013}'), "the pattern sentence has no EN DASH");
    let body = shipped_table();
    assert!(!body.contains("end in \u{2013}y."), "the heading's EN DASH form must not be in the table");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    let body = shipped_table();
    for (rule, desc, kind, spans) in ROWS {
        let mut expected = format!("    row ({rule}, {desc}) {{\n");
        for s in spans.iter() {
            if kind == "source" {
                expected.push_str(&format!("        source \"{s}\"\n"));
            } else {
                expected.push_str(&format!("        cites \"{s}\" locator \"{LOCATOR}\"\n"));
            }
        }
        expected.push_str("    }");
        assert!(body.contains(&expected), "row ({rule}, {desc}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 2, "two row sources (rules 1 and 2)");
    assert_eq!(body.matches("\n        cites \"").count(), 2, "two row corroborations (rule 3)");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n")),
        "the envelope is the framing sentence at the article"
    );
    assert!(!body.contains(&format!("\n    source \"{OLD_ENVELOPE}\"\n    locator")), "not rule 1's sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["syllable", "consonant", "-y", "\u{2013}est", "double", "change the y"] {
        assert!(!folded.contains(word), "the envelope must name no rule, but contains {word:?}");
    }
}
