//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/heart-chambers.adj`) driven through the built CLI:
//! a native `table` of heart-chamber → function resolves binding-query recalls
//! (forward and backward) with the source's NIH citation, and abstains on a
//! non-chamber (the aorta) — 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SPAN (RS-5e, #14986). The envelope used to be the
//! right-atrium sentence, the primary source of all four answers; it is now a
//! framing sentence from the page, which every row overrides. The
//! left-ventricle row also cites the right-atrium sentence, the page's only
//! gloss of "the systemic circulation" as the body. Each answer's citations
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
    let dir = std::env::temp_dir().join(format!("adjcli_factsk_{tag}_{}", std::process::id()));
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

const LOCATOR: &str = "https://www.ncbi.nlm.nih.gov/books/NBK470256/";
const ENVELOPE: &str = "The heart is a muscular organ situated in the center of the chest behind the sternum.";
const RIGHT_ATRIUM: &str = "The right atrium receives deoxygenated blood from the entire body except for the lungs (the systemic circulation) via the superior and inferior vena cavae.";
const RIGHT_VENTRICLE: &str = "The right ventricle pumps blood through the right ventricular outflow tract, across the pulmonic valve, and into the pulmonary artery that distributes it to the lungs for oxygenation.";
const LEFT_ATRIUM: &str = "This oxygenated blood is collected by the four pulmonary veins, two from each lung. All four of these veins open into the left atrium that acts as a collection chamber for oxygenated blood.";
const LEFT_VENTRICLE: &str = "The left ventricle is the main pumping chamber of the left heart, then pumps, sending freshly oxygenated blood to the systemic circulation through the aortic valve.";

/// (chamber, job, that row's own span, the spans it cites)
const ROWS: [(&str, &str, &str, &[&str]); 4] = [
    ("right_atrium", "receives_blood_from_body", RIGHT_ATRIUM, &[]),
    ("right_ventricle", "pumps_blood_to_lungs", RIGHT_VENTRICLE, &[]),
    ("left_atrium", "receives_blood_from_lungs", LEFT_ATRIUM, &[]),
    ("left_ventricle", "pumps_blood_to_body", LEFT_VENTRICLE, &[RIGHT_ATRIUM]),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(span: &str, cites: &[&str]) -> String {
    let corr: Vec<String> = cites
        .iter()
        .map(|s| format!("{{\"source\":\"{s}\",\"locator\":\"{LOCATOR}\"}}"))
        .collect();
    format!(
        "\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[{}]}}]",
        corr.join(",")
    )
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/heart-chambers.adj"))
        .expect("read shipped heart-chambers.adj");
    adj[adj.find("table heart_chamber_function").expect("table")..].to_string()
}

fn ask(tag: &str, query: &str) -> String {
    let dir = scratch(tag);
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/heart-chambers.adj");
    std::fs::copy(&src, dir.join("heart-chambers.adj")).expect("copy shipped heart-chambers.adj");
    std::fs::write(dir.join("case.adj"), format!("import \"heart-chambers.adj\"\n{query}")).unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn anatomy_heart_chambers_recall_binds_function_with_citation() {
    let out = ask(
        "heartchambers",
        "? heart_chamber_function(right_atrium, $Job)\n\
         ? heart_chamber_function(right_ventricle, $Job)\n\
         ? heart_chamber_function($C, pumps_blood_to_body)\n\
         ? heart_chamber_function(aorta, $Job)\n",
    );
    // THE WHOLE CITATIONS ARRAY, not a fragment (#13916, #13918). This used to
    // pin the right-atrium sentence as "the" citation of all three answers,
    // true only because the envelope was primary for every row (#14986).
    assert_eq!(out.matches("\"citations\":[").count(), 3, "three answers: {out}");
    assert!(out.contains(&only_citation(RIGHT_ATRIUM, &[])), "right atrium: its own sentence: {out}");
    assert!(out.contains(&only_citation(RIGHT_VENTRICLE, &[])), "right ventricle: its own sentence: {out}");
    assert!(
        out.contains(&only_citation(LEFT_VENTRICLE, &[RIGHT_ATRIUM])),
        "left ventricle: its own sentence, corroborated by the right-atrium gloss: {out}"
    );
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The right atrium receives blood from the body; the right ventricle pumps
    // it on to the lungs — the recalled jobs (forward binds).
    assert!(
        out.contains("\"Job\":\"receives_blood_from_body\""),
        "right_atrium → receives_blood_from_body: {out}"
    );
    assert!(
        out.contains("\"Job\":\"pumps_blood_to_lungs\""),
        "right_ventricle → pumps_blood_to_lungs: {out}"
    );
    // The relation runs BACKWARD: bind the job, recall the chamber.
    assert!(
        out.contains("\"C\":\"left_ventricle\""),
        "pumps_blood_to_body → left_ventricle (reverse recall): {out}"
    );
    // The aorta is not a chamber — honest abstention, never a fabricated job.
    assert!(out.contains("\"abstained\":true"), "aorta abstains: {out}");
}

#[test]
fn every_job_answer_carries_its_own_span() {
    for (chamber, job, span, cites) in ROWS {
        let out = ask(&format!("row_{chamber}"), &format!("? heart_chamber_function($C, {job})\n"));
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {job}: {out}");
        assert!(out.contains(&format!("\"C\":\"{chamber}\"")), "{job} is the {chamber}'s job: {out}");
        assert!(out.contains(&only_citation(span, cites)), "{chamber}: its own span, whole, and the only citation: {out}");
        for (_, _, other_span, _) in ROWS {
            if other_span != span && !cites.contains(&other_span) {
                assert!(!out.contains(other_span), "another chamber's span must not reach {chamber}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {chamber}: {out}");
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (chamber, job, span, cites) in ROWS {
        let mut expected = format!("    row ({chamber}, {job}) {{\n        source \"{span}\"\n");
        for c in cites {
            expected.push_str(&format!("        cites \"{c}\" locator \"{LOCATOR}\"\n"));
        }
        expected.push_str("    }");
        assert!(body.contains(&expected), "row ({chamber}, {job}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 4, "four row sources");
    assert_eq!(body.matches("\n        cites \"").count(), 1, "one row corroboration, the left ventricle's");
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
    for word in ["atri", "ventric", "chamber", "lung", "blood", "body", "pump", "receiv"] {
        assert!(!folded.contains(word), "the envelope must name no chamber or job, but contains {word:?}");
    }
}
