//! End-to-end test for the meteorology FACTS library
//! (`adj-facts-stdlib/meteorology/precipitation-minimum-diameter.adj`)
//! driven through the built CLI: a native `table` naming the MINIMUM
//! particle diameter (in mm) the NWS Glossary states for a precipitation
//! type, where the source states one -- a sibling to the already-shipped
//! `precipitation-types.adj` (which only carries ONE defining physical form
//! per type), decoding spans already sitting unused inside that table's own
//! provenance block. Resolves binding-query recall (both directions) with
//! the source's citation, and abstains on a type (snow) the cited spans
//! give no diameter figure for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_precipitationmindiameter_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("meteorology/precipitation-minimum-diameter.adj");
    std::fs::copy(&src, dir.join("precipitation-minimum-diameter.adj"))
        .expect("copy shipped precipitation-minimum-diameter.adj");
}

#[test]
fn precipitation_min_diameter_recalls_both_terms_with_citation() {
    let dir = scratch("both");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-minimum-diameter.adj\"\n\
         ? precipitation_min_diameter(rain, $MinDiameterMm)\n\
         ? precipitation_min_diameter(hail, $MinDiameterMm)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"precipitation_min_diameter(rain, 0.5)\""),
        "rain's minimum diameter is 0.5mm: {out}"
    );
    assert!(
        out.contains("\"term\":\"precipitation_min_diameter(hail, 5)\""),
        "hail's minimum diameter is 5mm: {out}"
    );
    assert!(
        out.contains("forecast.weather.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the NWS citation: {out}"
    );
}

#[test]
fn precipitation_min_diameter_recalls_backward_from_a_bound_diameter() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-minimum-diameter.adj\"\n\
         ? precipitation_min_diameter($Precip, 0.5)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"precipitation_min_diameter(rain, 0.5)\""),
        "0.5mm names rain: {out}"
    );
}

#[test]
fn precipitation_min_diameter_abstains_honestly_on_snow() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"precipitation-minimum-diameter.adj\"\n\
         ? precipitation_min_diameter(snow, $MinDiameterMm)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "snow's cited span gives no diameter figure -- honest abstention: {out}"
    );
}
