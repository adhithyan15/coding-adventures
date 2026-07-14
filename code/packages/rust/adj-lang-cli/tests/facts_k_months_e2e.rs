//! End-to-end test for the kindergarten months FACTS library
//! (`adj-facts-stdlib/kindergarten/months.adj`) driven through the built CLI:
//! a native `table` of month → month-number resolves a binding-query recall with
//! the source's citation, binds both directions, and abstains on a non-month —
//! 0 model calls.

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
fn kindergarten_months_recall_binds_month_number_with_citation() {
    let dir = scratch("months");
    // Copy the shipped kindergarten table beside the entry program and import it.
    let src = facts_stdlib().join("kindergarten/months.adj");
    std::fs::copy(&src, dir.join("months.adj")).expect("copy shipped months.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"months.adj\"\n\
         ? month_number(january, $Number)\n\
         ? month_number(december, $Number)\n\
         ? month_number($Month, 1)\n\
         ? month_number(smarch, $Number)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // January is the first month; December is the twelfth — the recalled numbers.
    assert!(out.contains("\"Number\":\"1\""), "january → 1: {out}");
    assert!(out.contains("\"Number\":\"12\""), "december → 12: {out}");
    // The relation binds in reverse too: number 1 recovers the month january.
    assert!(out.contains("\"Month\":\"january\""), "1 → january: {out}");
    // The answer carries the ISO 8601 citation as its proof.
    assert!(
        out.contains("cl.cam.ac.uk") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // `smarch` is not a month — honest abstention, never a fabricated index.
    assert!(out.contains("\"abstained\":true"), "smarch abstains: {out}");
}
