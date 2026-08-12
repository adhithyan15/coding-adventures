//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/comma-rule.adj`) driven through the built
//! CLI: a native `table` naming three comma rules and what each actually
//! says to do, quoted verbatim from Grammarly's "Rules for Using Commas,
//! With Examples" article. 0 answer-time model calls.

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
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
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
