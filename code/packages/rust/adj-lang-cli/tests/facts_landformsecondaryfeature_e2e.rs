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

const LANDFORM_SECONDARY_FEATURE_PIN: &str = r#""bindings":{"Feature":"bounded_by_abrupt_descent"},"citations":[{"source":"Low-lying land bordered by higher ground; especially elongate, relatively large gently sloping depressions of the Earth's surface, commonly situated between two mountains or between ranges of hills or mountains, and often containing a stream with an outlet.","locator":"https://apps.usgs.gov/thesaurus/thesaurus-full.php?thcode=3","trust":"authoritative","corroborations":[{"source":"Comparatively flat areas of great extent and elevation; specif. extensive land regions considerably above the adjacent country or above sea level; commonly limited on at least one side by an abrupt descent, have flat or nearly smooth surfaces but are often dissected by deep valleys and surmounted by high hills or mountains, and have a large part of their total surface at or near the summit level.","locator":"https://apps.usgs.gov/thesaurus/thesaurus-full.php?thcode=3"}"#;

#[test]
fn landform_secondary_feature_plateau_citation_is_the_pages_whole_sentence() {
    let dir = scratch("reground");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"landform-secondary-feature.adj\"\n? landform_secondary_feature(plateau, $Feature)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The value carried a MARKED trailing ellipsis ("...an abrupt descent ...").
    //
    // DROPPING IT ALONE WAS THE FIRST REPAIR AND IT WAS WORSE: it replaced a
    // declared elision with an undeclared mid-sentence truncation. The value is
    // now the page's complete 399-character sentence.
    //
    // THE CITATION DOES NOT NAME ITS SUBJECT, and cannot without crossing an
    // element boundary: the page renders the term and its definition as
    // siblings -- "plateaus" then "Comparatively flat areas..." -- so a span
    // naming it would be verbatim against the EXTRACTOR'S CONCATENATION rather
    // than the page. Stated rather than papered over.
    //
    // The pin spans the bindings THROUGH the corroboration, because a `cites`
    // repair lands past the envelope's trust field. The query asks `plateau`,
    // the row this corroboration grounds; no authored query does.
    assert!(
        out.contains(LANDFORM_SECONDARY_FEATURE_PIN),
        "the plateau corroboration is a contiguous page span: {out}"
    );
}


const LANDFORM_CANYON_PIN: &str = r#""bindings":{"F":"continuous_slope_at_bottom"},"citations":[{"source":"Low-lying land bordered by higher ground; especially elongate, relatively large gently sloping depressions of the Earth's surface, commonly situated between two mountains or between ranges of hills or mountains, and often containing a stream with an outlet.","locator":"https://apps.usgs.gov/thesaurus/thesaurus-full.php?thcode=3","trust":"authoritative","corroborations":[{"source":"Comparatively flat areas of great extent and elevation; specif. extensive land regions considerably above the adjacent country or above sea level; commonly limited on at least one side by an abrupt descent, have flat or nearly smooth surfaces but are often dissected by deep valleys and surmounted by high hills or mountains, and have a large part of their total surface at or near the summit level.","locator":"https://apps.usgs.gov/thesaurus/thesaurus-full.php?thcode=3"},{"source":"Relatively narrow, deep depressions with steep sides, the bottom of which generally has a continuous slope","locator":"https://apps.usgs.gov/thesaurus/thesaurus-full.php?thcode=3"}"#;

#[test]
fn landform_canyon_citation_carries_no_full_stop_the_page_lacks() {
    // NOT "reground": installment 4b's test in this same file already uses that
    // tag, and scratch() keys on tag + pid while cargo runs a binary's tests as
    // parallel threads of ONE process. The two tests shared a directory and one
    // overwrote the other's case.adj, so this test silently executed 4b's
    // plateau query instead of the canyon one.
    let dir = scratch("canyon_period_4d");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"landform-secondary-feature.adj\"\n? landform_secondary_feature(canyon, $F)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The USGS thesaurus ends this definition WITHOUT a period -- a bracketed
    // source citation follows it on the page:
    //
    //   canyons Relatively narrow, deep depressions with steep sides, the
    //   bottom of which generally has a continuous slope [NIMA GEONet ...]
    //
    // The shipped value had a full stop. One character, existing nowhere on the
    // page, in a field whose whole contract is that its bytes are on that page.
    // It reads like punctuation hygiene, which is exactly why no screen built
    // for quotes, elisions or dashes could see it.
    //
    // THE PIN REACHES THE SECOND CORROBORATION. The repaired value is the canyon
    // `cites`; a pin stopping at the envelope would stay green if the period
    // came back.
    assert!(
        out.contains(LANDFORM_CANYON_PIN),
        "the canyon corroboration matches its page: {out}"
    );
}
