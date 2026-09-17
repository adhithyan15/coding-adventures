//! End-to-end test for the physics FACTS library
//! (`adj-facts-stdlib/physics/band-emitter.adj`) driven through the built
//! CLI: a native `table` recording, for three of the seven electromagnetic-
//! spectrum bands already tabled in `em-spectrum.adj`, WHO or WHAT the
//! same already-cited NASA sentence states emits that band -- a sibling
//! decoding the emitter half of three already-verified quotes. Resolves
//! forward and backward recall queries with the source's citation, plus
//! honest abstention on a band whose cited span states a use but no
//! emitter -- 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the ultraviolet sentence, the primary source of all three answers, with the
//! infrared and visible spans as table-level `cites`; it is now a framing
//! sentence from the page, which every row overrides. The visible row carries
//! only the fireflies sentence, which names its emitter. Each answer's
//! citations array is pinned whole.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_bandemitter_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("physics/band-emitter.adj");
    std::fs::copy(&src, dir.join("band-emitter.adj"))
        .expect("copy shipped band-emitter.adj");
}

const LOCATOR: &str = "https://imagine.gsfc.nasa.gov/science/toolbox/emspectrum1.html";
const ENVELOPE: &str = "The electromagnetic (EM) spectrum is the range of all types of EM radiation.";
const ULTRAVIOLET: &str = "Ultraviolet radiation is emitted by the Sun and are the reason skin tans and burns.";
const INFRARED: &str = "Night vision goggles pick up the infrared light emitted by our skin and objects with heat.";
const VISIBLE: &str = "Fireflies, light bulbs, and stars all emit visible light.";
const VISIBLE_DETECTOR: &str = "Our eyes detect visible light.";

/// (band, emitter, that row's own sentence)
const ROWS: [(&str, &str, &str); 3] = [
    ("ultraviolet", "sun", ULTRAVIOLET),
    ("infrared", "skin_and_heat_objects", INFRARED),
    ("visible", "fireflies", VISIBLE),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(sentence: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("physics/band-emitter.adj"))
        .expect("read shipped band-emitter.adj");
    adj[adj.find("table band_emitter").expect("table")..].to_string()
}

#[test]
fn band_emitter_recalls_ultraviolet_emitter_with_citation() {
    let dir = scratch("uv");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"band-emitter.adj\"\n\
         ? band_emitter(ultraviolet, $Emitter)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"band_emitter(ultraviolet, sun)\""),
        "ultraviolet should recall its cited emitter: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("imagine.gsfc.nasa.gov") && contains(trust)` (#15209).
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(out.contains(&only_citation(ULTRAVIOLET)), "the ultraviolet sentence is the only citation: {out}");
}

#[test]
fn band_emitter_backward_recalls_ultraviolet_for_sun() {
    let dir = scratch("sun");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"band-emitter.adj\"\n\
         ? band_emitter($Band, sun)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"band_emitter(ultraviolet, sun)\""),
        "ultraviolet should be the only recalled sun-emitted band: {out}"
    );
    assert!(
        !out.contains("band_emitter(infrared, sun)"),
        "infrared's cited emitter is skin and heat objects, not the sun: {out}"
    );
}

#[test]
fn band_emitter_abstains_on_radio() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"band-emitter.adj\"\n\
         ? band_emitter(radio, $EmitterRadio)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "radio's cited span states a use (radio stations), not an emitter -- honest abstention expected: {out}"
    );
}

#[test]
fn every_emitter_answer_carries_its_own_sentence() {
    for (band, emitter, sentence) in ROWS {
        let dir = scratch(&format!("row_{band}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"band-emitter.adj\"\n? band_emitter($Band, {emitter})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {emitter}: {out}");
        assert!(out.contains(&format!("\"term\":\"band_emitter({band}, {emitter})\"")), "{emitter} emits {band}: {out}");
        assert!(out.contains(&only_citation(sentence)), "{band}: its own sentence, whole, and the only citation: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != band {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {band}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {band}: {out}");
    }
}

#[test]
fn the_visible_row_carries_the_sentence_that_names_its_emitter() {
    // "Our eyes detect visible light." names a detector, not an emitter; the
    // fireflies sentence names both the band and its emitter, so the row carries
    // it alone.
    let body = shipped_table();
    assert!(body.contains(&format!("    row (visible, fireflies) {{\n        source \"{VISIBLE}\"\n    }}")));
    assert!(!body.contains(VISIBLE_DETECTOR), "the detector sentence is not in the table");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (band, emitter, sentence) in ROWS {
        let expected = format!("    row ({band}, {emitter}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({band}, {emitter}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("cites \""), "no corroboration at row or table level");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{ULTRAVIOLET}\"\n    locator")), "not the ultraviolet sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["ultraviolet", "infrared", "visible", "sun", "skin", "heat", "firefl", "bulb", "emit"] {
        assert!(!folded.contains(word), "the envelope must name no band or emitter, but contains {word:?}");
    }
}
