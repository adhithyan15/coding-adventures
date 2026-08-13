//! End-to-end test for the geography FACTS library
//! (`adj-facts-stdlib/geography/reference-line-hemisphere-split.adj`) driven
//! through the built CLI: a native `table` naming the hemisphere pair the
//! SAME NOAA source/cites spans already state for the equator and prime
//! meridian -- a sibling to the already-shipped `reference-lines.adj`
//! (which only carries each line's degree-marking property, not the
//! hemisphere split), decoding the hemisphere-split half of two spans
//! already sitting unused inside that table's own `source`/`cites` fields.
//! Resolves binding-query recall (both directions) with the source's
//! citation, and abstains on a line (tropic_of_cancer) the cited spans give
//! no hemisphere split for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_referencelinehemispheresplit_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geography/reference-line-hemisphere-split.adj");
    std::fs::copy(&src, dir.join("reference-line-hemisphere-split.adj"))
        .expect("copy shipped reference-line-hemisphere-split.adj");
}

#[test]
fn reference_line_hemisphere_split_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-line-hemisphere-split.adj\"\n\
         ? reference_line_hemisphere_split(equator, $Hemispheres)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"reference_line_hemisphere_split(equator, northern_southern)\""),
        "equator splits northern/southern: {out}"
    );
    assert!(
        out.contains("oceanservice.noaa.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NOAA citation: {out}"
    );
}

#[test]
fn reference_line_hemisphere_split_recalls_backward_from_a_bound_hemispheres() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-line-hemisphere-split.adj\"\n\
         ? reference_line_hemisphere_split($Line, eastern_western)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"reference_line_hemisphere_split(prime_meridian, eastern_western)\""),
        "eastern_western is the prime meridian's split: {out}"
    );
}

#[test]
fn reference_line_hemisphere_split_abstains_honestly_on_tropic_of_cancer() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"reference-line-hemisphere-split.adj\"\n\
         ? reference_line_hemisphere_split(tropic_of_cancer, $Hemispheres)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "tropic_of_cancer has no hemisphere split in the cited spans -- honest abstention: {out}"
    );
}
