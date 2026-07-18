//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/chemical-bonds.adj`) driven through the built
//! CLI: a native `table` of bond type → defining token resolves binding-query
//! recalls (forward and backward) with the source's LibreTexts citation, and
//! abstains on a bond type this source does not pin to a token (hydrogen) —
//! 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factscb_{tag}_{}", std::process::id()));
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
fn chemistry_chemical_bonds_recall_binds_token_with_citation() {
    let dir = scratch("chemicalbonds");
    // Copy the shipped chemistry table beside the entry program and import it.
    let src = facts_stdlib().join("chemistry/chemical-bonds.adj");
    std::fs::copy(&src, dir.join("chemical-bonds.adj")).expect("copy shipped chemical-bonds.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"chemical-bonds.adj\"\n\
         ? chemical_bond_token(ionic, $T)\n\
         ? chemical_bond_token(covalent, $T)\n\
         ? chemical_bond_token(metallic, $T)\n\
         ? chemical_bond_token($B, weak)\n\
         ? chemical_bond_token(hydrogen, $T)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each bond to the token the source uses to define it.
    assert!(out.contains("\"T\":\"transfer\""), "ionic binds to transfer: {out}");
    assert!(
        out.contains("chemical_bond_token(ionic, transfer)"),
        "ionic is governing-bound to transfer: {out}"
    );
    assert!(
        out.contains("chemical_bond_token(covalent, sharing)"),
        "covalent is governing-bound to sharing: {out}"
    );
    assert!(
        out.contains("chemical_bond_token(metallic, gas)"),
        "metallic is governing-bound to gas: {out}"
    );
    // The relation runs BACKWARD: bind the token `weak`, recall the bond type.
    assert!(
        out.contains("chemical_bond_token(van_der_waals, weak)"),
        "reverse recall binds B=van_der_waals from weak: {out}"
    );
    // The answer carries the LibreTexts locator + trust tier as its proof, at
    // consensus trust (a secondary teaching reference, honestly tiered).
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation at consensus trust: {out}"
    );
    // Hydrogen bonding is NOT pinned to a token by this source — honest
    // abstention, never a fabricated token.
    assert!(out.contains("\"abstained\":true"), "hydrogen abstains: {out}");
}
