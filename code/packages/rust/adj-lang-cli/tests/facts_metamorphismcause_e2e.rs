//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/metamorphism-cause.adj`) driven through
//! the built CLI: a native `table` naming three causes of rock metamorphism
//! and their shared effect, per USGS's "What are metamorphic rocks?" FAQ --
//! a sibling library to `rock-types.adj` but a genuinely different,
//! finer-grained causal axis. 0 answer-time model calls.
//!
//! EACH ROW CARRIES BOTH SENTENCES (RS-5e, #14986). The envelope used to be the
//! cause-naming sentence, and the sentence stating what metamorphism DOES was
//! quoted in the library's header but cited nowhere -- so no answer carried it.
//! Each row now takes the cause sentence as its own `source` and `cites` the
//! effect sentence, and the envelope is a framing sentence naming neither.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_metamorphismcause_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("earth-science/metamorphism-cause.adj");
    std::fs::copy(&src, dir.join("metamorphism-cause.adj")).expect("copy shipped metamorphism-cause.adj");
}

#[test]
fn metamorphism_cause_recall_binds_the_effect_directly() {
    let dir = scratch("direct");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"metamorphism-cause.adj\"\n\
         ? metamorphism_cause(heat, $Effect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"Effect\":\"denser_more_compact_rock\""),
        "heat's effect is a denser, more compact rock: {out}"
    );
    // This was `contains("usgs.gov") && contains("\"trust\":\"authoritative\"")`,
    // which any USGS page's citation satisfies -- and which a truncated `source`
    // satisfies just as happily, since it constrains no sentence text at all.
    // The needle is now the whole citations array as the serialiser emits it,
    // closing on both the corroborations `]` and the citations `]`.
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(
        out.contains(&only_citation()),
        "the cause sentence, whole, corroborated by the effect sentence: {out}"
    );
    assert!(!out.contains(ENVELOPE), "the framing sentence is primary for no answer: {out}");
}

#[test]
fn metamorphism_cause_reverse_binds_every_cause_for_that_effect() {
    let dir = scratch("reverse");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"metamorphism-cause.adj\"\n\
         ? metamorphism_cause($Cause, denser_more_compact_rock)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"Cause\":\"heat\"")
            && out.contains("\"Cause\":\"pressure\"")
            && out.contains("\"Cause\":\"hot_mineral_rich_fluids\""),
        "all three shipped causes should be enumerated: {out}"
    );
}

#[test]
fn metamorphism_cause_abstains_honestly_on_an_untabled_cause() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"metamorphism-cause.adj\"\n\
         ? metamorphism_cause(sunlight, $Effect)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "sunlight is not a shipped cause of metamorphism -- honest abstention, never invented: {out}"
    );
}

const LOCATOR: &str = "https://www.usgs.gov/faqs/what-are-metamorphic-rocks";
const CAUSE: &str = "Metamorphic rocks form when rocks are subjected to high heat, high pressure, hot mineral-rich fluids or, more commonly, some combination of these factors.";
const EFFECT: &str = "The process of metamorphism does not melt the rocks, but instead transforms them into denser, more compact rocks.";
/// The page writes U+00A0 before "igneous," and before "sedimentary,". Typed
/// with plain spaces this sentence occurs ZERO times in the page's text, so the
/// non-breaking spaces are part of the quote, not an artifact of extraction.
const ENVELOPE: &str = "Metamorphic rocks started out as some other type of rock, but have been substantially changed from their original\u{a0}igneous,\u{a0}sedimentary, or earlier metamorphic form.";
const CAUSES: [&str; 3] = ["heat", "pressure", "hot_mineral_rich_fluids"];

/// The whole citations array of a one-citation answer, as one contiguous run:
/// the cause sentence as `source`, the effect sentence as its corroboration,
/// closing on the corroborations `]` so a fabricated `cites` cannot be appended
/// without reddening the assertion (#14735).
fn only_citation() -> String {
    format!(
        "\"citations\":[{{\"source\":\"{CAUSE}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[{{\"source\":\"{EFFECT}\",\"locator\":\"{LOCATOR}\"}}]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("earth-science/metamorphism-cause.adj"))
        .expect("read shipped metamorphism-cause.adj");
    adj[adj.find("table metamorphism_cause").expect("table")..].to_string()
}

#[test]
fn every_cause_answer_carries_the_cause_sentence_and_the_effect_it_states() {
    // The effect sentence used to be quoted in the header and cited NOWHERE, so
    // no answer carried the sentence stating what metamorphism actually does to
    // the rock. Each row now cites it, and the envelope -- which states neither
    // a cause nor the effect -- reaches no answer.
    for cause in CAUSES {
        let dir = scratch(&format!("cause_{cause}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"metamorphism-cause.adj\"\n? metamorphism_cause({cause}, $Effect)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {cause}: {out}");
        assert!(
            out.contains("\"Effect\":\"denser_more_compact_rock\""),
            "{cause} -> denser_more_compact_rock: {out}"
        );
        assert!(
            out.contains(&only_citation()),
            "{cause}: the cause sentence, whole, corroborated by the effect sentence: {out}"
        );
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {cause}: {out}");
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it -- including its two
    // non-breaking spaces, which a "whitespace cleanup" would otherwise turn
    // into plain spaces, leaving a `source` that occurs zero times on the page.
    let body = shipped_table();
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert_eq!(body.matches("\n        cites \"").count(), 3, "three row corroborations");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!(
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n"
        )),
        "the envelope is the page's framing sentence, in the page's own characters"
    );
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line");
    assert_eq!(
        shipped_envelope.matches('\u{a0}').count(),
        2,
        "the envelope keeps the page's two non-breaking spaces: {shipped_envelope}"
    );
    let lowered = shipped_envelope.to_lowercase();
    for word in ["heat", "pressure", "fluid", "denser", "compact"] {
        assert!(
            !lowered.contains(word),
            "the shipped envelope must name no cause and no effect, but contains {word:?}"
        );
    }
    assert!(
        !body.contains(&format!("\n    source \"{CAUSE}\"\n    locator")),
        "not the cause sentence as the envelope again"
    );
}
