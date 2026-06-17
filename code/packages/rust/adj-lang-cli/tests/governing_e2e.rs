//! End-to-end test for the ADJ73 `governing` section (defeasible precedence) through the
//! built CLI binary. A binding query gets a precedence-resolved view alongside `recall`:
//! every distinct answer tagged `governing` / `defeated` / `conflict_peer` + its standing.
//!
//! NOTE: the `functional` / `priority:` SURFACE syntax (adj-lang PR-C) is a separate PR; until
//! it merges this binary can't parse those, so this test covers the engine-backed render for a
//! non-functional predicate (every answer governs — the back-compat baseline). The functional
//! override path is proven at the engine + adj-lang lowering level (logic-engine `govern` tests
//! + adj-lang `functional_and_priority_tiers_resolve_a_conflict`).

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_gov_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(program: &PathBuf) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(program)
        .output()
        .expect("run adj-lang-cli");
    (out.status.success(), String::from_utf8(out.stdout).unwrap())
}

#[test]
fn governing_section_tags_every_binding_answer() {
    let dir = scratch("basic");
    std::fs::write(
        dir.join("case.adj"),
        "relate deficient_in(tay_sachs, hexosaminidase_a) trust authoritative\n\
         relate deficient_in(gaucher, glucocerebrosidase) trust authoritative\n\
         ? deficient_in($Disease, $Enzyme)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The governing section is present and parallels recall.
    assert!(out.contains("\"governing\""), "has a governing section: {out}");
    // A non-functional predicate ⇒ every answer governs, none defeated, no conflict.
    assert!(out.contains("\"status\":\"governing\""), "answers are governing: {out}");
    assert!(!out.contains("\"status\":\"defeated\""), "nothing defeated: {out}");
    assert!(out.contains("\"has_conflict\":false"), "no conflict: {out}");
    // Bindings + the ground answer term are carried.
    assert!(out.contains("\"Enzyme\":\"hexosaminidase_a\""), "carries bindings: {out}");
    assert!(
        out.contains("deficient_in(tay_sachs, hexosaminidase_a)"),
        "carries the ground answer term: {out}"
    );
}

#[test]
fn governing_section_absent_when_no_binding_query() {
    let dir = scratch("none");
    // A ground hypothesis query (no `$var`) is not a binding query → no governing section.
    std::fs::write(dir.join("case.adj"), "prior 0.1 for acs\n? acs\n").unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(!out.contains("\"governing\""), "no governing section for a ground query: {out}");
}
