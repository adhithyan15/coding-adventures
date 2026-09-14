//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/joint-types.adj`) driven through the built CLI:
//! a native `table` of synovial joint type → representative example resolves a
//! binding-query recall with the source's StatPearls (NIH/NLM) citation, runs
//! the relation backward (joint type → examples, and example → joint type), and
//! abstains on a non-synovial joint (a skull suture) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsjoint_{tag}_{}", std::process::id()));
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
fn anatomy_joint_types_recall_binds_example_with_citation() {
    let dir = scratch("jointtypes");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/joint-types.adj");
    std::fs::copy(&src, dir.join("joint-types.adj")).expect("copy shipped joint-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"joint-types.adj\"\n\
         ? joint_example(hinge, $Ex)\n\
         ? joint_example(saddle, $Ex)\n\
         ? joint_example(ball_and_socket, $Ex)\n\
         ? joint_example($T, elbow)\n\
         ? joint_example(suture, $Ex)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The stock hinge is the elbow; the saddle joint is at the base of the thumb.
    assert!(out.contains("\"Ex\":\"elbow\""), "hinge → elbow: {out}");
    assert!(out.contains("\"Ex\":\"thumb\""), "saddle → thumb: {out}");
    // ball_and_socket is one-to-many: the source names both the hip and shoulder.
    assert!(out.contains("\"Ex\":\"hip\""), "ball_and_socket → hip: {out}");
    assert!(
        out.contains("\"Ex\":\"shoulder\""),
        "ball_and_socket → shoulder: {out}"
    );
    // The relation runs backward: the elbow recalls the hinge shape.
    assert!(
        out.contains("\"T\":\"hinge\""),
        "elbow → hinge (reverse recall): {out}"
    );
    // The answer carries the StatPearls (NIH/NLM) citation as its proof.
    //
    // THE WHOLE LOCATOR BOUND TO THE TIER, not two loose substrings. #15139
    // found eight files whose locators had rotted underneath exactly that
    // shape without a test noticing, because a bare host survives a site
    // reorganization untouched.
    assert!(
        out.contains("\"locator\":\"https://www.ncbi.nlm.nih.gov/books/NBK507893/\",\"trust\":\"authoritative\""),
        "carries the whole source citation: {out}"
    );
    // A skull suture is an immovable joint, not a synovial type — honest
    // abstention, never a fabricated example.
    assert!(
        out.contains("\"abstained\":true"),
        "unknown joint type abstains: {out}"
    );
}

const JOINTS_LOCATOR: &str = "https://www.ncbi.nlm.nih.gov/books/NBK507893/";

/// Assert one row carries its own span, in a program returning only that row.
///
/// The example is pinned in the same call, because a span assertion that never
/// reads the binding leaves the binding free — the gap review found on
/// `mesosphere -> meteors`, and the reason every row in this cascade now moves
/// span and value together.
///
/// `joint_type` takes an atom, so the query below is by TYPE; several types
/// have one row, and `ball_and_socket` has two, which is why the caller says
/// how many answers to expect.
fn assert_joint(tag: &str, jtype: &str, example: &str, span: &str, answers: usize) -> String {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("anatomy/joint-types.adj"),
        dir.join("joint-types.adj"),
    )
    .expect("copy shipped joint-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"joint-types.adj\"\n? joint_example({jtype}, $Ex)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        answers,
        "{jtype} has {answers} row(s): {out}"
    );
    assert!(
        out.contains(&format!("\"Ex\":\"{example}\"")),
        "{jtype} binds {example}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{JOINTS_LOCATOR}\",\"trust\":\"authoritative\""
        )),
        "{jtype} carries the span naming its own type: {out}"
    );
    out
}

/// #14986: the envelope was the HINGE row's own examples sentence, so recalling
/// the hip came back proved by *"Examples include the elbow, knee, ankle, and
/// interphalangeal joints."*
#[test]
fn every_joint_row_carries_a_span_naming_its_own_type() {
    assert_joint(
        "jtpivot", "pivot", "atlantoaxial",
        "The atlantoaxial joint, formed by the 1st (atlas) and 2nd (axis) cervical vertebrae, is a pivot joint.",
        1,
    );
    assert_joint(
        "jtcondyloid", "condyloid", "knuckles",
        "Examples of condyloid joints are the knuckles, formed by the distal metacarpals and proximal phalanges of the medial 4 fingers.",
        1,
    );
    assert_joint(
        "jthinge", "hinge", "elbow",
        "Flexion and extension are typically the only movements allowed by hinge joints. Examples include the elbow, knee, ankle, and interphalangeal joints.",
        1,
    );
    assert_joint(
        "jtplanar", "planar", "intercarpal",
        "Planar joints are multiaxial but restricted by the surrounding ligaments. Examples include the acromioclavicular, intercarpal, and intertarsal joints.",
        1,
    );
}

/// The page never puts "saddle" and "thumb" in one sentence. It defines the
/// saddle joint, names the trapezium/metacarpal joint as its example, and only
/// then says that joint moves the thumb — so the span is all four sentences.
/// Widening across that chain is what the README rule asks for; quoting only
/// the last sentence (as this file's own header used to) leaves "This joint"
/// pointing at nothing.
#[test]
fn the_saddle_row_carries_the_whole_chain_from_type_to_thumb() {
    let out = assert_joint(
        "jtsaddle", "saddle", "thumb",
        "A saddle joint is an articulation between 2 saddle-shaped bones, which are concave in one direction and convex in another. This joint type is biaxial. One example is the joint formed by the trapezium and 1st metacarpal bone. This joint allows the thumb to flex and extend parallel to the palm and abduct and adduct perpendicular to the palm, making the digit opposable.",
        1,
    );
    // Both ends of the chain reach the answer: the type at one end, the thumb
    // at the other. Twice each — provenance is emitted under `citations` and
    // again under `steps`.
    assert_eq!(
        out.matches("A saddle joint is an articulation").count(),
        2,
        "the span opens on the sentence naming the type: {out}"
    );
    assert_eq!(
        out.matches("allows the thumb to flex").count(),
        2,
        "and closes on the one naming the thumb: {out}"
    );
}

/// Two rows, one sentence. Each carries its own copy, so either can be
/// re-checked or changed alone — and each is pinned SEPARATELY here.
///
/// A first version queried by type, which returns BOTH rows, so
/// `out.contains(span)` was satisfied by whichever copy was still intact.
/// Mutation caught it: breaking only the shoulder row survived, and so did
/// removing the page's hyphenation from one copy. Binding the EXAMPLE instead
/// returns exactly one answer, which is what makes the needle belong to the
/// row under test.
fn assert_joint_by_example(tag: &str, example: &str, jtype: &str, span: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("anatomy/joint-types.adj"),
        dir.join("joint-types.adj"),
    )
    .expect("copy shipped joint-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"joint-types.adj\"\n? joint_example($T, {example})\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row has {example}, so the needles below are its own: {out}"
    );
    assert!(
        out.contains(&format!("\"T\":\"{jtype}\"")),
        "{example} binds {jtype}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{JOINTS_LOCATOR}\",\"trust\":\"authoritative\""
        )),
        "{example}'s own row carries the span: {out}"
    );
}

#[test]
fn each_ball_and_socket_row_carries_its_own_copy_of_the_shared_sentence() {
    // The page hyphenates what the atom spells with underscores; pinned in
    // both rows so nobody "tidies" either quote into matching the atom.
    let span = "The body's only ball-and-socket joints are the hip and shoulder (glenohumeral) joints.";
    assert_joint_by_example("jtbship", "hip", "ball_and_socket", span);
    assert_joint_by_example("jtbsshoulder", "shoulder", "ball_and_socket", span);
}

/// The envelope is the framing sentence — the page's own list of the six
/// movement classes. It defends the table, warrants no row, and a leak back
/// into an answer is what would redden. Its wording cannot be pinned from
/// output once every row overrides `source`; that is disclosed, not implied.
#[test]
fn the_framing_envelope_never_reaches_an_answer() {
    let dir = scratch("jtenvelope");
    std::fs::copy(
        facts_stdlib().join("anatomy/joint-types.adj"),
        dir.join("joint-types.adj"),
    )
    .expect("copy shipped joint-types.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"joint-types.adj\"\n? joint_example($T, $Ex)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        7,
        "all seven rows answer: {out}"
    );
    assert!(
        !out.contains("synovial joints are often classified by the movement types"),
        "the framing span warrants no row: {out}"
    );
}

/// Parse `(row key, its own `source`)` out of a shipped `.adj`.
///
/// The full two-column key: column 1 alone is not a row identity. A first pass
/// at the #15193 census keyed on column 1 and printed brain-parts as
/// `brainstem+brainstem+…`, because ten rows there share that part and differ
/// only in the function.
fn row_spans(rel: &str) -> Vec<(String, String)> {
    let adj = std::fs::read_to_string(facts_stdlib().join(rel))
        .unwrap_or_else(|e| panic!("read shipped {rel}: {e}"));
    let mut out: Vec<(String, String)> = Vec::new();
    let mut key: Option<String> = None;
    for line in adj.lines() {
        if let Some(rest) = line.strip_prefix("    row (") {
            key = rest.split(')').next().map(|k| {
                k.split(',').map(|p| p.trim()).collect::<Vec<_>>().join(", ")
            });
        } else if let Some(rest) = line.strip_prefix(r#"        source ""#) {
            let span = rest.trim_end_matches(0x22 as char).to_string();
            out.push((key.clone().expect("a row precedes every source"), span));
        }
    }
    out
}

/// The groups of rows that share one span, in first-appearance order.
fn sharing_groups(pairs: &[(String, String)]) -> Vec<Vec<String>> {
    let mut order: Vec<String> = Vec::new();
    for (_, span) in pairs {
        if !order.contains(span) {
            order.push(span.clone());
        }
    }
    order
        .iter()
        .map(|span| {
            pairs
                .iter()
                .filter(|(_, s)| s == span)
                .map(|(k, _)| k.clone())
                .collect::<Vec<_>>()
        })
        .filter(|g| g.len() > 1)
        .collect()
}

/// #15193. The sharing structure of this table, asserted against the SHIPPED
/// FILE rather than described in a comment.
///
/// One sentence names both ball-and-socket joints.
#[test]
fn the_shared_spans_are_exactly_the_declared_ones() {
    let pairs = row_spans("anatomy/joint-types.adj");
    assert_eq!(pairs.len(), 7, "every row carries its own source: {pairs:?}");
    let groups = sharing_groups(&pairs);
    let expected: Vec<Vec<&str>> = vec![
        vec!["ball_and_socket, hip", "ball_and_socket, shoulder"],
    ];
    assert_eq!(
        groups, expected,
        "the shared sentences are shared by exactly these rows: {groups:?}"
    );
    let shared: usize = groups.iter().map(|g| g.len()).sum();
    assert_eq!(
        pairs.len() - shared,
        5,
        "and 5 rows have a sentence to themselves: {groups:?}"
    );
}
