//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/elements.adj`) driven through the built CLI:
//! a native `table` of element → atomic-number resolves a binding-query recall
//! with the source's citation, runs the relation backward (number → element),
//! and abstains on a non-element — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsg_{tag}_{}", std::process::id()));
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
fn chemistry_elements_recall_binds_atomic_number_with_citation() {
    let dir = scratch("elements");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/elements.adj");
    std::fs::copy(&src, dir.join("elements.adj")).expect("copy shipped elements.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"elements.adj\"\n\
         ? atomic_number(hydrogen, $Z)\n\
         ? atomic_number(oxygen, $Z)\n\
         ? atomic_number($E, 6)\n\
         ? atomic_number(kryptonite, $Z)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Hydrogen is element 1; oxygen is element 8 — the recalled atomic numbers.
    assert!(out.contains("\"Z\":\"1\""), "hydrogen → 1: {out}");
    assert!(out.contains("\"Z\":\"8\""), "oxygen → 8: {out}");
    // The relation runs backward: atomic number 6 recalls carbon.
    assert!(out.contains("\"E\":\"carbon\""), "6 → carbon (reverse recall): {out}");
    // The answer carries the PubChem citation as its proof.
    assert!(
        out.contains("pubchem.ncbi.nlm.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // "kryptonite" is not a chemical element — honest abstention, never a
    // fabricated atomic number.
    assert!(out.contains("\"abstained\":true"), "non-element abstains: {out}");
}
