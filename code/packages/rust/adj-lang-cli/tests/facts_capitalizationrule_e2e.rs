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
