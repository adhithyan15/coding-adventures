//! End-to-end test for the BIOLOGY FACTS library
//! (`adj-facts-stdlib/biology/dna-base-pairs.adj`) driven through the built CLI:
//! a native `table` of DNA base → Watson–Crick complement resolves a binding
//! query recall with the source's citation, and abstains on `uracil` (an RNA
//! base, deliberately not a DNA row) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsdna_{tag}_{}", std::process::id()));
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
fn biology_dna_complement_recall_binds_pair_with_citation() {
    let dir = scratch("dna");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/dna-base-pairs.adj");
    std::fs::copy(&src, dir.join("dna-base-pairs.adj")).expect("copy shipped dna-base-pairs.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"dna-base-pairs.adj\"\n\
         ? dna_complement(adenine, $Base)\n\
         ? dna_complement(guanine, $Base)\n\
         ? dna_complement(uracil, $Base)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Watson–Crick pairing: adenine binds thymine; guanine binds cytosine.
    assert!(out.contains("\"Base\":\"thymine\""), "adenine → thymine: {out}");
    assert!(out.contains("\"Base\":\"cytosine\""), "guanine → cytosine: {out}");
    // The answer carries the NHGRI genome.gov citation as its proof.
    assert!(
        out.contains("genome.gov/genetics-glossary/Base-Pair")
            && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Uracil is an RNA base, not a DNA row — honest abstention, never a
    // fabricated partner.
    assert!(out.contains("\"abstained\":true"), "uracil abstains: {out}");
}
