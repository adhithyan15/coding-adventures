//! End-to-end test for the chemistry FACTS library
//! (`adj-facts-stdlib/chemistry/measuring-tools.adj`) driven through the
//! built CLI: a native `table` naming which quantity each of four common
//! lab tools measures, quoted verbatim from a Chemistry LibreTexts intro
//! lab manual -- a genuinely new "observation and measurement" axis (ADJ-
//! STDLIB-COVERAGE.md 5.1's named Major Gap for K-8 science), distinct from
//! the sibling `lab-equipment.adj`'s tool->purpose-verb table. Deliberately
//! NOT a 5th ordinal-bridge instance -- the science lane's four prior slices
//! (season/planet/moon-phase/mitosis) already saturate that pattern. 0
//! answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_measuringtools_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("chemistry/measuring-tools.adj");
    std::fs::copy(&src, dir.join("measuring-tools.adj"))
        .expect("copy shipped measuring-tools.adj");
}

#[test]
fn measuring_tool_recall_binds_the_quantity_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"measuring-tools.adj\"\n\
         ? measuring_tool(thermometer, $Q)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Q\":\"temperature\""),
        "a thermometer measures temperature: {out}"
    );
    assert!(
        out.contains("chem.libretexts.org") && out.contains("\"trust\":\"consensus\""),
        "carries the LibreTexts citation: {out}"
    );
}

#[test]
fn measuring_tool_reverse_binds_the_tool_for_that_quantity() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"measuring-tools.adj\"\n\
         ? measuring_tool($T, volume)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"graduated_cylinder\""),
        "a graduated cylinder measures volume: {out}"
    );
}

#[test]
fn measuring_tool_abstains_honestly_on_an_unshipped_tool() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"measuring-tools.adj\"\n\
         ? measuring_tool(microscope, $Q)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "a microscope has no shipped row -- honest abstention, never invented: {out}"
    );
}
