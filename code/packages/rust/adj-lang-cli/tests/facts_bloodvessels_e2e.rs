//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/blood-vessels.adj`) driven through the built CLI:
//! a native `table` of the three main blood-vessel types → the defining
//! function each performs resolves binding-query recalls (forward AND backward)
//! with the source's NCI SEER Training Modules citation, and abstains on a word
//! that is not one of the three main blood vessels (a lymphatic vessel) — 0
//! model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the artery sentence, the primary source of all three answers; it is now the
//! page's framing sentence, which every row overrides. Each row's citation is
//! pinned whole.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsbv_{tag}_{}", std::process::id()));
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

const LOCATOR: &str = "https://training.seer.cancer.gov/anatomy/cardiovascular/blood/classification.html";
const ENVELOPE: &str = "Blood vessels are the channels or conduits through which blood is distributed to body tissues.";
const ARTERY: &str = "Arteries carry blood away from the heart.";
const VEIN: &str = "Veins carry blood toward the heart.";
const CAPILLARY: &str = "The primary function of capillaries is the exchange of materials between the blood and tissue cells.";

/// (vessel, function, that row's own sentence)
const ROWS: [(&str, &str, &str); 3] = [
    ("artery", "away_from_heart", ARTERY),
    ("vein", "toward_heart", VEIN),
    ("capillary", "exchange_of_materials", CAPILLARY),
];

/// The whole citation a row's answer carries, as one contiguous run.
fn citation(sentence: &str) -> String {
    format!("\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]")
}

fn place(dir: &Path) {
    let src = facts_stdlib().join("biology/blood-vessels.adj");
    std::fs::copy(&src, dir.join("blood-vessels.adj")).expect("copy shipped blood-vessels.adj");
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/blood-vessels.adj")).expect("read shipped blood-vessels.adj");
    adj[adj.find("table vessel_function").expect("table")..].to_string()
}

#[test]
fn biology_blood_vessels_recall_binds_function_with_citation() {
    let dir = scratch("bloodvessels");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"blood-vessels.adj\"\n\
         ? vessel_function(artery, $Function)\n\
         ? vessel_function(vein, $Function)\n\
         ? vessel_function(capillary, $Function)\n\
         ? vessel_function($Vessel, toward_heart)\n\
         ? vessel_function(lymphatic, $Function)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // FULL ANCHORED CITATION PIN, now the whole contiguous run for the artery
    // row: sentence, locator, tier and no corroborations (#13916, #13918).
    assert!(out.contains(&citation(ARTERY)), "the artery citation is its whole sentence, exactly: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Arteries carry blood away from the heart, veins carry blood toward the
    // heart, and capillaries are where the exchange of materials happens — the
    // recalled functions (forward binds).
    assert!(
        out.contains("\"Function\":\"away_from_heart\""),
        "artery → away_from_heart: {out}"
    );
    assert!(
        out.contains("\"Function\":\"toward_heart\""),
        "vein → toward_heart: {out}"
    );
    assert!(
        out.contains("\"Function\":\"exchange_of_materials\""),
        "capillary → exchange_of_materials: {out}"
    );
    // The relation runs BACKWARD: bind the function `toward_heart`, recall its
    // vessel type.
    assert!(
        out.contains("\"Vessel\":\"vein\""),
        "toward_heart → vein (reverse recall): {out}"
    );
    // A lymphatic vessel is not one of the three main blood vessels — honest
    // abstention, never a fabricated function.
    assert!(
        out.contains("\"abstained\":true"),
        "lymphatic abstains: {out}"
    );
}

#[test]
fn every_vessel_answer_carries_its_own_sentence() {
    for (vessel, function, sentence) in ROWS {
        let dir = scratch(&format!("row_{vessel}"));
        place(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"blood-vessels.adj\"\n? vessel_function({vessel}, $F)\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {vessel}: {out}");
        assert!(out.contains(&format!("\"F\":\"{function}\"")), "{vessel} → {function}: {out}");
        assert!(out.contains(&citation(sentence)), "{vessel}: its own sentence, whole: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != vessel {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {vessel}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {vessel}: {out}");
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (vessel, function, sentence) in ROWS {
        let expected = format!("    row ({vessel}, {function}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({vessel}, {function}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("\n        cites "), "no row corroboration");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{ARTERY}\"\n    locator")), "not the artery sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["arter", "vein", "capillar", "away", "toward", "exchange"] {
        assert!(!folded.contains(word), "the envelope must name no vessel or function, but contains {word:?}");
    }
}
