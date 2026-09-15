//! End-to-end test for the geography FACTS library
//! (`adj-facts-stdlib/geography/landform-secondary-feature.adj`) driven
//! through the built CLI: a native `table` naming the second (and, for
//! canyon, third) structural feature the USGS Feature Type Thesaurus
//! definitions state for four landforms. Resolves binding-query recall (both
//! directions), a multi-answer forward recall on canyon, with the source's
//! citation, and abstains on a landform (mountain) the definitions give no
//! secondary feature for -- 0 model calls.
//!
//! Each row CITES its own landform's definition. `cites`, not `source`: on the
//! page the landform is a separate term link and the definition a span beneath
//! it that never names the landform (#13934's held question). The envelope used
//! to be valley's definition, with the other three attached as table-level
//! `cites`, so every answer carried all four; it is now the thesaurus's own
//! description, which names no landform.

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

const LOCATOR: &str = "https://apps.usgs.gov/thesaurus/thesaurus-full.php?thcode=3";
const ENVELOPE: &str = "Types of named geographic features.";
const OLD_ENVELOPE: &str = "Low-lying land bordered by higher ground; especially elongate, relatively large gently sloping depressions of the Earth's surface, commonly situated between two mountains or between ranges of hills or mountains, and often containing a stream with an outlet.";

/// Each landform's own definition, generated from the converter's dump.
const DEFINITIONS: [(&str, &str); 4] = [
    ("valley", "Low-lying land bordered by higher ground; especially elongate, relatively large gently sloping depressions of the Earth's surface, commonly situated between two mountains or between ranges of hills or mountains, and often containing a stream with an outlet."),
    ("plateau", "Comparatively flat areas of great extent and elevation; specif. extensive land regions considerably above the adjacent country or above sea level; commonly limited on at least one side by an abrupt descent, have flat or nearly smooth surfaces but are often dissected by deep valleys and surmounted by high hills or mountains, and have a large part of their total surface at or near the summit level."),
    ("canyon", "Relatively narrow, deep depressions with steep sides, the bottom of which generally has a continuous slope"),
    ("plain", "Regions of general uniform slope, comparatively level and of considerable extent."),
];

/// (landform, feature, the definition that row cites).
const ROWS: [(&str, &str, &str); 5] = [
    ("valley", "contains_stream_with_outlet", "Low-lying land bordered by higher ground; especially elongate, relatively large gently sloping depressions of the Earth's surface, commonly situated between two mountains or between ranges of hills or mountains, and often containing a stream with an outlet."),
    ("plateau", "bounded_by_abrupt_descent", "Comparatively flat areas of great extent and elevation; specif. extensive land regions considerably above the adjacent country or above sea level; commonly limited on at least one side by an abrupt descent, have flat or nearly smooth surfaces but are often dissected by deep valleys and surmounted by high hills or mountains, and have a large part of their total surface at or near the summit level."),
    ("canyon", "continuous_slope_at_bottom", "Relatively narrow, deep depressions with steep sides, the bottom of which generally has a continuous slope"),
    ("canyon", "steep_sides", "Relatively narrow, deep depressions with steep sides, the bottom of which generally has a continuous slope"),
    ("plain", "uniform_slope", "Regions of general uniform slope, comparatively level and of considerable extent."),
];

fn definition(landform: &str) -> &'static str {
    DEFINITIONS
        .iter()
        .find(|(l, _)| *l == landform)
        .map(|(_, d)| *d)
        .expect("known landform")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("geography/landform-secondary-feature.adj"))
        .expect("read shipped landform-secondary-feature.adj");
    adj[adj.find("table landform_secondary_feature").expect("table")..].to_string()
}

/// The whole primary citation, as the serializer emits it.
fn primary() -> String {
    format!("\"source\":\"{ENVELOPE}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\"")
}

/// The whole citation for an answer whose row cites `landform`'s definition.
fn citation_for(landform: &str) -> String {
    format!(
        "{},\"corroborations\":[{{\"source\":\"{}\",\"locator\":\"{LOCATOR}\"}}]",
        primary(),
        definition(landform)
    )
}

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"landform-secondary-feature.adj\"\n? {query}\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn landform_secondary_feature_recalls_with_citation() {
    let out = ask("forward", "landform_secondary_feature(valley, $Feature)");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"landform_secondary_feature(valley, contains_stream_with_outlet)\""),
        "valley contains a stream with an outlet: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("apps.usgs.gov") && contains(trust)`
    // (#15209's two-loose-needles shape).
    assert!(out.contains(&citation_for("valley")), "carries the USGS citation, whole: {out}");
}

#[test]
fn landform_secondary_feature_recalls_backward_from_a_bound_feature() {
    let out = ask("backward", "landform_secondary_feature($Landform, continuous_slope_at_bottom)");
    assert!(
        out.contains("\"term\":\"landform_secondary_feature(canyon, continuous_slope_at_bottom)\""),
        "continuous_slope_at_bottom names canyon: {out}"
    );
}

#[test]
fn landform_secondary_feature_abstains_honestly_on_mountain() {
    let out = ask("abstain", "landform_secondary_feature(mountain, $Feature)");
    assert!(
        out.contains("\"abstained\":true"),
        "mountain has no secondary feature in the cited definitions -- honest abstention: {out}"
    );
}

#[test]
fn landform_secondary_feature_recalls_both_canyon_features() {
    let out = ask("canyon", "landform_secondary_feature(canyon, $CanyonFeature)");
    assert!(
        out.contains("\"term\":\"landform_secondary_feature(canyon, continuous_slope_at_bottom)\""),
        "canyon's definition states a continuous slope at the bottom: {out}"
    );
    assert!(
        out.contains("\"term\":\"landform_secondary_feature(canyon, steep_sides)\""),
        "canyon's definition also states steep sides -- multi-answer recall: {out}"
    );
    // Both answers carry the one canyon definition.
    assert_eq!(
        out.matches(&citation_for("canyon")).count(),
        4,
        "two canyon answers, each citation emitted twice (citations and steps): {out}"
    );
}

#[test]
fn landform_secondary_feature_recalls_plain_uniform_slope_with_citation() {
    let out = ask("plain", "landform_secondary_feature(plain, $Feature)");
    assert!(
        out.contains("\"term\":\"landform_secondary_feature(plain, uniform_slope)\""),
        "plain's definition states a general uniform slope: {out}"
    );
    assert!(out.contains(&citation_for("plain")), "carries the USGS citation, whole: {out}");
}

#[test]
fn landform_secondary_feature_plateau_citation_is_the_pages_whole_sentence() {
    // The plateau definition once carried a marked trailing ellipsis, then an
    // undeclared mid-sentence truncation; it is the page's complete sentence.
    // It used to be pinned inside a table-level corroboration list that EVERY
    // answer carried; it is now pinned on the plateau answer, as that row's
    // only corroboration.
    let out = ask("reground", "landform_secondary_feature(plateau, $Feature)");
    assert!(out.contains("\"Feature\":\"bounded_by_abrupt_descent\""), "{out}");
    assert!(out.contains(&citation_for("plateau")), "the plateau corroboration is whole: {out}");
    assert_eq!(definition("plateau").chars().count(), 399, "the page's whole 399-character sentence");
    assert!(definition("plateau").ends_with("summit level."), "ends where the page's sentence ends");
}

#[test]
fn landform_canyon_citation_carries_no_full_stop_the_page_lacks() {
    // Tag kept distinct from "reground": scratch() keys on tag + pid, and
    // cargo runs a binary's tests as threads of one process.
    let out = ask("canyon_period_4d", "landform_secondary_feature(canyon, $F)");
    // The USGS thesaurus ends this definition WITHOUT a period -- a bracketed
    // source reference follows it on the page. The pin reaches the canyon
    // answer's own corroboration, so a restored period fails it.
    assert!(out.contains(&citation_for("canyon")), "the canyon corroboration matches its page: {out}");
    assert!(
        definition("canyon").ends_with("a continuous slope"),
        "no full stop the page lacks: {:?}",
        definition("canyon")
    );
}

#[test]
fn every_row_is_corroborated_by_its_own_landforms_definition_only() {
    for (landform, feature, span) in ROWS {
        // BIND THE LANDFORM, DO NOT GROUND BOTH. A fully ground query is ranked
        // as a HYPOTHESIS and carries no citations at all -- so the negative
        // arm below would pass over an empty answer. The one-answer count
        // caught exactly that. Every feature atom is distinct, so binding the
        // landform from the feature yields one answer even for canyon's pair.
        let out = ask(
            &format!("corr_{landform}_{feature}"),
            &format!("landform_secondary_feature($L, {feature})"),
        );
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {landform}/{feature}: {out}");
        assert!(out.contains(&format!("\"L\":\"{landform}\"")), "{feature} binds {landform}: {out}");
        assert!(out.contains(&citation_for(landform)), "{landform}/{feature}: envelope primary, own definition: {out}");
        assert_eq!(span, definition(landform), "the constant row span is its landform's definition");
        // NEGATIVE ARM: before this change every answer carried all four.
        for (other, other_def) in DEFINITIONS {
            if other != landform {
                assert!(!out.contains(other_def), "the {other} definition must not reach {landform}/{feature}: {out}");
            }
        }
    }
}

#[test]
fn no_row_carries_a_source_and_no_cites_is_table_level() {
    let body = shipped_table();
    assert_eq!(
        body.lines()
            .filter(|l| l.trim_start().starts_with("row (") && l.trim_end().ends_with('{'))
            .count(),
        5,
        "every row opens its own block"
    );
    let cites: Vec<&str> = body.lines().filter(|l| l.trim_start().starts_with("cites \"")).collect();
    assert_eq!(cites.len(), 5, "one corroboration per row: {cites:?}");
    assert!(
        cites.iter().all(|l| l.starts_with("        cites \"")),
        "no `cites` at table level, where it would reach every answer: {cites:?}"
    );
    let source_lines: Vec<&str> = body.lines().filter(|l| l.trim_start().starts_with("source \"")).collect();
    assert_eq!(
        source_lines,
        vec![format!("    source \"{ENVELOPE}\"").as_str()],
        "exactly one `source` line in the table -- the envelope's"
    );
    let out = ask("nosource", "landform_secondary_feature(plain, $F)");
    assert!(out.contains(&primary()), "the envelope is the PRIMARY source: {out}");
}

#[test]
fn the_envelope_describes_the_thesaurus_and_names_no_landform() {
    let body = shipped_table();
    assert!(
        !body.contains(&format!("source \"{OLD_ENVELOPE}\"")),
        "the envelope must not be valley's definition again"
    );
    assert!(body.contains(&format!("    source \"{ENVELOPE}\"")), "the envelope is shipped");
    let folded = ENVELOPE.to_lowercase();
    let mut keys = 0;
    for line in body.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("row (") {
            let key = rest.split(',').next().expect("row key").trim();
            keys += 1;
            assert!(!folded.contains(key), "the envelope must name no landform, but names {key:?}");
        }
    }
    assert_eq!(keys, 5, "all five row keys were actually checked");
    for (landform, feature, span) in ROWS {
        assert!(
            body.contains(&format!("    row ({landform}, {feature}) {{\n        cites \"{span}\" locator \"{LOCATOR}\"\n    }}")),
            "row ({landform}, {feature}) is shipped with its own definition"
        );
    }
}
