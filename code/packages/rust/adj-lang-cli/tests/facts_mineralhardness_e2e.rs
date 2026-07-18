//! End-to-end test for the geology FACTS library
//! (`adj-facts-stdlib/geology/mineral-hardness.adj`) driven through the built
//! CLI: a native `table` of Mohs reference mineral → numeric hardness resolves a
//! binding-query recall with the NPS "Mohs Hardness Scale" citation, runs the
//! relation backward (hardness → mineral), and abstains on a mineral the scale's
//! ten reference minerals do not fix (graphite) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factstemp_{tag}_{}", std::process::id()));
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
fn geology_mineral_hardness_recall_binds_value_with_citation() {
    let dir = scratch("mineralhardness");
    // Copy the shipped geology table beside the entry program and import it.
    let src = facts_stdlib().join("geology/mineral-hardness.adj");
    std::fs::copy(&src, dir.join("mineral-hardness.adj")).expect("copy shipped mineral-hardness.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"mineral-hardness.adj\"\n\
         ? mineral_hardness(talc, $H)\n\
         ? mineral_hardness(quartz, $H)\n\
         ? mineral_hardness(corundum, $H)\n\
         ? mineral_hardness($M, 10)\n\
         ? mineral_hardness(graphite, $H)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Forward lookups bind each reference mineral to the whole number NPS fixes
    // it at, each a plain number on the Mohs 1..10 ladder.
    assert!(out.contains("\"H\":\"1\""), "talc → 1: {out}");
    assert!(out.contains("\"H\":\"7\""), "quartz → 7: {out}");
    assert!(out.contains("\"H\":\"9\""), "corundum → 9: {out}");
    // The relation runs BACKWARD: the hardness 10 recalls diamond.
    assert!(
        out.contains("\"M\":\"diamond\""),
        "10 → diamond (reverse recall): {out}"
    );
    // The answer carries the NPS locator + trust tier as its proof.
    assert!(
        out.contains("nps.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // Graphite is not one of the ten Mohs reference minerals fixed by this
    // table — honest abstention, never a fabricated hardness.
    assert!(
        out.contains("\"abstained\":true"),
        "ungrounded mineral abstains: {out}"
    );
}
