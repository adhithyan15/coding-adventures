//! End-to-end test for the BIOLOGY FACTS library
//! (`adj-facts-stdlib/biology/genetic-code.adj`) driven through the built CLI:
//! a native `table` of the STANDARD GENETIC CODE (NCBI translation table 1) maps
//! each mRNA codon to its amino acid. A binding-query recall returns the amino
//! acid with the NCBI citation, and abstains on a triplet that is not a real
//! codon (`xyz`) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsgc_{tag}_{}", std::process::id()));
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
fn biology_genetic_code_recall_binds_amino_acid_with_citation() {
    let dir = scratch("gc");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/genetic-code.adj");
    std::fs::copy(&src, dir.join("genetic-code.adj")).expect("copy shipped genetic-code.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"genetic-code.adj\"\n\
         ? codon_amino_acid(atg, $A)\n\
         ? codon_amino_acid(taa, $A)\n\
         ? codon_amino_acid(gag, $A)\n\
         ? codon_amino_acid(xyz, $A)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Standard genetic code: AUG (atg) codes methionine (also START);
    // a third-base wobble / sense-vs-stop decode is what a mutation reasoner reads.
    assert!(out.contains("\"A\":\"m\""), "atg → m (methionine): {out}");
    // A STOP codon terminates translation — maps to the atom `stop`.
    assert!(out.contains("\"A\":\"stop\""), "taa → stop: {out}");
    // Glutamate — the sickle-cell wild-type codon (gag e → gtg v is the mutation).
    assert!(out.contains("\"A\":\"e\""), "gag → e (glutamate): {out}");
    // The answer carries the NCBI translation-table citation as its proof.
    assert!(
        out.contains("ncbi.nlm.nih.gov/Taxonomy/Utils/wprintgc.cgi")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the NCBI source citation: {out}"
    );
    // `xyz` is not a codon — honest abstention, never a fabricated amino acid.
    assert!(out.contains("\"abstained\":true"), "xyz abstains: {out}");
}
