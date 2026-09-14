//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/rainforest-layer.adj`) driven through the
//! built CLI: a native `table` naming the four rainforest layers and a
//! one-fact description of each, per National Geographic Education's "Rain
//! Forest" entry. 0 answer-time model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_rainforestlayer_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/rainforest-layer.adj");
    std::fs::copy(&src, dir.join("rainforest-layer.adj")).expect("copy shipped rainforest-layer.adj");
}

#[test]
fn rainforest_layer_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rainforest-layer.adj\"\n\
         ? rainforest_layer(emergent, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"tallest_trees_dominate_skyline\""),
        "the emergent layer is the tallest-trees layer: {out}"
    );
    // This asserted `"trust":"consensus"` until #14986. The tier CHANGED, and
    // deliberately: `emergent` is one of the two rows whose atom restates its
    // span rather than quoting it -- "tallest" occurs zero times on the cited
    // page -- so the row now carries `inferred`. The old assertion encoded the
    // uniform table-wide tier that per-row provenance exists to replace, and
    // updating it is the point of the change, not a concession to it.
    assert!(
        out.contains("nationalgeographic.org") && out.contains("\"trust\":\"inferred\""),
        "carries the National Geographic citation, at the reasoned tier: {out}"
    );
}

#[test]
fn rainforest_layer_reverse_binds_the_layer_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rainforest-layer.adj\"\n\
         ? rainforest_layer($L, deep_treetop_vegetation_layer)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"L\":\"canopy\""),
        "the canopy is the deep treetop vegetation layer: {out}"
    );
}

#[test]
fn rainforest_layer_abstains_honestly_on_an_untabled_layer() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rainforest-layer.adj\"\n\
         ? rainforest_layer(soil_layer, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "soil layer is not one of the four named rainforest layers -- honest abstention, never invented: {out}"
    );
}

const LOCATOR: &str = "https://education.nationalgeographic.org/resource/rain-forest/";

/// Assert one layer's row carries its own page sentence, as a whole
/// `source`/`locator`/`trust` object, in a program returning only that row.
fn assert_layer_row(tag: &str, layer: &str, span: &str, tier: &str) -> String {
    let dir = scratch(tag);
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"rainforest-layer.adj\"\n? rainforest_layer({layer}, $D)\n"),
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one answer, so every needle below belongs to {layer}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"{tier}\""
        )),
        "{layer} carries its own sentence at tier {tier}: {out}"
    );
    // NAMED NEGATIVE. The emergent sentence was the old envelope and so every
    // row's warrant -- a forest-floor recall was warranted by a sentence about
    // the treetops.
    if layer != "emergent" {
        assert!(
            !out.contains("trees as tall as 60 meters"),
            "{layer} is not warranted by the emergent sentence: {out}"
        );
    }
    out
}

/// #14986: this table had one envelope -- the EMERGENT row's own sentence --
/// so every layer answered with it.
///
/// `forest_floor` is the interesting one. The bounding-language audit flagged
/// `darkest_layer_...` as a superlative with no bounding language in its
/// provenance. The page states it outright -- "the darkest of all rainforest
/// layers", one occurrence -- so the defect was never the claim, only that the
/// evidence sat outside the machine-readable envelope. That is exactly what
/// RS-5e fixes, and this pins it.
#[test]
fn rainforest_forest_floor_carries_the_superlative_the_page_states() {
    let out = assert_layer_row(
        "rainforestfloor",
        "forest_floor",
        "The forest floor is the darkest of all rainforest layers, making it extremely difficult for plants to grow.",
        "consensus",
    );
    assert!(
        out.contains("darkest of all rainforest layers"),
        "the bound itself reaches the answer, not just a sentence about the layer: {out}"
    );
}

#[test]
fn rainforest_understory_is_read_off_its_own_sentence() {
    assert_layer_row(
        "rainforestunderstory",
        "understory",
        "Located several meters below the canopy, the understory is an even darker, stiller and more humid environment.",
        "consensus",
    );
}

/// TWO ROWS REST ON A READING, and the tier says so.
///
/// Measured 2026-09-13 UTC on the cited page's rendered text: "tallest" occurs
/// ZERO times and "treetop" occurs ZERO times. The page says "The top layer of
/// the rainforest is the emergent layer" and "trees as tall as 60 meters"; it
/// says the canopy is "a deep layer of vegetation" whose leaves form a "roof".
/// The atoms `tallest_trees_...` and `deep_treetop_...` restate those rather
/// than quote them, so both rows carry `trust inferred`.
#[test]
fn rainforest_emergent_and_canopy_are_reasoned_not_read() {
    let out = assert_layer_row(
        "rainforestemergent",
        "emergent",
        "The top layer of the rainforest is the emergent layer. Here, trees as tall as 60 meters (200 feet) dominate the skyline.",
        "inferred",
    );
    assert!(
        !out.contains("\"trust\":\"consensus\""),
        "a reasoned row does not claim the read tier: {out}"
    );

    assert_layer_row(
        "rainforestcanopy",
        "canopy",
        "Beneath the emergent layer is the canopy, a deep layer of vegetation roughly six meters (20 feet) thick.",
        "inferred",
    );
}

/// The envelope is unreachable by construction -- every row overrides
/// `source` -- so no test can see its wording. The property is what is
/// pinnable, and a leak back into an answer is what would redden.
#[test]
fn rainforest_envelope_never_reaches_an_answer() {
    let dir = scratch("rainforestenvelope");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"rainforest-layer.adj\"\n? rainforest_layer($Layer, $D)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        4,
        "all four layers answer: {out}"
    );
    assert!(
        !out.contains("Most rainforests are structured in four layers"),
        "the framing span warrants no row: {out}"
    );
}

/// Parse every row-level `source` and `locator` out of a shipped `.adj`.
fn row_fields(rel: &str) -> (Vec<String>, Vec<String>) {
    let adj = std::fs::read_to_string(facts_stdlib().join(rel))
        .unwrap_or_else(|e| panic!("read shipped {rel}: {e}"));
    let mut spans: Vec<String> = Vec::new();
    let mut locators: Vec<String> = Vec::new();
    for line in adj.lines() {
        if let Some(rest) = line.strip_prefix(r#"        source ""#) {
            spans.push(rest.trim_end_matches(0x22 as char).to_string());
        } else if let Some(rest) = line.strip_prefix(r#"        locator ""#) {
            locators.push(rest.trim_end_matches(0x22 as char).to_string());
        }
    }
    (spans, locators)
}

/// #15193. NO ROW HERE SHARES A SPAN — asserted against the SHIPPED FILE, the
/// same way the sharing tables assert which rows share one.
///
/// Four rows, four sentences, one page.
#[test]
fn no_two_rows_share_a_span() {
    let (spans, locators) = row_fields("biology/rainforest-layer.adj");
    assert_eq!(spans.len(), 4, "every row carries its own source: {spans:?}");
    let mut uniq = spans.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(
        uniq.len(),
        4,
        "and no two rows share one: {spans:?}"
    );
    // ONE PAGE, and the file says so once: no row carries a `locator`, so
    // every row inherits the envelope's. Pinned because a row-level locator
    // appearing here would mean this table had quietly become multi-page.
    assert!(
        locators.is_empty(),
        "no row carries its own locator; all inherit the envelope's: {locators:?}"
    );
}
