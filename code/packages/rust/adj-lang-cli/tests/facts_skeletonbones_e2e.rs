//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/skeleton-bones.adj`) driven through the built CLI:
//! a native `table` of common human bone → body region resolves binding-query
//! recalls (forward AND backward) with the source's NIH / MedlinePlus citation,
//! and abstains on a word that is not one of these bones (the spleen) — 0 model
//! calls.

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

#[test]
fn biology_skeleton_bones_recall_binds_region_with_citation() {
    let dir = scratch("skeletonbones");
    // Copy the shipped biology table beside the entry program and import it.
    let src = facts_stdlib().join("biology/skeleton-bones.adj");
    std::fs::copy(&src, dir.join("skeleton-bones.adj")).expect("copy shipped skeleton-bones.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"skeleton-bones.adj\"\n\
         ? bone_region(femur, $Region)\n\
         ? bone_region(patella, $Region)\n\
         ? bone_region(sternum, $Region)\n\
         ? bone_region($Bone, arm)\n\
         ? bone_region(spleen, $Region)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The femur is a leg bone, the patella is at the knee, the sternum is in the
    // chest — the recalled regions (forward binds).
    assert!(out.contains("\"Region\":\"leg\""), "femur → leg: {out}");
    assert!(out.contains("\"Region\":\"knee\""), "patella → knee: {out}");
    assert!(out.contains("\"Region\":\"chest\""), "sternum → chest: {out}");
    // The relation runs BACKWARD: bind the region `arm`, recall the arm bones.
    assert!(
        out.contains("\"Bone\":\"humerus\"")
            && out.contains("\"Bone\":\"radius\"")
            && out.contains("\"Bone\":\"ulna\""),
        "arm → humerus ; radius ; ulna (reverse recall): {out}"
    );
    // The answer carries the NIH / NLM MedlinePlus citation as its proof, at the
    // `authoritative` trust tier for a primary U.S. government source.
    assert!(
        out.contains("medlineplus.gov") && out.contains("\"trust\":\"authoritative\""),
        "carries the source citation: {out}"
    );
    // The spleen is an organ, not a bone — honest abstention, never a fabricated
    // region.
    assert!(out.contains("\"abstained\":true"), "spleen abstains: {out}");
}

/// Assert one row carries its own span AND its own locator, in a program that
/// returns only that row.
///
/// Bound by BONE, not by region: `arm` has three rows and `leg` three, and a
/// multi-answer query lets a needle be satisfied by a sibling row's intact
/// copy. That is not hypothetical — it is the defect mutation found in
/// `anatomy/joint-types.adj`, where `hip` and `shoulder` share one sentence
/// and breaking only one of them stayed green.
fn assert_bone(tag: &str, bone: &str, region: &str, span: &str, locator: &str) {
    let dir = scratch(tag);
    std::fs::copy(
        facts_stdlib().join("biology/skeleton-bones.adj"),
        dir.join("skeleton-bones.adj"),
    )
    .expect("copy shipped skeleton-bones.adj");
    std::fs::write(
        dir.join("case.adj"),
        format!("import \"skeleton-bones.adj\"\n? bone_region({bone}, $R)\n"),
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        1,
        "exactly one row for {bone}, so every needle below is its own: {out}"
    );
    assert!(
        out.contains(&format!("\"R\":\"{region}\"")),
        "{bone} binds {region}: {out}"
    );
    assert!(
        out.contains(&format!(
            "\"source\":\"{span}\",\"locator\":\"{locator}\",\"trust\":\"authoritative\""
        )),
        "{bone} carries its own span and locator: {out}"
    );
}

const LEG: &str = "https://medlineplus.gov/ency/imagepages/8844.htm";
const ARM: &str = "https://medlineplus.gov/arminjuriesanddisorders.html";
const SHOULDER: &str = "https://medlineplus.gov/shoulderinjuriesanddisorders.html";
const CHEST: &str = "https://medlineplus.gov/ency/imagepages/8787.htm";
const SKULL: &str = "https://www.ncbi.nlm.nih.gov/books/NBK499834/";

/// #14986, and the sharpest case in this cascade so far: the envelope was the
/// FEMUR row's sentence on a MedlinePlus **leg** image page. Measured against
/// that page, **12 of these 15 bones are never mentioned at all** — so a recall
/// of the occipital bone came back proved by a sentence about the thigh.
#[test]
fn every_bone_row_carries_its_own_span_and_locator() {
    assert_bone(
        "sbfemur", "femur", "leg",
        "The thigh bone, or femur, is the large upper leg bone that connects the lower leg bones (knee joint) to the pelvic bone (hip joint).",
        LEG,
    );
    assert_bone(
        "sbtibia", "tibia", "leg",
        "The lower leg is comprised of two bones, the tibia and the smaller fibula.",
        LEG,
    );
    assert_bone(
        "sbhumerus", "humerus", "arm",
        "Of the 206 bones in your body, three of them are in your arm: the humerus, radius, and ulna.",
        ARM,
    );
    assert_bone(
        "sbscapula", "scapula", "shoulder",
        "Your shoulder joint is composed of three bones: the clavicle (collarbone), the scapula (shoulder blade), and the humerus (upper arm bone).",
        SHOULDER,
    );
    assert_bone(
        "sbribs", "ribs", "chest",
        "The ribs connect on the front of the chest with the long flat sternum, or breast bone, and on the back with the vertebral column, creating a cage of protection for the lungs and heart.",
        CHEST,
    );
    // EVERY row, not one per shared sentence. A first version asserted only
    // femur/tibia/humerus/scapula/ribs/patella — one bone per span group — and
    // review showed nine mutants surviving: `clavicle`'s source could be
    // replaced with "Made up entirely." and the suite stayed green. Worse, the
    // mutation set I had written tested the COVERED direction of each shared
    // sentence (tibia, not fibula; scapula, not clavicle; ribs, not sternum),
    // so "7 of 7 killed" was true and drawn around the gap.
    assert_bone(
        "sbfibula", "fibula", "leg",
        "The lower leg is comprised of two bones, the tibia and the smaller fibula.",
        LEG,
    );
    assert_bone(
        "sbradius", "radius", "arm",
        "Of the 206 bones in your body, three of them are in your arm: the humerus, radius, and ulna.",
        ARM,
    );
    assert_bone(
        "sbulna", "ulna", "arm",
        "Of the 206 bones in your body, three of them are in your arm: the humerus, radius, and ulna.",
        ARM,
    );
    assert_bone(
        "sbclavicle", "clavicle", "shoulder",
        "Your shoulder joint is composed of three bones: the clavicle (collarbone), the scapula (shoulder blade), and the humerus (upper arm bone).",
        SHOULDER,
    );
    assert_bone(
        "sbsternum", "sternum", "chest",
        "The ribs connect on the front of the chest with the long flat sternum, or breast bone, and on the back with the vertebral column, creating a cage of protection for the lungs and heart.",
        CHEST,
    );
    assert_bone(
        "sbpatella", "patella", "knee",
        "Your kneecap is called the patella.",
        "https://medlineplus.gov/ency/article/002974.htm",
    );
}

/// The four skull rows are RE-GROUNDED, not moved. The header's old skull
/// quote — *"…6 separate cranial (skull) bones: Frontal bone, Occipital bone,
/// Two parietal bones, Two temporal bones."* — occurs **zero** times on the
/// MedlinePlus page it named: there the bones are a bulleted list and the
/// commas joining them to the lead-in were invented, the same defect class as
/// `language/contraction.adj`'s invented `=` (#15151). StatPearls states them
/// in prose.
#[test]
fn the_skull_rows_cite_prose_and_not_a_welded_list() {
    for (tag, bone) in [
        ("sbfrontal", "frontal"),
        ("sbparietal", "parietal"),
        ("sbtemporal", "temporal"),
        ("sboccipital", "occipital"),
    ] {
        assert_bone(
            tag, bone, "skull",
            "The calvaria, the uppermost part of the skull, protects the cerebral cortex, cerebellum, and orbital contents. It is composed of the frontal bone, parietal bones, temporal bones, and occipital bone.",
            SKULL,
        );
    }
}

/// Every row overrides `locator`, so nothing inherits the envelope's — which
/// now points at the SEER divisions page, whose framing SPAN names no bone.
/// (The page itself does mention ribs and the sternum elsewhere; the claim is
/// about the span, not the page.) A row that lost its own locator would
/// silently cite it, and that is the regression this conversion could most
/// easily introduce.
#[test]
fn no_answer_cites_the_framing_page_or_leaks_the_framing_span() {
    let dir = scratch("sbenvelope");
    std::fs::copy(
        facts_stdlib().join("biology/skeleton-bones.adj"),
        dir.join("skeleton-bones.adj"),
    )
    .expect("copy shipped skeleton-bones.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"skeleton-bones.adj\"\n? bone_region($B, $R)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert_eq!(
        out.matches("\"citations\":[").count(),
        15,
        "all fifteen rows answer: {out}"
    );
    assert!(
        !out.contains("The adult human skeleton usually consists of"),
        "the framing span warrants no row: {out}"
    );
    assert!(
        !out.contains("training.seer.cancer.gov"),
        "no row inherits the framing locator: {out}"
    );
    // And the defect itself: the leg image page must warrant ONLY the three
    // leg rows. Six occurrences — three rows, each emitting its provenance
    // under `citations` and again under `steps`.
    //
    // A first version asserted `!out.contains("\"R\":\"skull\",\"B\"")` with a
    // comment claiming exactly this. Bindings serialize B before R, so that
    // needle could never appear whatever the data said: an assertion that
    // cannot fail, under a comment promising it guards the central defect.
    assert_eq!(
        out.matches("imagepages/8844.htm").count(),
        6,
        "the leg page warrants the three leg rows and nothing else: {out}"
    );
}
