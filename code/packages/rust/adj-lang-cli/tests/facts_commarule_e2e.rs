//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/comma-rule.adj`) driven through the built
//! CLI: a native `table` naming three comma rules and what each actually
//! says to do, quoted verbatim from Grammarly's "Rules for Using Commas,
//! With Examples" article. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the series row's own sentence, the primary source of all three answers; it
//! is now the article's framing sentence, which every row overrides. Each
//! row's citation is pinned whole, and the "but" sentence carries the page's
//! closing colon (the header had quoted it with a period).

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_comma_rule_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/comma-rule.adj");
    std::fs::copy(&src, dir.join("comma-rule.adj")).expect("copy shipped comma-rule.adj");
}

const LOCATOR: &str = "https://www.grammarly.com/blog/punctuation-capitalization/comma/";
const ENVELOPE: &str = "There are lots of rules about comma usage, and often the factors that determine whether you should use one are quite subtle.";
const SERIES: &str = "When you have a list that contains more than two elements, use commas to separate them.";
const BUT: &str = "Use a comma before the coordinating conjunction but if it is joining two independent clauses:";
const ADDRESS: &str = "When addressing another person by name, set off the name with commas.";

/// (rule, description, that row's own sentence)
const ROWS: [(&str, &str, &str); 3] = [
    ("comma_in_a_series", "use_commas_to_separate_elements_in_a_list_of_more_than_two_elements", SERIES),
    ("comma_before_but", "use_a_comma_before_but_when_it_is_joining_two_independent_clauses", BUT),
    ("comma_with_direct_address", "set_off_the_name_with_commas_when_addressing_another_person_by_name", ADDRESS),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(sentence: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"consensus\",\"corroborations\":[]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("language/comma-rule.adj"))
        .expect("read shipped comma-rule.adj");
    adj[adj.find("table comma_rule").expect("table")..].to_string()
}

#[test]
fn comma_rule_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comma-rule.adj\"\n\
         ? comma_rule(comma_in_a_series, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"use_commas_to_separate_elements_in_a_list_of_more_than_two_elements\""),
        "comma_in_a_series means use_commas_to_separate_elements_in_a_list_of_more_than_two_elements: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("grammarly.com") && contains(trust)` (#15209).
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(out.contains(&only_citation(SERIES)), "the series sentence is the only citation: {out}");
}

#[test]
fn comma_rule_reverse_binds_the_rule_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comma-rule.adj\"\n\
         ? comma_rule($R, set_off_the_name_with_commas_when_addressing_another_person_by_name)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"R\":\"comma_with_direct_address\""),
        "the shipped set_off_the_name_with_commas example is comma_with_direct_address: {out}"
    );
}

#[test]
fn comma_rule_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"comma-rule.adj\"\n\
         ? comma_rule(oxford_comma, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "oxford_comma is a real, well-known term, but its own rule sentence bundles the rule with an optionality caveat rather than one clean fact -- honest abstention, never invented: {out}"
    );
}

#[test]
fn every_rule_answer_carries_its_own_sentence() {
    for (rule, desc, sentence) in ROWS {
        let dir = scratch(&format!("row_{rule}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"comma-rule.adj\"\n? comma_rule($R, {desc})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {rule}: {out}");
        assert!(out.contains(&format!("\"R\":\"{rule}\"")), "{desc} binds {rule}: {out}");
        assert!(out.contains(&only_citation(sentence)), "{rule}: its own sentence, whole, and the only citation: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != rule {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {rule}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {rule}: {out}");
    }
}

#[test]
fn the_but_row_carries_the_pages_closing_colon() {
    // The header quoted this sentence ending in a PERIOD while calling it
    // verbatim; on the page it ends in a colon that introduces an example
    // pair, and the period form occurs zero times.
    let body = shipped_table();
    assert!(
        body.contains("        source \"Use a comma before the coordinating conjunction but if it is joining two independent clauses:\"\n"),
        "the shipped but row ends in the page's colon"
    );
    let adj = std::fs::read_to_string(facts_stdlib().join("language/comma-rule.adj")).unwrap();
    assert!(
        !adj.contains("joining two independent clauses.\""),
        "no period form anywhere in the file, header included"
    );
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
    assert!(!body.contains("cites "), "no corroboration at row or table level");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust consensus\n")),
        "the envelope is the article's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{SERIES}\"\n    locator")), "not the series sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["series", "list", "element", "separate", " but ", "clause", "conjunction", "address", "name"] {
        assert!(!folded.contains(word), "the envelope must name no rule or description, but contains {word:?}");
    }
}
