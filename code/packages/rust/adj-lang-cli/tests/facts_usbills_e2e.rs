//! End-to-end test for the US-bills FACTS library
//! (`adj-facts-stdlib/money/us-bills.adj`) driven through the built CLI:
//! a native `table` of bill-denomination -> front-portrait resolves a
//! binding-query recall with the U.S. Currency Education Program's citation,
//! runs the relation backwards (portrait -> denomination), and abstains on an
//! amount that is not a printed bill -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_facts_{tag}_{}", std::process::id()));
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
fn usbills_recall_binds_portrait_with_citation() {
    let dir = scratch("usbills");
    // Copy the shipped money table beside the entry program and import it.
    let src = facts_stdlib().join("money/us-bills.adj");
    std::fs::copy(&src, dir.join("us-bills.adj")).expect("copy shipped us-bills.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"us-bills.adj\"\n\
         ? bill_portrait(1, $Who)\n\
         ? bill_portrait(100, $Who)\n\
         ? bill_portrait($Dollars, lincoln)\n\
         ? bill_portrait(3, $Who)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Washington is on the $1 note; Franklin is on the $100 -- the recalled portraits.
    assert!(out.contains("\"Who\":\"washington\""), "$1 -> washington: {out}");
    assert!(out.contains("\"Who\":\"franklin\""), "$100 -> franklin: {out}");
    // The relation runs backwards: the bill Lincoln is on is the five.
    assert!(out.contains("\"Dollars\":\"5\""), "lincoln -> $5: {out}");
    // The answer carries the U.S. Currency Education Program citation as its proof.
    assert!(
        out.contains("uscurrency.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the uscurrency.gov citation: {out}"
    );
    // There is no $3 bill -- honest abstention, never a fabricated portrait.
    assert!(out.contains("\"abstained\":true"), "$3 abstains: {out}");
}
