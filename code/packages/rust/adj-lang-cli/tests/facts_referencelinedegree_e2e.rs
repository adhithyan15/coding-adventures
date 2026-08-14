//! End-to-end test for the geography FACTS library
//! (`adj-facts-stdlib/geography/reference-line-degree.adj`) driven through
//! the built CLI: a native `table` naming the exact signed numeric latitude
//! of the two tropics, decoded from a clause already sitting unused inside
//! `reference-lines.adj`'s own already-quoted NOAA NESDIS source sentence
//! -- a sibling to that table. Resolves binding-query recall (both
//! directions, including a negative-number bind) with the source's
//! citation, and abstains on a real, already-tabled reference line
//! (equator) whose own 0-degree fact is already fully captured by its own
//! `marks` atom, not decoded here -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_referencelinedegree_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geography/reference-line-degree.adj");
    std::fs::copy(&src, dir.join("reference-line-degree.adj"))
        .expect("copy shipped reference-line-degree.adj");
}

#[test]
fn reference_line_degree_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-line-degree.adj\"\n\
         ? reference_line_degree(tropic_of_cancer, $Degrees)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"reference_line_degree(tropic_of_cancer, 23.5)\""),
        "the Tropic of Cancer sits at +23.5 degrees: {out}"
    );
    assert!(
        out.contains("nesdis.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA NESDIS citation: {out}"
    );
}

#[test]
fn reference_line_degree_recalls_backward_to_capricorn() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-line-degree.adj\"\n\
         ? reference_line_degree($Line, -23.5)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"reference_line_degree(tropic_of_capricorn, -23.5)\""),
        "-23.5 degrees recalls the Tropic of Capricorn: {out}"
    );
}

#[test]
fn reference_line_degree_abstains_honestly_on_equator() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-line-degree.adj\"\n\
         ? reference_line_degree(equator, $Degrees)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "the equator is a real, already-tabled reference line but its 0-degree fact is already fully captured by its own marks atom, not decoded in this table -- honest abstention: {out}"
    );
}
