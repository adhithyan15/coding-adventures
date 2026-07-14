//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/element-symbols.adj`) driven through the built
//! CLI: a native `table` of element -> chemical symbol resolves binding-query
//! recall in BOTH directions with the source's citation, and abstains on a
//! non-element — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsym_{tag}_{}", std::process::id()));
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
fn chemistry_element_symbol_recall_binds_both_directions_with_citation() {
    let dir = scratch("elemsym");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/element-symbols.adj");
    std::fs::copy(&src, dir.join("element-symbols.adj")).expect("copy shipped element-symbols.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"element-symbols.adj\"\n\
         ? element_symbol(oxygen, $S)\n\
         ? element_symbol($E, na)\n\
         ? element_symbol(unobtainium, $S)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward recall: oxygen -> o (name binds the symbol).
    assert!(out.contains("\"S\":\"o\""), "oxygen -> o: {out}");
    // Reverse recall: the symbol na binds the element name sodium.
    assert!(out.contains("\"E\":\"sodium\""), "na -> sodium: {out}");
    // The answer carries the PubChem citation as its proof.
    assert!(
        out.contains("pubchem.ncbi.nlm.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // unobtainium is not a real element in the table — honest abstention.
    assert!(out.contains("\"abstained\":true"), "unobtainium abstains: {out}");
}
