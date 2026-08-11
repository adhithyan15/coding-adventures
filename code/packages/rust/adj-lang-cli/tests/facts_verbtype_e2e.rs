//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/verb-type.adj`) driven through the built
//! CLI: a native `table` naming three verb types and what each actually
//! is, quoted verbatim from Grammarly's "Verbs: Definition and Examples"
//! article. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_verb_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/verb-type.adj");
    std::fs::copy(&src, dir.join("verb-type.adj")).expect("copy shipped verb-type.adj");
}

#[test]
fn verb_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"verb-type.adj\"\n\
         ? verb_type(action_verb, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"physical_action_or_activity_that_can_be_seen_or_heard\""),
        "action_verb means physical_action_or_activity_that_can_be_seen_or_heard: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn verb_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"verb-type.adj\"\n\
         ? verb_type($T, changes_another_verbs_tense_voice_or_mood)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"auxiliary_verb\""),
        "the shipped changes_another_verbs_tense_voice_or_mood example is auxiliary_verb: {out}"
    );
}

#[test]
fn verb_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"verb-type.adj\"\n\
         ? verb_type(transitive_verb, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "transitive_verb is a real verb type the source covers but not one of the three tabled here -- honest abstention, never invented: {out}"
    );
}
