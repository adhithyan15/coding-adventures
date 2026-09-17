//! End-to-end test for the environment FACTS library
//! (`adj-facts-stdlib/environment/aqi-category-color.adj`) driven through
//! the built CLI: a native `table` naming the color the SAME AirNow source
//! spans already state for each AQI category -- a sibling to the
//! already-shipped `air-quality-index.adj` (which only carries each
//! band's numeric breakpoint and category), decoding the color half of
//! spans already sitting unused inside that table's own per-row
//! provenance block. Resolves binding-query recall (both directions) with
//! the source's citation, and covers the full category domain with no
//! abstention -- 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SPAN (RS-5e, #14986). The six AirNow spans used to
//! be table-level `cites`, so every answer carried all six and a `hazardous`
//! answer's primary source was the envelope rather than the Maroon row. This
//! conversion only RELOCATED them: all six were already on the envelope's own
//! locator, so no row restates a locator or a trust tier.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_aqicategorycolor_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("environment/aqi-category-color.adj");
    std::fs::copy(&src, dir.join("aqi-category-color.adj"))
        .expect("copy shipped aqi-category-color.adj");
}

#[test]
fn aqi_category_color_recalls_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"aqi-category-color.adj\"\n\
         ? aqi_category_color(hazardous, $Color)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"aqi_category_color(hazardous, maroon)\""),
        "hazardous is maroon: {out}"
    );
    // This was `contains("airnow.gov") && contains("\"trust\":\"authoritative\"")`,
    // which any AirNow citation satisfies and which constrains no span text at
    // all. Before the RS-5e conversion it was satisfied by ONE citation object
    // carrying six corroborations, so a hazardous answer dragged the Green,
    // Yellow, Orange, Red and Purple spans along with it. The needle is now the
    // whole citations array, closing on both the corroborations `]` and the
    // citations `]`.
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(
        out.contains(&only_citation(HAZARDOUS)),
        "the hazardous answer carries the Maroon span, whole, and nothing else: {out}"
    );
    for other in [GOOD, MODERATE, USG, UNHEALTHY, VERY_UNHEALTHY] {
        assert!(!other.is_empty() && !out.contains(other), "another category's span must not ride along: {out}");
    }
    assert!(!out.contains(ENVELOPE), "the envelope is primary for no answer: {out}");
}

const LOCATOR: &str = "https://www.airnow.gov/aqi/aqi-basics/";
const ENVELOPE: &str = "The AQI includes six color-coded categories, each corresponding to a range of index values.";
const GOOD: &str = "Green Good 0 to 50 Air quality is satisfactory, and air pollution poses little or no risk.";
const MODERATE: &str = "Yellow Moderate 51 to 100 Air quality is acceptable. However, there may be a risk for some people, particularly those who are unusually sensitive to air pollution.";
const USG: &str = "Orange Unhealthy for Sensitive Groups 101 to 150 Members of sensitive groups may experience health effects. The general public is less likely to be affected.";
const UNHEALTHY: &str = "Red Unhealthy 151 to 200 Some members of the general public may experience health effects; members of sensitive groups may experience more serious health effects.";
const VERY_UNHEALTHY: &str = "Purple Very Unhealthy 201 to 300 Health alert: The risk of health effects is increased for everyone.";
const HAZARDOUS: &str = "Maroon Hazardous 301 and higher Health warning of emergency conditions: everyone is more likely to be affected.";

/// (category, its colour, the AirNow span naming that colour)
fn categories() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("good", "green", GOOD),
        ("moderate", "yellow", MODERATE),
        ("unhealthy_for_sensitive_groups", "orange", USG),
        ("unhealthy", "red", UNHEALTHY),
        ("very_unhealthy", "purple", VERY_UNHEALTHY),
        ("hazardous", "maroon", HAZARDOUS),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run,
/// closing on both the corroborations `]` and the citations `]` (#14735).
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("environment/aqi-category-color.adj"))
        .expect("read shipped aqi-category-color.adj");
    adj[adj.find("table aqi_category_color").expect("table")..].to_string()
}

#[test]
fn every_category_answer_carries_only_the_span_that_names_its_colour() {
    // The six spans used to be table-level `cites`, so every answer carried all
    // six and a hazardous answer's primary source was the envelope.
    for (category, colour, span) in categories() {
        let dir = scratch(&format!("cat_{category}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"aqi-category-color.adj\"\n? aqi_category_color({category}, $Color)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {category}: {out}");
        assert!(
            out.contains(&format!("\"term\":\"aqi_category_color({category}, {colour})\"")),
            "{category} is {colour}: {out}"
        );
        assert!(
            out.contains(&only_citation(span)),
            "{category}: its own AirNow span, whole, and the only citation: {out}"
        );
        assert!(
            span.to_lowercase().contains(colour),
            "the span carried by {category} names {colour} -- the defect was that every answer carried all six"
        );
        for other in [GOOD, MODERATE, USG, UNHEALTHY, VERY_UNHEALTHY, HAZARDOUS] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another category's span must not reach {category}: {out}"
                );
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {category}: {out}");
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it, including the tier.
    let body = shipped_table();
    assert_eq!(body.matches("\n        source \"").count(), 6, "six row sources");
    // Keyword-anchored rather than indent-scoped: a row-level `cites` at eight
    // spaces must fail this too, but a `% ... cites ...` comment inside the
    // table body must not, and `cites"..."` with no space must not slip past.
    assert!(
        !body.lines().any(|l| l.trim_start().starts_with("cites")),
        "no corroboration at any indent -- the six spans now live in their rows"
    );
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust: every row's page is the envelope's"
    );
    assert!(
        body.contains(&format!(
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n"
        )),
        "the envelope is unchanged, at the authoritative tier"
    );
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    for word in [
        "good", "moderate", "unhealthy", "hazardous", "green", "yellow", "orange", "red",
        "purple", "maroon",
    ] {
        assert!(
            !shipped_envelope.contains(word),
            "the shipped envelope must name no category and no colour, but contains {word:?}"
        );
    }
}

#[test]
fn aqi_category_color_recalls_backward_from_a_bound_color() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"aqi-category-color.adj\"\n\
         ? aqi_category_color($Category, orange)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"aqi_category_color(unhealthy_for_sensitive_groups, orange)\""),
        "orange names the sensitive-groups category: {out}"
    );
}

#[test]
fn aqi_category_color_covers_the_full_domain_without_abstention() {
    let dir = scratch("full");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"aqi-category-color.adj\"\n\
         ? aqi_category_color(good, $Color)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"aqi_category_color(good, green)\""),
        "good is green: {out}"
    );
    assert!(
        !out.contains("\"abstained\":true"),
        "every category has a color -- no abstention expected: {out}"
    );
}
