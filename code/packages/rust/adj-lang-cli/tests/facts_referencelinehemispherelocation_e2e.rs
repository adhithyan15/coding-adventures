//! End-to-end test for the geography FACTS library
//! (`adj-facts-stdlib/geography/reference-line-hemisphere-location.adj`)
//! driven through the built CLI: a native `table` naming the single
//! hemisphere the SAME already-quoted NOAA NESDIS sentence states each
//! tropic sits within -- a sibling to the already-shipped
//! `reference-line-degree.adj`, decoding a clause already sitting unused in
//! that table's own header. Distinct in purpose from the already-shipped
//! `reference_line_hemisphere_split` table (which covers a DIFFERENT
//! question -- which pair of hemispheres a line DIVIDES -- for a disjoint
//! set of lines, equator/prime_meridian). Resolves binding-query recall
//! (both directions) with the source's citation, and abstains on equator
//! (whose own cited span states no single containing hemisphere) -- 0
//! model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_referencelinehemispherelocation_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geography/reference-line-hemisphere-location.adj");
    std::fs::copy(&src, dir.join("reference-line-hemisphere-location.adj"))
        .expect("copy shipped reference-line-hemisphere-location.adj");
}

#[test]
fn reference_line_hemisphere_location_recalls_tropic_of_cancer_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-line-hemisphere-location.adj\"\n\
         ? reference_line_hemisphere_location(tropic_of_cancer, $Hemisphere)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"reference_line_hemisphere_location(tropic_of_cancer, northern)\""),
        "the Tropic of Cancer is in the Northern Hemisphere: {out}"
    );
    assert!(
        out.contains("nesdis.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA NESDIS citation: {out}"
    );
}

#[test]
fn reference_line_hemisphere_location_recalls_backward_to_capricorn() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-line-hemisphere-location.adj\"\n\
         ? reference_line_hemisphere_location($Line, southern)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"reference_line_hemisphere_location(tropic_of_capricorn, southern)\""),
        "southern names the Tropic of Capricorn: {out}"
    );
}

#[test]
fn reference_line_hemisphere_location_abstains_honestly_on_equator() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-line-hemisphere-location.adj\"\n\
         ? reference_line_hemisphere_location(equator, $Hemisphere)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "equator's own cited span states no single containing hemisphere (it divides hemispheres instead) -- honest abstention: {out}"
    );
}
