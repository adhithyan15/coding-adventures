//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/muscle-groups.adj`) driven through the built CLI:
//! a native `table` of skeletal muscle → body region resolves a binding-query
//! recall with the source's Wikipedia (consensus) citation, runs the relation
//! backward (region → every muscle in it, one-to-many), and abstains on a
//! non-muscle (the femur, a bone) — 0 model calls.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsmuscle_{tag}_{}", std::process::id()));
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
fn anatomy_muscle_groups_recall_binds_region_with_citation() {
    let dir = scratch("musclegroups");
    // Copy the shipped anatomy table beside the entry program and import it.
    let src = facts_stdlib().join("anatomy/muscle-groups.adj");
    std::fs::copy(&src, dir.join("muscle-groups.adj")).expect("copy shipped muscle-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-groups.adj\"\n\
         ? muscle_region(biceps_brachii, $R)\n\
         ? muscle_region(deltoid, $R)\n\
         ? muscle_region(quadriceps, $R)\n\
         ? muscle_region($M, arm)\n\
         ? muscle_region(femur, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Each named muscle binds its body region — a single lowercase token echoed
    // verbatim from the source sentence.
    assert!(out.contains("\"R\":\"arm\""), "biceps_brachii → arm: {out}");
    assert!(out.contains("\"R\":\"shoulder\""), "deltoid → shoulder: {out}");
    assert!(out.contains("\"R\":\"thigh\""), "quadriceps → thigh: {out}");
    // The relation runs backward and is one-to-many: the region `arm` recalls
    // BOTH the biceps and the triceps.
    assert!(
        out.contains("\"M\":\"biceps_brachii\"") && out.contains("\"M\":\"triceps_brachii\""),
        "arm → biceps_brachii AND triceps_brachii (reverse recall): {out}"
    );
    // The answer carries the Wikipedia citation as its proof, at consensus trust.
    assert!(
        out.contains("en.wikipedia.org") && out.contains("\"trust\":\"consensus\""),
        "carries the source citation: {out}"
    );
    // The femur is a bone, not a muscle — honest abstention, never a fabricated
    // location.
    assert!(out.contains("\"abstained\":true"), "unknown muscle abstains: {out}");
}

const MG_DELT_PIN: &str = r#""bindings":{"R":"shoulder"},"citations":[{"source":"The biceps or biceps brachii (Latin: musculus biceps brachii, \"two-headed muscle of the arm\") is a large muscle that lies on the front of the upper arm between the shoulder and the elbow.","locator":"https://en.wikipedia.org/wiki/Biceps","trust":"consensus","corroborations":[{"source":"The triceps, or triceps brachii (Latin for \"three-headed muscle of the arm\"), is a large muscle on the back of the upper limb of many vertebrates.","locator":"https://en.wikipedia.org/wiki/Triceps"},{"source":"The deltoid muscle (or musculus deltoideus) is the muscle[1] forming the rounded contour of the human shoulder.","locator":"https://en.wikipedia.org/wiki/Deltoid_muscle""#;

const MG_QUAD_PIN: &str = r#""bindings":{"R":"thigh"},"citations":[{"source":"The biceps or biceps brachii (Latin: musculus biceps brachii, \"two-headed muscle of the arm\") is a large muscle that lies on the front of the upper arm between the shoulder and the elbow.","locator":"https://en.wikipedia.org/wiki/Biceps","trust":"consensus","corroborations":[{"source":"The triceps, or triceps brachii (Latin for \"three-headed muscle of the arm\"), is a large muscle on the back of the upper limb of many vertebrates.","locator":"https://en.wikipedia.org/wiki/Triceps"},{"source":"The deltoid muscle (or musculus deltoideus) is the muscle[1] forming the rounded contour of the human shoulder.","locator":"https://en.wikipedia.org/wiki/Deltoid_muscle"},{"source":"The pectoralis major (from Latin pectus 'breast') is a thick, fan-shaped or triangular convergent muscle of the human chest.","locator":"https://en.wikipedia.org/wiki/Pectoralis_major"},{"source":"The rectus abdominis, (Latin: straight abdominal) also known as the \"abdominal muscle\" or simply better known as the \"abs\", and sometimes informally referred to as the \"six-pack\", is a pair of segmented skeletal muscle on the ventral aspect of a person's abdomen.","locator":"https://en.wikipedia.org/wiki/Rectus_abdominis_muscle"},{"source":"The gluteus maximus is the main extensor muscle of the hip in humans.","locator":"https://en.wikipedia.org/wiki/Gluteus_maximus_muscle"},{"source":"The quadriceps femoris muscle (/ˈkwɒdrɪsɛps ˈfɛmərɪs/, also called the quadriceps extensor, quadriceps or quads) is a large muscle group that includes the four prevailing muscles on the front of the thigh.","locator":"https://en.wikipedia.org/wiki/Quadriceps""#;

const MG_ALL_PIN: &str = r#""bindings":{"M":"biceps_brachii"},"citations":[{"source":"The biceps or biceps brachii (Latin: musculus biceps brachii, \"two-headed muscle of the arm\") is a large muscle that lies on the front of the upper arm between the shoulder and the elbow.","locator":"https://en.wikipedia.org/wiki/Biceps","trust":"consensus","corroborations":[{"source":"The triceps, or triceps brachii (Latin for \"three-headed muscle of the arm\"), is a large muscle on the back of the upper limb of many vertebrates.","locator":"https://en.wikipedia.org/wiki/Triceps"},{"source":"The deltoid muscle (or musculus deltoideus) is the muscle[1] forming the rounded contour of the human shoulder.","locator":"https://en.wikipedia.org/wiki/Deltoid_muscle"},{"source":"The pectoralis major (from Latin pectus 'breast') is a thick, fan-shaped or triangular convergent muscle of the human chest.","locator":"https://en.wikipedia.org/wiki/Pectoralis_major"},{"source":"The rectus abdominis, (Latin: straight abdominal) also known as the \"abdominal muscle\" or simply better known as the \"abs\", and sometimes informally referred to as the \"six-pack\", is a pair of segmented skeletal muscle on the ventral aspect of a person's abdomen.","locator":"https://en.wikipedia.org/wiki/Rectus_abdominis_muscle"},{"source":"The gluteus maximus is the main extensor muscle of the hip in humans.","locator":"https://en.wikipedia.org/wiki/Gluteus_maximus_muscle"},{"source":"The quadriceps femoris muscle (/ˈkwɒdrɪsɛps ˈfɛmərɪs/, also called the quadriceps extensor, quadriceps or quads) is a large muscle group that includes the four prevailing muscles on the front of the thigh.","locator":"https://en.wikipedia.org/wiki/Quadriceps"},{"source":"The sartorius muscle (/sɑːrˈtɔːriəs/), historically known as couturier (French for \"tailor\"), is the longest muscle in the human body.[2] It is a long, thin, superficial muscle that runs down the length of the thigh in the anterior compartment.","locator":"https://en.wikipedia.org/wiki/Sartorius_muscle"},{"source":"The gastrocnemius muscle (plural gastrocnemii) is a superficial two-headed muscle. It is located superficial to the soleus in the posterior (back) compartment of the leg.","locator":"https://en.wikipedia.org/wiki/Gastrocnemius_muscle""#;

#[test]
fn muscle_groups_deltoid_answer_keeps_its_footnote_marker() {
    let dir = scratch("cite_delt");
    std::fs::copy(
        facts_stdlib().join("anatomy/muscle-groups.adj"),
        dir.join("muscle-groups.adj"),
    )
    .expect("copy shipped muscle-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-groups.adj\"\n? muscle_region(deltoid, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The library header quoted this as "is the muscle forming the rounded
    // contour", dropping the page's "[1]" footnote marker -- real rendered
    // text. The pin runs to DELTOID'S OWN corroboration (index 1), not
    // index 0, so it fails if deltoid's cite specifically is damaged.
    assert!(
        out.contains(MG_DELT_PIN),
        "deltoid's answer keeps the page's footnote marker: {out}"
    );
}

#[test]
fn muscle_groups_quadriceps_answer_keeps_the_parenthetical_its_header_had_deleted() {
    let dir = scratch("cite_quad");
    std::fs::copy(
        facts_stdlib().join("anatomy/muscle-groups.adj"),
        dir.join("muscle-groups.adj"),
    )
    .expect("copy shipped muscle-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-groups.adj\"\n? muscle_region(quadriceps, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // THE WORST HEADER-QUOTE SUBTYPE FOUND. The shipped header read "The
    // quadriceps femoris muscle is a large muscle group..." -- the page's
    // IPA-and-alias parenthetical had been deleted WITH NO ELLIPSIS AT ALL,
    // so the quote read as faithful and was not.
    //
    // That is the same shape as the defect that propagated unnoticed from
    // quadrilateral-types' header into another library's shipped `source`:
    // marked elisions announce themselves, unmarked ones are found only by
    // fetching the page. This pin is the standing check for it.
    //
    // Runs to QUADRICEPS' OWN corroboration (index 5).
    assert!(
        out.contains(MG_QUAD_PIN),
        "quadriceps' answer keeps the parenthetical its header had deleted: {out}"
    );
}

#[test]
fn muscle_groups_reverse_answer_carries_all_eight_corroborations_in_order() {
    let dir = scratch("cite_all");
    std::fs::copy(
        facts_stdlib().join("anatomy/muscle-groups.adj"),
        dir.join("muscle-groups.adj"),
    )
    .expect("copy shipped muscle-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-groups.adj\"\n? muscle_region($M, arm)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // Spans the WHOLE eight-entry corroboration list. A pure reorder, or a
    // dropped middle entry, fails here while every sentence is still present
    // somewhere in the blob -- invisible to any per-sentence check.
    assert!(
        out.contains(MG_ALL_PIN),
        "the reverse answer carries all eight corroborations in order: {out}"
    );
}


const MUSCLE_BICEPS_PIN: &str = r#""bindings":{"R":"arm"},"citations":[{"source":"The biceps or biceps brachii (Latin: musculus biceps brachii, \"two-headed muscle of the arm\") is a large muscle that lies on the front of the upper arm between the shoulder and the elbow.","locator":"https://en.wikipedia.org/wiki/Biceps","trust":"consensus""#;

#[test]
fn muscle_groups_biceps_citation_keeps_the_pages_latin_gloss() {
    let dir = scratch("biceps_gloss_4d");
    std::fs::copy(
        facts_stdlib().join("anatomy/muscle-groups.adj"),
        dir.join("muscle-groups.adj"),
    )
    .expect("copy shipped muscle-groups.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-groups.adj\"\n? muscle_region(biceps_brachii, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    // The value dropped the page's Latin gloss without a marker:
    //
    //   The biceps or biceps brachii (Latin: musculus biceps brachii,
    //   "two-headed muscle of the arm") is a large muscle ...
    //
    // WHAT MAKES THIS A SLIP RATHER THAN A POLICY is this same table: seven
    // of its eight `cites` values keep their parentheticals -- the gluteus
    // maximus sentence has none to keep -- including the triceps one three
    // lines below, which is the identical construction on the identical kind
    // of page. One of eight was tidied.
    assert!(
        out.contains(MUSCLE_BICEPS_PIN),
        "the biceps citation matches its page: {out}"
    );
}
