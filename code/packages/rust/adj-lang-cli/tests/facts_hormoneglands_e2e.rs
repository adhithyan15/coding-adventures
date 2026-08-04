//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/hormone-glands.adj`) driven through the built CLI:
//! a native `table` mapping each hormone → the endocrine gland that secretes it
//! resolves binding-query recalls (forward AND backward) with the source's NCI
//! SEER Training Modules citation, and abstains on a hormone that is not one of
//! the grounded rows (aldosterone) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsh_{tag}_{}", std::process::id()));
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
fn biology_hormone_glands_recall_binds_gland_with_citation() {
    let dir = scratch("hormoneglands");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/hormone-glands.adj");
    std::fs::copy(&src, dir.join("hormone-glands.adj")).expect("copy shipped hormone-glands.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"hormone-glands.adj\"\n\
         ? hormone_gland(insulin, $Gland)\n\
         ? hormone_gland(cortisol, $Gland)\n\
         ? hormone_gland(melatonin, $Gland)\n\
         ? hormone_gland($Hormone, pituitary)\n\
         ? hormone_gland(aldosterone, $Gland)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Insulin comes from the pancreas, cortisol from the adrenal glands, melatonin
    // from the pineal gland — the recalled glands (forward binds).
    assert!(
        out.contains("\"Gland\":\"pancreas\""),
        "insulin → pancreas: {out}"
    );
    assert!(
        out.contains("\"Gland\":\"adrenal_gland\""),
        "cortisol → adrenal_gland: {out}"
    );
    assert!(
        out.contains("\"Gland\":\"pineal_gland\""),
        "melatonin → pineal_gland: {out}"
    );
    // The relation runs BACKWARD: bind the gland `pituitary`, recall the hormone
    // it makes.
    assert!(
        out.contains("\"Hormone\":\"growth_hormone\""),
        "pituitary → growth_hormone (reverse recall): {out}"
    );
    // The answer carries the NCI SEER Training Modules citation as its proof, at
    // the `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("training.seer.cancer.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Aldosterone is a real adrenal hormone but is deliberately NOT a row (its
    // fetched span did not name the adrenal gland) — honest abstention, never a
    // fabricated gland.
    assert!(
        out.contains("\"abstained\":true"),
        "aldosterone abstains: {out}"
    );
}
