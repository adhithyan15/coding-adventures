//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/joint-formed-by.adj`) driven through the
//! built CLI: a native `table` naming the actual bones that meet to form
//! three synovial-joint types, from the StatPearls "Anatomy, Joints" page
//! that `joint-types.adj` also cites -- a sibling to that table. Resolves binding-query recall (both
//! directions, including a 2-answer forward recall), and abstains on a
//! real, already-tabled joint type (hinge) whose own quote names only an
//! example joint, never its forming bones -- 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SPAN (RS-5e, #14986). The envelope used to be the
//! pivot sentence, the primary source of all six answers, with the condyloid
//! and saddle spans as table-level `cites`; it is now a framing sentence from
//! the page, which every row overrides. Each answer's citations array is
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
    let dir = std::env::temp_dir().join(format!("adjcli_jointformedby_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("anatomy/joint-formed-by.adj");
    std::fs::copy(&src, dir.join("joint-formed-by.adj")).expect("copy shipped joint-formed-by.adj");
}

const LOCATOR: &str = "https://www.ncbi.nlm.nih.gov/books/NBK507893/";
const ENVELOPE: &str = "Joint classifications offer a broad understanding of joints.";
const PIVOT: &str = "The atlantoaxial joint, formed by the 1st (atlas) and 2nd (axis) cervical vertebrae, is a pivot joint.";
const CONDYLOID: &str = "Examples of condyloid joints are the knuckles, formed by the distal metacarpals and proximal phalanges of the medial 4 fingers.";
const SADDLE: &str = "A saddle joint is an articulation between 2 saddle-shaped bones, which are concave in one direction and convex in another. This joint type is biaxial. One example is the joint formed by the trapezium and 1st metacarpal bone.";

/// (joint type, bone, that row's own span)
const ROWS: [(&str, &str, &str); 6] = [
    ("pivot", "atlas", PIVOT),
    ("pivot", "axis", PIVOT),
    ("condyloid", "distal_metacarpals", CONDYLOID),
    ("condyloid", "proximal_phalanges", CONDYLOID),
    ("saddle", "trapezium", SADDLE),
    ("saddle", "first_metacarpal", SADDLE),
];

/// The whole citations array of a one-citation answer, as one contiguous run.
fn only_citation(span: &str) -> String {
    format!("\"citations\":[{{\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]}}]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/joint-formed-by.adj"))
        .expect("read shipped joint-formed-by.adj");
    adj[adj.find("table joint_formed_by").expect("table")..].to_string()
}

#[test]
fn joint_formed_by_recalls_forward_both_pivot_bones_with_citation() {
    let dir = scratch("forward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"joint-formed-by.adj\"\n\
         ? joint_formed_by(pivot, $Bone)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    for bone in ["atlas", "axis"] {
        assert!(
            out.contains(&format!("\"term\":\"joint_formed_by(pivot, {bone})\"")),
            "the pivot joint is formed by {bone}: {out}"
        );
    }
    // ONE CONTIGUOUS RUN, not `contains("ncbi.nlm.nih.gov") && contains(trust)` (#15209).
    assert_eq!(out.matches("\"citations\":[").count(), 2, "two answers: {out}");
    assert_eq!(out.matches(&only_citation(PIVOT)).count(), 2, "both pivot bones carry only the pivot sentence: {out}");
}

#[test]
fn joint_formed_by_recalls_backward_to_saddle() {
    let dir = scratch("backward");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"joint-formed-by.adj\"\n\
         ? joint_formed_by($Type, trapezium)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"joint_formed_by(saddle, trapezium)\""),
        "trapezium helps form the saddle joint: {out}"
    );
}

#[test]
fn joint_formed_by_abstains_honestly_on_hinge() {
    let dir = scratch("abstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"joint-formed-by.adj\"\n\
         ? joint_formed_by(hinge, $Bone)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"abstained\":true"),
        "hinge is a real, already-tabled joint type but its own quote names no forming bones -- honest abstention: {out}"
    );
}

#[test]
fn every_bone_answer_carries_its_own_span() {
    for (kind, bone, span) in ROWS {
        let dir = scratch(&format!("row_{bone}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"joint-formed-by.adj\"\n? joint_formed_by($Type, {bone})\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {bone}: {out}");
        assert!(out.contains(&format!("\"term\":\"joint_formed_by({kind}, {bone})\"")), "{bone} forms the {kind} joint: {out}");
        assert!(out.contains(&only_citation(span)), "{bone}: its own span, whole, and the only citation: {out}");
        for (_, _, other_span) in ROWS {
            if other_span != span {
                assert!(!out.contains(other_span), "another joint type's span must not reach {bone}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {bone}: {out}");
    }
}

#[test]
fn the_saddle_rows_carry_the_whole_three_sentence_span() {
    // The bone sentence alone ("One example is the joint formed by ...") names
    // no joint type, and "This joint type is biaxial." sits between it and the
    // saddle sentence, so the saddle rows carry all three, contiguous.
    let body = shipped_table();
    assert_eq!(body.matches(&format!("        source \"{SADDLE}\"\n")).count(), 2, "both saddle rows carry the whole span");
    assert!(
        !body.contains("        source \"One example is the joint formed by the trapezium and 1st metacarpal bone.\""),
        "no saddle row is warranted by the bone sentence alone"
    );
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (kind, bone, span) in ROWS {
        let expected = format!("    row ({kind}, {bone}) {{\n        source \"{span}\"\n    }}");
        assert!(body.contains(&expected), "row ({kind}, {bone}) is shipped in its measured shape");
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
    assert!(!body.contains(&format!("\n    source \"{PIVOT}\"\n    locator")), "not the pivot sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["pivot", "condyl", "saddle", "atlas", "axis", "metacarp", "phalan", "trapez", "knuckle", "bone"] {
        assert!(!folded.contains(word), "the envelope must name no joint type or bone, but contains {word:?}");
    }
}
