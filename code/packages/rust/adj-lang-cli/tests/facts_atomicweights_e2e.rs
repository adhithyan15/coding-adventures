//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/atomic-weights.adj`) driven through the built CLI:
//! a native `table` of element → standard atomic weight resolves a binding-query
//! recall with the source's citation, and abstains on a non-element — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsw_{tag}_{}", std::process::id()));
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
fn chemistry_atomic_weight_recall_binds_weight_with_citation() {
    let dir = scratch("atomicweights");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/atomic-weights.adj");
    std::fs::copy(&src, dir.join("atomic-weights.adj")).expect("copy shipped atomic-weights.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"atomic-weights.adj\"\n\
         ? atomic_weight(carbon, $W)\n\
         ? atomic_weight(oxygen, $W)\n\
         ? atomic_weight(adamantium, $W)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Carbon's standard atomic weight is the abridged conventional value 12.011,
    // the leading number of CIAAW's verbatim "12.011 ± 0.002"; oxygen is 15.999.
    assert!(out.contains("\"W\":\"12.011\""), "carbon -> 12.011: {out}");
    assert!(out.contains("\"W\":\"15.999\""), "oxygen -> 15.999: {out}");
    // The answer carries the CIAAW citation as its proof, at authoritative trust.
    assert!(
        out.contains("ciaaw.org/abridged-atomic-weights.htm")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // "adamantium" is not a chemical element — honest abstention, never a
    // fabricated weight.
    assert!(out.contains("\"abstained\":true"), "adamantium abstains: {out}");
}
