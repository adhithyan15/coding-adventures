//! End-to-end test for the kindergarten US-coins FACTS library
//! (`adj-facts-stdlib/kindergarten/us-coins.adj`) driven through the built CLI:
//! a native `table` of coin → value-in-cents resolves a binding-query recall
//! with the U.S. Mint's citation, runs the relation backwards (value → coin),
//! and abstains on a non-coin — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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
fn kindergarten_coins_recall_binds_cent_value_with_citation() {
    let dir = scratch("coins");
    // Copy the shipped kindergarten table beside the entry program and import it.
    let src = facts_stdlib().join("kindergarten/us-coins.adj");
    std::fs::copy(&src, dir.join("us-coins.adj")).expect("copy shipped us-coins.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"us-coins.adj\"\n\
         ? coin_cents(quarter, $Cents)\n\
         ? coin_cents(penny, $Cents)\n\
         ? coin_cents($Coin, 5)\n\
         ? coin_cents(doubloon, $Cents)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // A quarter is worth twenty-five cents; a penny is worth one — the recalled values.
    assert!(out.contains("\"Cents\":\"25\""), "quarter → 25: {out}");
    assert!(out.contains("\"Cents\":\"1\""), "penny → 1: {out}");
    // The relation runs backwards: the coin worth five cents is the nickel.
    assert!(out.contains("\"Coin\":\"nickel\""), "5 cents → nickel: {out}");
    // The answer carries the U.S. Mint citation as its proof.
    assert!(
        out.contains("kids.usmint.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the U.S. Mint citation: {out}"
    );
    // A doubloon is not a US coin — honest abstention, never a fabricated value.
    assert!(out.contains("\"abstained\":true"), "doubloon abstains: {out}");
}
