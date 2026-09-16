//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/heart-chamber-vessel.adj`) driven through the
//! built CLI: a native `table` naming the vessel(s)/valve(s) each heart
//! chamber connects to, decoded from spans already sitting unused inside
//! the SAME StatPearls quotes `heart-chambers.adj`'s own header already
//! reproduces -- a sibling to that table, and a genuinely different axis
//! from `heart-valves.adj`'s valve-keyed `valve_separates` table. Resolves
//! binding-query recall (both directions) with the source's citation, and
//! abstains on a real cardiac structure (septum) that is not one of the
//! four chambers -- 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SPAN (RS-5e, #14986). The envelope used to be the
//! right-atrium sentence, the primary source of all six answers, with the
//! other chambers' sentences as table-level `cites`; it is now a framing
//! sentence from the page, which every row overrides. Each answer's citations
//! array is pinned whole.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_heartchambervessel_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/heart-chamber-vessel.adj");
    std::fs::copy(&src, dir.join("heart-chamber-vessel.adj"))
        .expect("copy shipped heart-chamber-vessel.adj");
}

const LOCATOR: &str = "https://www.ncbi.nlm.nih.gov/books/NBK470256/";
const ENVELOPE: &str = "The heart is a muscular organ situated in the center of the chest behind the sternum.";
const RIGHT_ATRIUM: &str = "The right atrium receives deoxygenated blood from the entire body except for the lungs (the systemic circulation) via the superior and inferior vena cavae.";
const RIGHT_VENTRICLE: &str = "The right ventricle pumps blood through the right ventricular outflow tract, across the pulmonic valve, and into the pulmonary artery that distributes it to the lungs for oxygenation.";
const LEFT_ATRIUM: &str = "This oxygenated blood is collected by the four pulmonary veins, two from each lung. All four of these veins open into the left atrium that acts as a collection chamber for oxygenated blood.";
const LEFT_VENTRICLE: &str = "The left ventricle is the main pumping chamber of the left heart, then pumps, sending freshly oxygenated blood to the systemic circulation through the aortic valve.";

/// (chamber, vessel, that row's own span)
const ROWS: [(&str, &str, &str); 6] = [
    ("right_atrium", "superior_vena_cava", RIGHT_ATRIUM),
    ("right_atrium", "inferior_vena_cava", RIGHT_ATRIUM),
    ("right_ventricle", "pulmonic_valve", RIGHT_VENTRICLE),
    ("right_ventricle", "pulmonary_artery", RIGHT_VENTRICLE),
    ("left_atrium", "pulmonary_veins", LEFT_ATRIUM),
    ("left_ventricle", "aortic_valve", LEFT_VENTRICLE),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(span: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/heart-chamber-vessel.adj"))
        .expect("read shipped heart-chamber-vessel.adj");
    adj[adj.find("table heart_chamber_vessel").expect("table")..].to_string()
}

#[test]
fn heart_chamber_vessel_recalls_forward_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heart-chamber-vessel.adj\"\n\
         ? heart_chamber_vessel(left_ventricle, $Vessel)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // THE WHOLE CITATIONS ARRAY, not a fragment (#13916, #13918). This used
    // to pin the RIGHT-ATRIUM sentence on a left-ventricle answer: the
    // envelope was primary for every row (#14986). The answer now carries
    // the left-ventricle sentence, and nothing else.
    assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer: {out}");
    assert!(
        out.contains(&only_citation(LEFT_VENTRICLE)),
        "the left-ventricle sentence is the only citation, whole: {out}"
    );
    assert!(!out.contains(RIGHT_ATRIUM), "the right-atrium sentence no longer reaches this answer: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"heart_chamber_vessel(left_ventricle, aortic_valve)\""),
        "the left ventricle connects to the aortic valve: {out}"
    );
}

#[test]
fn heart_chamber_vessel_recalls_backward_both_right_atrium_vessels() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heart-chamber-vessel.adj\"\n\
         ? heart_chamber_vessel($Chamber, superior_vena_cava)\n\
         ? heart_chamber_vessel($Chamber2, inferior_vena_cava)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"heart_chamber_vessel(right_atrium, superior_vena_cava)\""),
        "superior_vena_cava -> right_atrium: {out}"
    );
    assert!(
        out.contains("\"term\":\"heart_chamber_vessel(right_atrium, inferior_vena_cava)\""),
        "inferior_vena_cava -> right_atrium: {out}"
    );
    assert_eq!(out.matches("\"citations\":[").count(), 2, "two answers: {out}");
    assert_eq!(
        out.matches(&only_citation(RIGHT_ATRIUM)).count(),
        2,
        "both vena cavae carry only the right-atrium sentence: {out}"
    );
}

#[test]
fn heart_chamber_vessel_abstains_honestly_on_septum() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"heart-chamber-vessel.adj\"\n\
         ? heart_chamber_vessel(septum, $Vessel)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "septum is a real cardiac structure but not one of the four chambers -- honest abstention: {out}"
    );
}

#[test]
fn every_vessel_answer_carries_its_own_span() {
    for (chamber, vessel, span) in ROWS {
        let dir = scratch(&format!("row_{vessel}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"heart-chamber-vessel.adj\"\n? heart_chamber_vessel($Chamber, {vessel})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {vessel}: {out}");
        assert!(
            out.contains(&format!("\"term\":\"heart_chamber_vessel({chamber}, {vessel})\"")),
            "{vessel} connects to the {chamber}: {out}"
        );
        assert!(out.contains(&only_citation(span)), "{vessel}: its own span, whole, and the only citation: {out}");
        for (_, _, other_span) in ROWS {
            if other_span != span {
                assert!(!out.contains(other_span), "another chamber's span must not reach {vessel}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {vessel}: {out}");
    }
}

#[test]
fn the_left_atrium_row_carries_the_sentence_that_names_the_pulmonary_veins() {
    // The left-atrium sentence alone says only "All four of these veins"; the
    // sentence before it names the pulmonary veins. The row carries both,
    // contiguous, and never the "these veins" sentence alone.
    let body = shipped_table();
    assert!(body.contains(&format!("    row (left_atrium, pulmonary_veins) {{\n        source \"{LEFT_ATRIUM}\"\n    }}")));
    assert!(
        !body.contains("        source \"All four of these veins"),
        "no row is warranted by the 'these veins' sentence alone"
    );
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (chamber, vessel, span) in ROWS {
        let expected = format!("    row ({chamber}, {vessel}) {{\n        source \"{span}\"\n    }}");
        assert!(body.contains(&expected), "row ({chamber}, {vessel}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 6, "six row sources");
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
    assert!(
        !body.contains(&format!("\n    source \"{RIGHT_ATRIUM}\"\n    locator")),
        "not the right-atrium sentence as the envelope again"
    );
    let folded = ENVELOPE.to_lowercase();
    for word in ["atri", "ventric", "vena", "pulmon", "aort", "valve", "vein", "arter", "chamber"] {
        assert!(!folded.contains(word), "the envelope must name no chamber or vessel, but contains {word:?}");
    }
}
