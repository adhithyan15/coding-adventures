//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/amino-acids.adj`) driven through the built CLI:
//! a native `table` of amino acid -> one-letter code resolves binding-query
//! recall in BOTH directions with the source's (DDBJ) citation, and abstains on
//! a non-standard amino acid — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsaa_{tag}_{}", std::process::id()));
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
fn biology_amino_acid_code_recall_binds_both_directions_with_citation() {
    let dir = scratch("aminoacids");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/amino-acids.adj");
    std::fs::copy(&src, dir.join("amino-acids.adj")).expect("copy shipped amino-acids.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"amino-acids.adj\"\n\
         ? amino_acid_code(glycine, $C)\n\
         ? amino_acid_code($A, a)\n\
         ? amino_acid_code(selenocysteine, $C)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward recall: glycine -> g (the name binds the one-letter code).
    assert!(out.contains("\"C\":\"g\""), "glycine -> g: {out}");
    // Reverse recall: the code a binds the amino-acid name alanine.
    assert!(out.contains("\"A\":\"alanine\""), "a -> alanine: {out}");
    // The answer carries the DDBJ citation as its proof.
    assert!(
        out.contains("ddbj.nig.ac.jp") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Selenocysteine (Sec/U) is the 21st amino acid, not one of the standard
    // twenty — honest abstention, never a fabricated code.
    assert!(out.contains("\"abstained\":true"), "selenocysteine abstains: {out}");
}
