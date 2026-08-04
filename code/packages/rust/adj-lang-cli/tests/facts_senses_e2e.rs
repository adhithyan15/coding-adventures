//! End-to-end test for the biology FIVE-SENSES facts library
//! (`adj-facts-stdlib/biology/five-senses.adj`) driven through the built CLI:
//! a native `table` of sense → body organ resolves a binding-query recall with
//! the source's citation, and abstains on a non-sense — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_senses_{tag}_{}", std::process::id()));
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
fn biology_five_senses_recall_binds_organ_with_citation() {
    let dir = scratch("fivesenses");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/five-senses.adj");
    std::fs::copy(&src, dir.join("five-senses.adj")).expect("copy shipped five-senses.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"five-senses.adj\"\n\
         ? sense_organ(sight, $Organ)\n\
         ? sense_organ(taste, $Organ)\n\
         ? sense_organ(balance, $Organ)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // You see with your eyes and taste with your tongue — the recalled organs.
    assert!(out.contains("\"Organ\":\"eyes\""), "sight → eyes: {out}");
    assert!(out.contains("\"Organ\":\"tongue\""), "taste → tongue: {out}");
    // The answer carries the KidsHealth citation as its proof, at consensus trust.
    assert!(
        out.contains("kidshealth.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // "balance" is not one of the five senses — honest abstention, never a
    // fabricated organ.
    assert!(out.contains("\"abstained\":true"), "balance abstains: {out}");
}
