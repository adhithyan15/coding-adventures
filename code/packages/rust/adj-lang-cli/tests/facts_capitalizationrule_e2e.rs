//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/capitalization-rule.adj`) driven through the
//! built CLI: a native `table` naming three common English capitalization
//! rules and what each actually requires, quoted verbatim from Grammarly's
//! "Capitalization Rules and Examples" article. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the first-word rule's own sentence, the primary source of all three
//! answers; it is now the article's framing sentence, which every row
//! overrides. Each row's citation is pinned whole.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_capitalization_rule_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/capitalization-rule.adj");
    std::fs::copy(&src, dir.join("capitalization-rule.adj"))
        .expect("copy shipped capitalization-rule.adj");
}

const LOCATOR: &str = "https://www.grammarly.com/blog/punctuation-capitalization/capitalization-rules/";
const ENVELOPE: &str = "Knowing which types of words to capitalize is an important part of learning English capitalization rules.";
const FIRST_WORD: &str = "Here\u{2019}s an easy rule to follow\u{2014}whenever you start a sentence, capitalize the first letter of the first word.";
const PRONOUN_I: &str = "Capitalize I when using it as a pronoun anywhere in a sentence.";
const PROPER_NOUN: &str = "Proper nouns are always capitalized in English, no matter where they fall in a sentence.";
/// The page's OTHER statement of the proper-noun rule, in a list item. A
/// different string; the row quotes the paragraph form above.
const PROPER_NOUN_LIST_ITEM: &str = "Proper nouns (specific names for a particular person, place, or thing) are always capitalized in English, no matter where they fall in a sentence.";

/// (rule, description, that row's own sentence)
const ROWS: [(&str, &str, &str); 3] = [
    ("first_word_of_sentence", "capitalize_first_letter", FIRST_WORD),
    ("pronoun_i", "capitalized_anywhere_in_sentence", PRONOUN_I),
    ("proper_noun", "capitalized_regardless_of_position", PROPER_NOUN),
];

/// The whole citation a row's answer carries: its sentence, the article, the
/// tier and NO corroborations, as one contiguous run.
fn citation(sentence: &str) -> String {
    format!("\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("language/capitalization-rule.adj"))
        .expect("read shipped capitalization-rule.adj");
    adj[adj.find("table capitalization_rule").expect("table")..].to_string()
}

#[test]
fn capitalization_rule_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"capitalization-rule.adj\"\n\
         ? capitalization_rule(pronoun_i, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"capitalized_anywhere_in_sentence\""),
        "pronoun_i means capitalized_anywhere_in_sentence: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("grammarly.com") && contains(trust)` (#15209).
    assert!(out.contains(&citation(PRONOUN_I)), "carries the pronoun-I sentence, whole: {out}");
}

#[test]
fn capitalization_rule_reverse_binds_the_rule_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"capitalization-rule.adj\"\n\
         ? capitalization_rule($R, capitalize_first_letter)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"R\":\"first_word_of_sentence\""),
        "the shipped capitalize_first_letter example is first_word_of_sentence: {out}"
    );
}

#[test]
fn capitalization_rule_abstains_honestly_on_an_untabled_rule() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"capitalization-rule.adj\"\n\
         ? capitalization_rule(quotation, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "quotation is a real capitalization rule the source covers but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}

const CAPITALIZATION_RULE_PIN: &str = r#""bindings":{"R":"first_word_of_sentence"},"citations":[{"source":"Here’s an easy rule to follow—whenever you start a sentence, capitalize the first letter of the first word.","locator":"https://www.grammarly.com/blog/punctuation-capitalization/capitalization-rules/","trust":"consensus""#;

#[test]
fn capitalization_rule_citation_is_the_pages_whole_sentence() {
    let dir = scratch("reground");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"capitalization-rule.adj\"
? capitalization_rule($R, capitalize_first_letter)
",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The shipped value was a FRAGMENT PUNCTUATED INTO A SENTENCE: the page's
    // wording, with one character changed so it would read as standalone. It
    // therefore appeared on no page. Every quote-keyed screen passed it,
    // because the quotes were all correct.
    //
    // THIS QUERY IS THE REVERSE ONE. The companion's FIRST query asks pronoun_i,
    // and this envelope -- about capitalizing the first letter of a sentence --
    // does not ground that row. Reading the bindings beside the source caught it.
    assert!(
        out.contains(CAPITALIZATION_RULE_PIN),
        "the first-word rule citation is the page's own sentence: {out}"
    );
}

#[test]
fn every_rule_answer_carries_its_own_sentence() {
    for (rule, desc, sentence) in ROWS {
        let dir = scratch(&format!("row_{desc}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"capitalization-rule.adj\"\n? capitalization_rule($R, {desc})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {rule}: {out}");
        assert!(out.contains(&format!("\"R\":\"{rule}\"")), "{desc} binds {rule}: {out}");
        assert!(out.contains(&citation(sentence)), "{rule}: its own sentence, whole: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != rule {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {rule}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {rule}: {out}");
    }
}

#[test]
fn the_proper_noun_row_carries_the_paragraph_form_not_the_list_item() {
    let body = shipped_table();
    assert!(!body.contains("(specific names"), "the list item's parenthetical form is not the row's sentence");
    assert!(
        !PROPER_NOUN.contains('(') && PROPER_NOUN_LIST_ITEM.contains("(specific names"),
        "the two forms differ by the parenthetical"
    );
    let dir = scratch("proper_noun_form");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"capitalization-rule.adj\"\n? capitalization_rule(proper_noun, $D)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(!out.contains(PROPER_NOUN_LIST_ITEM), "the list item's form reaches no answer: {out}");
    assert!(out.contains(&citation(PROPER_NOUN)), "the paragraph form is the proper-noun citation: {out}");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (rule, desc, sentence) in ROWS {
        let expected = format!("    row ({rule}, {desc}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({rule}, {desc}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("\n        cites "), "no row corroboration");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n")),
        "the envelope is the article's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{FIRST_WORD}\"\n    locator")), "not the first-word sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["first word", "first letter", "pronoun", "proper noun", "anywhere", "no matter where"] {
        assert!(!folded.contains(word), "the envelope must name no rule, but contains {word:?}");
    }
}
