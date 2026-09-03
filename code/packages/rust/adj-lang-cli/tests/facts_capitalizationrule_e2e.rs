//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/capitalization-rule.adj`) driven through the
//! built CLI: a native `table` naming three common English capitalization
//! rules and what each actually requires, quoted verbatim from Grammarly's
//! "Capitalization Rules and Examples" article. 0 answer-time model calls.

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
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
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
