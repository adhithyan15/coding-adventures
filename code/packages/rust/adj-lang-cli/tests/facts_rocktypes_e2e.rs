//! End-to-end test for the earth-science FACTS library
//! (`adj-facts-stdlib/earth-science/rock-types.adj`) driven through the built
//! CLI: a native `table` of rock type → how it forms resolves a binding-query
//! recall with the NPS citation, and abstains on `magma` (the molten material
//! rock forms FROM, not one of the three rock types) — 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the IGNEOUS sentence, so a sedimentary or metamorphic answer's primary
//! source was a sentence about igneous rock. Each row now carries the sentence
//! that names its own type -- and the metamorphic row carries two, because
//! "This normally happens deep underground..." names no rock type on its own.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsrt_{tag}_{}", std::process::id()));
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

#[test]
fn rock_types_recall_binds_formation_with_citation() {
    let dir = scratch("rocktypes");
    // Copy the shipped earth-science table beside the entry program and import it.
    let src = facts_stdlib().join("earth-science/rock-types.adj");
    std::fs::copy(&src, dir.join("rock-types.adj")).expect("copy shipped rock-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"rock-types.adj\"\n\
         ? rock_formation(igneous, $How)\n\
         ? rock_formation(metamorphic, $How)\n\
         ? rock_formation(magma, $How)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Igneous rock is cooled magma; metamorphic rock is made by heat and
    // pressure — the recalled formations, straight from the grounded rows.
    assert!(out.contains("\"How\":\"cooled_magma\""), "igneous → cooled_magma: {out}");
    assert!(
        out.contains("\"How\":\"heat_and_pressure\""),
        "metamorphic → heat_and_pressure: {out}"
    );
    // Each answer carries its OWN sentence as proof, pinned as text.
    //
    // This was `contains("nps.gov") && contains("\"trust\":\"authoritative\"")`,
    // which any NPS page's citation satisfies and which says nothing about what
    // the `source` field contains -- a truncated sentence passes it just as
    // happily. The needles below are whole citations arrays as the serialiser
    // emits them, closing on both the corroborations `]` and the citations `]`.
    assert_eq!(out.matches("\"citations\":[").count(), 2, "two answers bind: {out}");
    assert!(
        out.contains(&only_citation(IGNEOUS)),
        "igneous carries its own sentence, whole, and nothing else: {out}"
    );
    assert!(
        out.contains(&only_citation(METAMORPHIC)),
        "metamorphic carries its naming sentence and the one that points back: {out}"
    );
    // Neither answer is sedimentary, so that sentence must not ride along.
    assert!(!out.contains(SEDIMENTARY), "the sedimentary sentence reaches neither answer: {out}");
    assert!(!out.contains(ENVELOPE), "the envelope is primary for no answer: {out}");
    // Magma is the molten material rock forms FROM, not a rock type — honest
    // abstention, never a fabricated formation.
    assert!(out.contains("\"abstained\":true"), "magma abstains: {out}");
}

const LOCATOR: &str =
    "https://www.nps.gov/teachers/classrooms/olympic-geology-pt-2-rock-sorting.htm";
const ENVELOPE: &str =
    "Geologists: scientists who study rocks and recognize three major groups of rocks.";
const IGNEOUS: &str = "Igneous Rocks: form when hot, liquid rock, or magma, cools.";
const SEDIMENTARY: &str = "With the help of time and external pressures, these sediments get compacted into sedimentary rock.";
/// Two sentences, contiguous in one paragraph on the page. The second -- "This
/// normally happens deep underground..." -- names no rock type on its own, so
/// the row carries the sentence it points back to as well.
const METAMORPHIC: &str = "Metamorphic Rocks: are created through the metamorphosis, or change, of other types of rocks. This normally happens deep underground where heat, pressure, and chemical activity can actually alter the minerals inside rocks.";
const META_TAIL: &str = "This normally happens deep underground where heat, pressure, and chemical activity can actually alter the minerals inside rocks.";

/// (rock type, its formation atom, the span that names both)
fn types() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("igneous", "cooled_magma", IGNEOUS),
        ("sedimentary", "compacted_sediment", SEDIMENTARY),
        ("metamorphic", "heat_and_pressure", METAMORPHIC),
    ]
}

/// The whole citations array of a one-citation answer, as one contiguous run.
/// It closes on the corroborations `]` and on the citations `]`, so a
/// fabricated `cites` cannot be appended without reddening it (#14735).
fn only_citation(span: &str) -> String {
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]"
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("earth-science/rock-types.adj"))
        .expect("read shipped rock-types.adj");
    adj[adj.find("table rock_formation").expect("table")..].to_string()
}

#[test]
fn every_rock_answer_carries_only_the_sentence_that_names_that_rock() {
    // The envelope used to be the IGNEOUS sentence, so a sedimentary or
    // metamorphic answer's primary source was a sentence about igneous rock.
    for (rock, how, span) in types() {
        let dir = scratch(&format!("rock_{rock}"));
        let src = facts_stdlib().join("earth-science/rock-types.adj");
        std::fs::copy(&src, dir.join("rock-types.adj")).expect("copy shipped rock-types.adj");
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"rock-types.adj\"\n? rock_formation({rock}, $How)\n"),
        )
        .unwrap();

        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {rock}: {out}");
        assert!(out.contains(&format!("\"How\":\"{how}\"")), "{rock} -> {how}: {out}");
        assert!(
            out.contains(&only_citation(span)),
            "{rock}: its own span, whole, and the only citation: {out}"
        );
        assert!(
            span.to_lowercase().contains(rock),
            "the span carried by {rock} names {rock} -- the defect was that it did not"
        );
        for other in [IGNEOUS, SEDIMENTARY, METAMORPHIC] {
            if other != span {
                assert!(
                    !out.contains(other),
                    "another rock's sentence must not reach {rock}: {out}"
                );
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {rock}: {out}");
    }
}

#[test]
fn the_metamorphic_row_carries_the_sentence_its_span_points_back_to() {
    // "This normally happens deep underground..." names no rock type, so a row
    // warranted by it alone would cite a sentence that cannot identify its own
    // subject. The row carries the naming sentence before it, contiguous in the
    // same paragraph on the page.
    let body = shipped_table();
    assert!(
        body.contains(&format!("        source \"{METAMORPHIC}\"\n")),
        "the metamorphic row carries both sentences: {body}"
    );
    assert!(
        !body.contains(&format!("        source \"{META_TAIL}\"\n")),
        "no row is warranted by the dangling sentence alone"
    );
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!(
            "\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n"
        )),
        "the envelope is the page's framing sentence"
    );
    assert!(
        !body.contains(&format!("\n    source \"{IGNEOUS}\"\n    locator")),
        "not the igneous sentence as the envelope again"
    );
    let shipped_envelope = body
        .lines()
        .find(|l| l.starts_with("    source \""))
        .expect("the table has an envelope source line")
        .to_lowercase();
    for word in ["igneous", "sedimentary", "metamorphic", "magma", "sediment", "heat", "pressure"] {
        assert!(
            !shipped_envelope.contains(word),
            "the shipped envelope must name no rock type and no formation, but contains {word:?}"
        );
    }
}
