//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/newton-laws.adj`) driven through the built CLI: a
//! native `table` of law number → short name resolves a binding-query recall
//! with the NASA "Newton's Laws of Motion" citation, runs the relation backward
//! (name → number), and abstains on a law number the source does not fix (there
//! is no fourth law of motion) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsnewton_{tag}_{}", std::process::id()));
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

#[test]
fn physics_newton_laws_recall_binds_name_with_citation() {
    let dir = scratch("newtonlaws");
    // Copy the shipped physics table beside the entry program and import it.
    let src = facts_stdlib().join("physics/newton-laws.adj");
    std::fs::copy(&src, dir.join("newton-laws.adj")).expect("copy shipped newton-laws.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"newton-laws.adj\"\n\
         ? newton_law(1, $Name)\n\
         ? newton_law(2, $Name)\n\
         ? newton_law(3, $Name)\n\
         ? newton_law($N, action_reaction)\n\
         ? newton_law(4, $Name)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each law NUMBER to the short name NASA labels it with.
    assert!(out.contains("\"Name\":\"inertia\""), "law 1 → inertia: {out}");
    assert!(out.contains("\"Name\":\"force\""), "law 2 → force: {out}");
    assert!(
        out.contains("\"Name\":\"action_reaction\""),
        "law 3 → action_reaction: {out}"
    );
    // The relation runs BACKWARD: the name action_reaction recalls number 3.
    assert!(
        out.contains("\"N\":\"3\""),
        "action_reaction → 3 (reverse recall): {out}"
    );
    // The answer carries the NASA locator + trust tier as its proof.
    assert!(
        out.contains("grc.nasa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // There is no fourth law of motion — honest abstention, never a fabricated law.
    assert!(
        out.contains("\"abstained\":true"),
        "ungrounded law number abstains: {out}"
    );
}
