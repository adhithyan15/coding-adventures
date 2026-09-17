//! End-to-end test for the astronomy FACTS library
//! (`adj-facts-stdlib/astronomy/lunar-eclipse-type.adj`) driven through the
//! built CLI: a native `table` naming the three named lunar eclipse types
//! and what each actually is, quoted from NASA's "Eclipses and the Moon"
//! page. 0 answer-time model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the total row's own sentence, the primary source of all three answers; it
//! is now a framing sentence from the page, which every row overrides. Each
//! answer's citations array is pinned whole, with the page's mixed
//! apostrophes (U+2019 in the total and penumbral sentences, U+0027 in the
//! partial one).

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_lunar_eclipse_type_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("astronomy/lunar-eclipse-type.adj");
    std::fs::copy(&src, dir.join("lunar-eclipse-type.adj")).expect("copy shipped lunar-eclipse-type.adj");
}

const LOCATOR: &str = "https://science.nasa.gov/moon/eclipses/";
const ENVELOPE: &str = "Lunar eclipses occur at the full Moon phase.";
const TOTAL: &str = "The Moon moves into the inner part of Earth\u{2019}s shadow, or the umbra.";
const PARTIAL: &str = "An imperfect alignment of Sun, Earth and Moon results in the Moon passing through only part of Earth's umbra.";
const PENUMBRAL: &str = "The Moon travels through Earth\u{2019}s penumbra, or the faint outer part of its shadow.";

/// (type, description, that row's own sentence)
const ROWS: [(&str, &str, &str); 3] = [
    ("total_lunar_eclipse", "the_moon_moves_into_the_inner_part_of_earths_shadow_the_umbra", TOTAL),
    ("partial_lunar_eclipse", "an_imperfect_alignment_of_sun_earth_and_moon_results_in_partial_umbra_passage", PARTIAL),
    ("penumbral_eclipse", "the_moon_travels_through_earths_penumbra_the_faint_outer_part_of_its_shadow", PENUMBRAL),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(sentence: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("astronomy/lunar-eclipse-type.adj"))
        .expect("read shipped lunar-eclipse-type.adj");
    adj[adj.find("table lunar_eclipse_type").expect("table")..].to_string()
}

#[test]
fn lunar_eclipse_type_recall_binds_the_description_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"lunar-eclipse-type.adj\"\n\
         ? lunar_eclipse_type(total_lunar_eclipse, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"D\":\"the_moon_moves_into_the_inner_part_of_earths_shadow_the_umbra\""),
        "total_lunar_eclipse means the_moon_moves_into_the_inner_part_of_earths_shadow_the_umbra: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("science.nasa.gov") && contains(trust)` (#15209).
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(out.contains(&only_citation(TOTAL)), "the total sentence is the only citation: {out}");
}

#[test]
fn lunar_eclipse_type_reverse_binds_the_type_for_that_description() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"lunar-eclipse-type.adj\"\n\
         ? lunar_eclipse_type($T, the_moon_travels_through_earths_penumbra_the_faint_outer_part_of_its_shadow)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"T\":\"penumbral_eclipse\""),
        "the shipped the_moon_travels_through_earths_penumbra example is penumbral_eclipse: {out}"
    );
}

#[test]
fn lunar_eclipse_type_abstains_honestly_on_an_untabled_nickname() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"lunar-eclipse-type.adj\"\n\
         ? lunar_eclipse_type(blood_moon, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "blood_moon is a real term the source discusses, but as a nickname for the color effect, not a fourth peer eclipse type -- honest abstention, never invented: {out}"
    );
}

const LUNAR_PIN: &str = r#""bindings":{"D":"the_moon_moves_into_the_inner_part_of_earths_shadow_the_umbra"},"citations":[{"source":"The Moon moves into the inner part of Earth’s shadow, or the umbra.","locator":"https://science.nasa.gov/moon/eclipses/","trust":"authoritative""#;

#[test]
fn lunar_eclipse_type_citation_keeps_the_pages_curly_apostrophe() {
    let dir = scratch("cite_lunar");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"lunar-eclipse-type.adj\"\n? lunar_eclipse_type(total_lunar_eclipse, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The shipped citation carried an ASCII apostrophe where the page renders
    // U+2019, so it did not appear on its own page -- the whole premise being
    // that a caller can check a citation against its locator. The replacement
    // text was taken FROM the page (a candidate swap applied, then confirmed
    // present in a rendered block), not hand-curled.
    assert!(
        out.contains(LUNAR_PIN),
        "total lunar eclipse's citation matches its page: {out}"
    );
}

#[test]
fn every_type_answer_carries_its_own_sentence() {
    for (kind, desc, sentence) in ROWS {
        let dir = scratch(&format!("row_{kind}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"lunar-eclipse-type.adj\"\n? lunar_eclipse_type($T, {desc})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {kind}: {out}");
        assert!(out.contains(&format!("\"T\":\"{kind}\"")), "{desc} binds {kind}: {out}");
        assert!(out.contains(&only_citation(sentence)), "{kind}: its own sentence, whole, and the only citation: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != kind {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {kind}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {kind}: {out}");
    }
}

#[test]
fn the_rows_keep_the_pages_mixed_apostrophes() {
    // The page writes U+2019 in the total and penumbral sentences and U+0027 in
    // the partial one; a blanket curl or straighten would make a row stop
    // appearing on its own page (#14070).
    let body = shipped_table();
    assert!(body.contains(&format!("        source \"{TOTAL}\"\n")) && TOTAL.contains('\u{2019}'));
    assert!(body.contains(&format!("        source \"{PENUMBRAL}\"\n")) && PENUMBRAL.contains('\u{2019}'));
    assert!(body.contains(&format!("        source \"{PARTIAL}\"\n")) && PARTIAL.contains("Earth's"));
    assert!(!body.contains("Earth\u{2019}s umbra."), "the partial sentence is not curled");
    assert!(!body.contains("Earth's shadow, or") && !body.contains("Earth's penumbra"), "the others are not straightened");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (kind, desc, sentence) in ROWS {
        let expected = format!("    row ({kind}, {desc}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({kind}, {desc}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("cites "), "no corroboration at row or table level");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{TOTAL}\"\n    locator")), "not the total sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["total", "partial", "penumbr", "umbra", "shadow", "align", "blood"] {
        assert!(!folded.contains(word), "the envelope must name no type or description, but contains {word:?}");
    }
}
