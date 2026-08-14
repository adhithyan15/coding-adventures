//! End-to-end test for the money FACTS library
//! (`adj-facts-stdlib/money/bill-back-vignette.adj`) driven through the
//! built CLI: a native `table` naming the back-of-note vignette the SAME
//! USCEP feature-sheet sentence already states for six of the seven US
//! paper bills -- a sibling to the already-shipped `us-bills.adj` (which
//! only carries each note's front portrait), decoding the vignette half of
//! spans already sitting unused inside that table's own header quotes.
//! Resolves binding-query recall (both directions) with the source's
//! citation, and abstains on the $5 note, whose own cited span names no
//! back vignette -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_billbackvignette_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("money/bill-back-vignette.adj");
    std::fs::copy(&src, dir.join("bill-back-vignette.adj"))
        .expect("copy shipped bill-back-vignette.adj");
}

#[test]
fn bill_back_vignette_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"bill-back-vignette.adj\"\n\
         ? bill_back_vignette(20, $Vignette)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"bill_back_vignette(20, white_house)\""),
        "the $20 note's back vignette is the White House: {out}"
    );
    assert!(
        out.contains("uscurrency.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USCEP citation: {out}"
    );
}

#[test]
fn bill_back_vignette_recalls_backward_from_a_bound_vignette() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"bill-back-vignette.adj\"\n\
         ? bill_back_vignette($Dollars, us_capitol)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"bill_back_vignette(50, us_capitol)\""),
        "the US Capitol names the $50 note: {out}"
    );
}

#[test]
fn bill_back_vignette_abstains_honestly_on_the_five() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"bill-back-vignette.adj\"\n\
         ? bill_back_vignette(5, $Vignette)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the $5 note's own cited span names no back vignette -- honest abstention: {out}"
    );
}
