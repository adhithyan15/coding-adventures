//! End-to-end test for the geography reference-lines FACTS library
//! (`adj-facts-stdlib/geography/reference-lines.adj`): a native `table` of
//! reference line → what it marks resolves forward AND reverse binding queries
//! with the NOAA citation, and abstains on a line the table does not ground —
//! 0 answer-time model calls.

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
fn geography_reference_lines_recall_binds_marks_with_citation_and_abstains() {
    let dir = scratch("referencelines");
    // Copy the shipped geography table beside the entry program and import it.
    let src = facts_stdlib().join("geography/reference-lines.adj");
    std::fs::copy(&src, dir.join("reference-lines.adj"))
        .expect("copy shipped reference-lines.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-lines.adj\"\n\
         ? reference_line(equator, $Marks)\n\
         ? reference_line($Line, zero_degrees_longitude)\n\
         ? reference_line(international_date_line, $Marks)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // (a) Forward: the equator marks 0 degrees latitude — the source's atom.
    assert!(
        out.contains("\"Marks\":\"zero_degrees_latitude\""),
        "equator → zero_degrees_latitude: {out}"
    );
    // Reverse: the line at 0 degrees longitude is the prime meridian.
    assert!(
        out.contains("\"Line\":\"prime_meridian\""),
        "0 deg longitude → prime_meridian: {out}"
    );
    // (a cont.) The answer carries the NOAA citation (locator + trust) as its proof.
    assert!(
        out.contains("oceanservice.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA citation: {out}"
    );
    // (b) The international date line is a real line but is NOT grounded in a row
    // here — honest abstention, never a fabricated `marks` atom.
    assert!(
        out.contains("\"abstained\":true"),
        "international_date_line abstains: {out}"
    );
}
