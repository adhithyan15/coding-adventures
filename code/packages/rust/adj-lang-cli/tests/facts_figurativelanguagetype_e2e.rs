//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/figurative-language-type.adj`) driven
//! through the built CLI: a native `table` naming three figures of speech
//! and what each actually does, quoted verbatim from Grammarly's
//! "Figurative Language Examples: 6 Common Types and Definitions"
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
    let dir = std::env::temp_dir().join(format!("adjcli_figurative_language_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/figurative-language-type.adj");
    std::fs::copy(&src, dir.join("figurative-language-type.adj")).expect("copy shipped figurative-language-type.adj");
}

#[test]
fn figurative_language_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"figurative-language-type.adj\"\n\
         ? figurative_language_type(hyperbole, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"a_great_exaggeration_used_to_add_emphasis\""),
        "hyperbole means a_great_exaggeration_used_to_add_emphasis: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn figurative_language_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"figurative-language-type.adj\"\n\
         ? figurative_language_type($T, gives_human_characteristics_to_nonhuman_or_abstract_things)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"personification\""),
        "the shipped gives_human_characteristics_to_nonhuman_or_abstract_things example is personification: {out}"
    );
}

#[test]
fn figurative_language_type_abstains_honestly_on_an_untabled_type() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"figurative-language-type.adj\"\n\
         ? figurative_language_type(allusion, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "allusion is a real device the source names with its own clean sentence, but one that works by referencing an external work/person/event, a different mechanism than the three tabled here -- honest abstention, never invented: {out}"
    );
}
