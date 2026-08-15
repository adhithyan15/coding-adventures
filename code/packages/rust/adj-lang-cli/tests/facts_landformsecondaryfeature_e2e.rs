//! End-to-end test for the geography FACTS library
//! (`adj-facts-stdlib/geography/landform-secondary-feature.adj`) driven
//! through the built CLI: a native `table` naming the second (and, for
//! canyon, third) structural feature the SAME USGS Feature Type Thesaurus
//! spans already state for four landforms -- a sibling to the
//! already-shipped `landforms.adj` (which only carries each landform's
//! single descriptor), decoding the structural-clause half of spans already
//! sitting unused inside that table's own header. Resolves binding-query
//! recall (both directions), a multi-answer forward recall on canyon (whose
//! cited span states two distinct structural clauses), with the source's
//! citation, and abstains on a landform (mountain) the cited spans give no
//! secondary feature for -- 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_landformsecondaryfeature_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("geography/landform-secondary-feature.adj");
    std::fs::copy(&src, dir.join("landform-secondary-feature.adj"))
        .expect("copy shipped landform-secondary-feature.adj");
}

#[test]
fn landform_secondary_feature_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"landform-secondary-feature.adj\"\n\
         ? landform_secondary_feature(valley, $Feature)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"landform_secondary_feature(valley, contains_stream_with_outlet)\""),
        "valley contains a stream with an outlet: {out}"
    );
    assert!(
        out.contains("apps.usgs.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS citation: {out}"
    );
}

#[test]
fn landform_secondary_feature_recalls_backward_from_a_bound_feature() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"landform-secondary-feature.adj\"\n\
         ? landform_secondary_feature($Landform, continuous_slope_at_bottom)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"landform_secondary_feature(canyon, continuous_slope_at_bottom)\""),
        "continuous_slope_at_bottom names canyon: {out}"
    );
}

#[test]
fn landform_secondary_feature_abstains_honestly_on_mountain() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"landform-secondary-feature.adj\"\n\
         ? landform_secondary_feature(mountain, $Feature)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "mountain has no secondary feature in the cited spans -- honest abstention: {out}"
    );
}

#[test]
fn landform_secondary_feature_recalls_both_canyon_features() {
    let dir = scratch("canyon");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"landform-secondary-feature.adj\"\n\
         ? landform_secondary_feature(canyon, $CanyonFeature)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"landform_secondary_feature(canyon, continuous_slope_at_bottom)\""),
        "canyon's cited span states a continuous slope at the bottom: {out}"
    );
    assert!(
        out.contains("\"term\":\"landform_secondary_feature(canyon, steep_sides)\""),
        "canyon's cited span also states steep sides -- multi-answer recall: {out}"
    );
}

#[test]
fn landform_secondary_feature_recalls_plain_uniform_slope_with_citation() {
    let dir = scratch("plain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"landform-secondary-feature.adj\"\n\
         ? landform_secondary_feature(plain, $Feature)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"landform_secondary_feature(plain, uniform_slope)\""),
        "plain's cited span states a general uniform slope: {out}"
    );
    assert!(
        out.contains("apps.usgs.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the USGS citation: {out}"
    );
}
