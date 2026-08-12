//! End-to-end test for the language FACTS library
//! (`adj-facts-stdlib/language/clause-type.adj`) driven through the built
//! CLI: a native `table` naming the two structural kinds of clause and
//! what makes a clause one or the other, quoted verbatim from Grammarly's
//! "Independent and Dependent Clauses" article. 0 answer-time model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_clause_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("language/clause-type.adj");
    std::fs::copy(&src, dir.join("clause-type.adj")).expect("copy shipped clause-type.adj");
}

#[test]
fn clause_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"clause-type.adj\"\n\
         ? clause_type(independent_clause, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"is_a_clause_that_alone_is_a_complete_sentence\""),
        "independent_clause means is_a_clause_that_alone_is_a_complete_sentence: {out}"
    );
    assert!(
        out.contains("grammarly.com") && out.contains("\"trust\":\"consensus\""),
        "carries the Grammarly citation: {out}"
    );
}

#[test]
fn clause_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"clause-type.adj\"\n\
         ? clause_type($T, is_a_clause_that_alone_is_not_a_complete_sentence)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"dependent_clause\""),
        "the shipped is_a_clause_that_alone_is_not_a_complete_sentence example is dependent_clause: {out}"
    );
}

#[test]
fn clause_type_abstains_honestly_on_an_untabled_term() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"clause-type.adj\"\n\
         ? clause_type(noun_clause, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "noun_clause is a real clause category, but it names a functional role, not a structural independent/dependent type -- honest abstention, never invented: {out}"
    );
}
