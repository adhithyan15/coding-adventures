//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/comparative-adjective-rule.adj`) driven
//! through the built CLI: a native `table` naming five common English
//! comparative-adjective formation rules and what each actually requires,
//! from Grammarly's comparative-adjectives article. 0 answer-time model calls.
//!
//! The rows carry the provenance their sentences support. Rules 1, 2 and 4
//! each have one sentence naming both the spelling pattern and the action, so
//! each is that row's `source`. Rule 3's action sentence names its pattern only
//! as "these", and rule 5's -er/-ow and -le halves are separate paragraphs, so
//! those rows `cites` both sentences and the framing envelope stays primary.
//! The envelope used to be rule 1's own sentence, the primary source of all
//! five answers.

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

const LOCATOR: &str = "https://www.grammarly.com/blog/parts-of-speech/comparative-adjectives/";
const ENVELOPE: &str = "In theory, any adjective can become a comparative adjective, as long as you follow the rules.";
const OLD_ENVELOPE: &str = "For most adjectives with one syllable, simply add the suffix \u{2013}er at the end of the word without changing the spelling.";

/// (rule, description, "source" or "cites", that row's sentences in page
/// order) -- generated from the converter's page-verified spans, not retyped.
const ROWS: [(&str, &str, &str, &[&str]); 5] = [
    ("one_syllable_adjective", "add_er_suffix", "source", &["For most adjectives with one syllable, simply add the suffix \u{2013}er at the end of the word without changing the spelling."]),
    ("one_syllable_adjective_ending_in_e", "add_r_only", "source", &["If a one-syllable adjective already ends in -e, just add an -r at the end."]),
    ("one_syllable_consonant_vowel_consonant", "double_final_consonant_before_er", "cites", &["Be careful of one-syllable adjectives with the last three letters in a consonant-vowel-consonant format, like big or thin.", "For these, you have to double the last consonant and then add \u{2013}er."]),
    ("adjective_ending_in_y", "change_y_to_i_before_er", "source", &["If an adjective with either one or two syllables ends in a -y, first change the y into an i and then add \u{2013}er."]),
    ("two_syllable_adjective_ending_in_er_ow_or_le", "add_er_or_r_without_spelling_change", "cites", &["If an adjective with two syllables ends with -er (like bitter) or \u{2013}ow (like narrow), you can just add \u{2013}er to the end without changing the spelling (bitterer or narrower).", "If a two-syllable adjective ends in \u{2013}le, you can just add \u{2013}r without adding a second e."]),
];

/// A string as the serializer writes it inside JSON: a quote becomes \".
fn json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn row(rule: &str) -> (&'static str, &'static str, &'static str, &'static [&'static str]) {
    *ROWS.iter().find(|r| r.0 == rule).expect("known rule")
}

/// The whole citation run a row's answer must carry, for its shape.
fn row_citation(rule: &str) -> String {
    let (_, _, kind, spans) = row(rule);
    if kind == "source" {
        format!(
            "\"source\":\"{}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[]",
            json(spans[0])
        )
    } else {
        let corr: Vec<String> = spans
            .iter()
            .map(|s| format!("{{\"source\":\"{}\",\"locator\":\"{LOCATOR}\"}}", json(s)))
            .collect();
        format!(
            "\"source\":\"{}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[{}]",
            json(ENVELOPE),
            corr.join(",")
        )
    }
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("language/comparative-adjective-rule.adj"))
        .expect("read shipped comparative-adjective-rule.adj");
    adj[adj.find("table comparative_adjective_rule").expect("table")..].to_string()
}

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("language/comparative-adjective-rule.adj"),
        dir.join("comparative-adjective-rule.adj"),
    )
    .expect("copy shipped comparative-adjective-rule.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"comparative-adjective-rule.adj\"\n? {query}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn comparative_adjective_rule_recall_binds_the_description_directly() {
    let out = ask("direct", "comparative_adjective_rule(adjective_ending_in_y, $D)");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"change_y_to_i_before_er\""),
        "adjective_ending_in_y means change_y_to_i_before_er: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("grammarly.com") && contains(trust)` (#15209).
    assert!(out.contains(&row_citation("adjective_ending_in_y")), "carries the rule 4 citation, whole: {out}");
}

#[test]
fn comparative_adjective_rule_reverse_binds_the_rule_for_that_description() {
    let out = ask("reverse", "comparative_adjective_rule($R, add_er_suffix)");
    assert!(
        out.contains("\"R\":\"one_syllable_adjective\""),
        "the shipped add_er_suffix example is one_syllable_adjective: {out}"
    );
    assert!(out.contains(&row_citation("one_syllable_adjective")), "carries the rule 1 citation, whole: {out}");
}

#[test]
fn comparative_adjective_rule_covers_the_silent_e_rule_distinct_from_est() {
    let out = ask("silent_e", "comparative_adjective_rule(one_syllable_adjective_ending_in_e, $D)");
    assert!(
        out.contains("\"D\":\"add_r_only\""),
        "a one-syllable adjective already ending in -e just adds -r -- a rule that has no -est-formation counterpart in the sibling table: {out}"
    );
    assert!(out.contains(&row_citation("one_syllable_adjective_ending_in_e")), "carries the rule 2 citation, whole: {out}");
}

#[test]
fn comparative_adjective_rule_covers_the_two_syllable_er_ow_le_rule() {
    let out = ask("two_syllable", "comparative_adjective_rule($R, add_er_or_r_without_spelling_change)");
    assert!(
        out.contains("\"R\":\"two_syllable_adjective_ending_in_er_ow_or_le\""),
        "two-syllable adjectives ending in -er/-ow/-le add -er or -r without a spelling change: {out}"
    );
    assert!(
        out.contains(&row_citation("two_syllable_adjective_ending_in_er_ow_or_le")),
        "the envelope is primary and both rule 5 sentences corroborate, in page order: {out}"
    );
}

#[test]
fn comparative_adjective_rule_covers_the_cvc_doubling_rule() {
    let out = ask("cvc", "comparative_adjective_rule(one_syllable_consonant_vowel_consonant, $D)");
    assert!(
        out.contains("\"D\":\"double_final_consonant_before_er\""),
        "big -> bigger, thin -> thinner -- the -er sibling of superlative-adjective-rule.adj's own double_final_consonant_before_est row: {out}"
    );
    assert!(
        out.contains(&row_citation("one_syllable_consonant_vowel_consonant")),
        "the envelope is primary and both rule 3 sentences corroborate, in page order: {out}"
    );
}

#[test]
fn comparative_adjective_rule_abstains_honestly_on_an_untabled_rule() {
    let out = ask("abstain", "comparative_adjective_rule(two_syllable_adjective_not_ending_in_er_ow_le_or_y, $D)");
    assert!(
        out.contains("\"abstained\":true"),
        "long two-or-more-syllable adjectives use \"more\" instead of \"-er\" -- a real rule the same source covers, but not one of the five tabled here -- honest abstention, never invented: {out}"
    );
}

#[test]
fn every_rule_answer_carries_the_citation_shape_its_sentences_support() {
    for (rule, desc, kind, spans) in ROWS {
        let out = ask(&format!("row_{desc}"), &format!("comparative_adjective_rule($R, {desc})"));
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {rule}: {out}");
        assert!(out.contains(&format!("\"R\":\"{rule}\"")), "{desc} binds {rule}: {out}");
        assert!(out.contains(&row_citation(rule)), "{rule} ({kind}): the measured citation shape: {out}");
        for (other, _, _, other_spans) in ROWS {
            if other != rule {
                for s in other_spans.iter() {
                    assert!(!out.contains(&json(s)), "the {other} sentence must not reach {rule}: {out}");
                }
            }
        }
        if kind == "source" {
            assert!(!out.contains(&json(ENVELOPE)), "the envelope is not primary for a source row ({rule}): {out}");
        }
        assert!(spans.len() == if kind == "source" { 1 } else { 2 }, "{rule} has the measured number of sentences");
    }
}

#[test]
fn the_rows_carry_the_pages_own_dashes() {
    // The header quoted these with the WRONG dashes while calling itself
    // byte-for-byte. The page writes HYPHEN-MINUS in rule 2 and EN DASH before
    // "ow" and "le" in rule 5.
    let (_, _, _, r2) = row("one_syllable_adjective_ending_in_e");
    assert!(r2[0].contains("ends in -e, just add an -r at the end."), "rule 2 uses U+002D twice: {:?}", r2[0]);
    assert!(!r2[0].contains('\u{2013}'), "rule 2 carries no EN DASH: {:?}", r2[0]);
    let (_, _, _, r5) = row("two_syllable_adjective_ending_in_er_ow_or_le");
    assert!(r5[0].contains("ends with -er (like bitter) or \u{2013}ow (like narrow)"), "rule 5 writes -er then EN DASH ow: {:?}", r5[0]);
    assert!(r5[1].contains("ends in \u{2013}le, you can just add \u{2013}r"), "rule 5 writes EN DASH le and r: {:?}", r5[1]);
    let body = shipped_table();
    for bad in ["just add an \u{2013}r", "or -ow (like narrow)", "ends in -le,"] {
        assert!(!body.contains(bad), "the table must not carry the header's old dash {bad:?}");
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    let body = shipped_table();
    let row_sources = body.lines().filter(|l| l.starts_with("        source \"")).count();
    let row_cites = body.lines().filter(|l| l.starts_with("        cites \"")).collect::<Vec<_>>();
    assert_eq!(row_sources, 3, "three row sources (rules 1, 2, 4)");
    assert_eq!(row_cites.len(), 4, "four row corroborations (rules 3 and 5, two each)");
    assert!(
        row_cites.iter().all(|l| l.ends_with(&format!(" locator \"{LOCATOR}\""))),
        "every corroboration is at the article"
    );
    assert_eq!(
        body.lines().filter(|l| l.trim_start().starts_with("source \"")).count(),
        4,
        "three row sources and one envelope, at any indentation"
    );
    assert!(!body.lines().any(|l| l.starts_with("    cites ")), "no table-level corroboration");
    assert!(
        !body.lines().any(|l| l.starts_with("        locator ") || l.starts_with("        trust ")),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n")),
        "the envelope is the framing sentence at the article"
    );
    assert!(!body.contains(&format!("\n    source \"{OLD_ENVELOPE}\"\n    locator")), "not rule 1's sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["syllable", "consonant", "-e", "-y", "\u{2013}er", "\u{2013}le", "double"] {
        assert!(!folded.contains(word), "the envelope must name no rule, but contains {word:?}");
    }
    for (rule, desc, kind, spans) in ROWS {
        let mut expected = format!("    row ({rule}, {desc}) {{\n");
        for s in spans.iter() {
            if kind == "source" {
                expected.push_str(&format!("        source \"{}\"\n", json(s)));
            } else {
                expected.push_str(&format!("        cites \"{}\" locator \"{LOCATOR}\"\n", json(s)));
            }
        }
        expected.push_str("    }");
        assert!(body.contains(&expected), "row ({rule}, {desc}) is shipped in its measured shape");
    }
}
