//! End-to-end tests for relational recall (MYCIN-2026 REL-3) through the built
//! CLI binary: a `relate`-edge knowledge graph + a `$variable` binding query
//! resolves to a `"recall"` section carrying the bindings AND the citing edge's
//! provenance, with 0 answer-time model calls. Abstention (no grounded edge) is
//! an explicit empty answer set, not a fabricated value.

use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_recall_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, src: &str) {
    std::fs::write(dir.join(name), src).unwrap();
}

fn run(program: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn forward_recall_binds_the_enzyme_with_a_citation() {
    let dir = scratch("forward");
    write(
        &dir,
        "case.adj",
        "relate deficient_in(tay_sachs, hexosaminidase_a)\n\
             source \"Tay-Sachs results from deficient hexosaminidase A.\"\n\
             trust authoritative\n\
         relate deficient_in(gaucher, glucocerebrosidase) trust authoritative\n\
         ? deficient_in(tay_sachs, $Enzyme)\n",
    );
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Enzyme\":\"hexosaminidase_a\""),
        "binds the enzyme: {out}"
    );
    // The answer carries its proof — the citing edge's source.
    assert!(
        out.contains("deficient hexosaminidase A"),
        "carries the citation: {out}"
    );
    assert!(
        out.contains("\"abstained\":false"),
        "not an abstention: {out}"
    );
}

#[test]
fn recall_abstains_on_an_ungrounded_disease() {
    let dir = scratch("abstain");
    write(
        &dir,
        "case.adj",
        "relate deficient_in(tay_sachs, hexosaminidase_a) trust authoritative\n\
         ? deficient_in(niemann_pick, $Enzyme)\n",
    );
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // No grounded edge → empty answers → explicit abstention, no fabricated enzyme.
    assert!(out.contains("\"abstained\":true"), "must abstain: {out}");
    assert!(
        out.contains("\"answers\":[]"),
        "no fabricated answer: {out}"
    );
}

#[test]
fn binding_query_under_a_dictionary_use_scope_is_accepted() {
    // A relation-typed query inside a `use`d vocabulary must not be rejected as a
    // non-hypothesis (REL-3 enforce_vocabulary fix).
    let dir = scratch("scoped");
    write(
        &dir,
        "case.adj",
        "dictionary bio {\n\
             define disease : entity\n\
             define enzyme : entity\n\
             define deficient_in : relation from disease to enzyme\n\
         }\n\
         use bio\n\
         relate deficient_in(tay_sachs, hexosaminidase_a) trust authoritative\n\
         ? deficient_in(tay_sachs, $Enzyme)\n",
    );
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed (no UndefinedTerm): {out}");
    assert!(
        out.contains("\"Enzyme\":\"hexosaminidase_a\""),
        "binds under use-scope: {out}"
    );
}
