//! End-to-end test for the money FACTS library
//! (`adj-facts-stdlib/money/coin-penny-discontinued.adj`) driven through
//! the built CLI: a native `table` naming the discontinuation year and
//! production span the SAME U.S. Mint span already states for the penny
//! -- a sibling to the already-shipped `us-coins.adj` (which only carries
//! each coin's cent value), decoding the status/year/duration facts
//! already sitting unused inside that table's own header quote. Resolves
//! binding-query recall with the source's citation, and abstains on every
//! OTHER coin -- the table carries no row for them at all, since none of
//! their own cited spans states any circulation-status change -- 0 model
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
    let dir = std::env::temp_dir().join(format!("adjcli_coinpennydiscontinued_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("money/coin-penny-discontinued.adj");
    std::fs::copy(&src, dir.join("coin-penny-discontinued.adj"))
        .expect("copy shipped coin-penny-discontinued.adj");
}

#[test]
fn coin_penny_discontinued_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"coin-penny-discontinued.adj\"\n\
         ? coin_status(penny, $Status, $Year, $ProductionYears)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"coin_status(penny, discontinued, 2025, 232)\""),
        "the penny was discontinued in 2025 after 232 years: {out}"
    );
    assert!(
        out.contains("usmint.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the U.S. Mint citation: {out}"
    );
}

#[test]
fn coin_penny_discontinued_recalls_year_alone() {
    let dir = scratch("year");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"coin-penny-discontinued.adj\"\n\
         ? coin_status(penny, discontinued, $Year, $ProductionYears)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"coin_status(penny, discontinued, 2025, 232)\""),
        "binding on status still recalls year 2025 and 232 years of production: {out}"
    );
}

#[test]
fn coin_penny_discontinued_abstains_honestly_on_other_coins() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"coin-penny-discontinued.adj\"\n\
         ? coin_status(nickel, $Status, $Year, $ProductionYears)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "nickel's own cited span states no circulation-status change -- honest abstention: {out}"
    );
}
