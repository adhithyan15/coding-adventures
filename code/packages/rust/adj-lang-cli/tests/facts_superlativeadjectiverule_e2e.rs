//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/superlative-adjective-rule.adj`) driven
//! through the built CLI: a native `table` naming three common English
//! superlative-adjective formation rules and what each actually requires,
//! quoted verbatim from Grammarly's "What Are Superlative Adjectives?
//! Definition and Examples" article. 0 answer-time model calls.

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
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
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
