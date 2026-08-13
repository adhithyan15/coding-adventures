//! End-to-end test for the BIOLOGY FACTS library
//! (`adj-facts-stdlib/biology/start-codon.adj`) driven through the built CLI:
//! a native `table` naming which mRNA codons NCBI's Standard Genetic Code
//! table (translation table 1) marks with an `M` on its `Starts` line -- a
//! sibling to the already-shipped `genetic-code.adj`, decoding a line of the
//! SAME already-quoted artifact that table's own schema had no room for. A
//! binding-query recall returns the role with the NCBI citation, and
//! abstains on a codon the source's `Starts` line does not flag -- 0 model
//! calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_startcodon_{tag}_{}", std::process::id()));
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

fn place_lib(dir: &Path) {
    let src = facts_stdlib().join("biology/start-codon.adj");
    std::fs::copy(&src, dir.join("start-codon.adj")).expect("copy shipped start-codon.adj");
}

#[test]
fn start_codon_recall_binds_the_primary_start_codon() {
    let dir = scratch("primary");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"start-codon.adj\"\n\
         ? start_codon(atg, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Role\":\"start\""),
        "atg is flagged as a start codon: {out}"
    );
    assert!(
        out.contains("ncbi.nlm.nih.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NCBI citation: {out}"
    );
}

#[test]
fn start_codon_recall_binds_both_alternative_start_codons() {
    let dir = scratch("alt");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"start-codon.adj\"\n\
         ? start_codon(ttg, $Role)\n\
         ? start_codon(ctg, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"start_codon(ttg, start)\""),
        "ttg is a flagged alternative start codon: {out}"
    );
    assert!(
        out.contains("\"term\":\"start_codon(ctg, start)\""),
        "ctg is a flagged alternative start codon: {out}"
    );
}

#[test]
fn start_codon_abstains_honestly_on_an_unflagged_codon() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"start-codon.adj\"\n\
         ? start_codon(gcc, $Role)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "gcc -> ? has no shipped row -- honest abstention, never invented: {out}"
    );
}
