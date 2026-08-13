//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/pronoun-type.adj`) driven through the
//! built CLI: a native `table` naming four pronoun types and what each
//! actually is, quoted verbatim from Grammarly's "Pronouns: Definition
//! and Examples" article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_pronoun_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/pronoun-type.adj");
    std::fs::copy(&src, dir.join("pronoun-type.adj")).expect("copy shipped pronoun-type.adj");
}

#[test]
fn pronoun_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pronoun-type.adj\"\n\
         ? pronoun_type(interrogative_pronoun, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"used_in_questions\""),
        "interrogative_pronoun means used_in_questions: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn pronoun_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pronoun-type.adj\"\n\
         ? pronoun_type($T, changes_form_based_on_grammatical_person)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"personal_pronoun\""),
        "the shipped changes_form_based_on_grammatical_person example is personal_pronoun: {out}"
    );
}

#[test]
fn pronoun_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pronoun-type.adj\"\n\
         ? pronoun_type(relative_pronoun, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "relative_pronoun is a real pronoun type the source covers but not one of the four tabled here -- honest abstention, never invented: {out}"
    );
}

#[test]
fn pronoun_type_extension_recalls_the_newly_added_distributive_pronoun() {
    let dir = scratch("ext");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"pronoun-type.adj\"\n\
         ? pronoun_type(distributive_pronoun, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("pronoun_type(distributive_pronoun, refers_to_nouns_as_individual_elements_of_larger_groups)"),
        "distributive_pronoun refers to nouns as individual elements of larger groups (added this cycle): {out}"
    );
}
