//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/muscle-striation.adj`) driven through the
//! built CLI: a native `table` recording whether each of the three
//! muscle-tissue types is striated -- a sibling to the already-shipped
//! `muscle-types.adj` (which only carries one distinctive characteristic
//! per muscle type), decoding the striated/lacks-striations clause already
//! sitting unused inside that table's own per-muscle header quotes.
//! Resolves forward and backward recall queries with the source's
//! citation -- 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the skeletal sentence, the primary source of all three answers; it is now
//! the page's framing sentence, which every row overrides. The smooth row
//! carries one sentence, not the two the table used to cite. Each row's
//! citation is pinned whole.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_musclestriation_{tag}_{}", std::process::id()));
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
    let src = facts_stdlib().join("biology/muscle-striation.adj");
    std::fs::copy(&src, dir.join("muscle-striation.adj"))
        .expect("copy shipped muscle-striation.adj");
}

const LOCATOR: &str = "https://training.seer.cancer.gov/anatomy/cells_tissues_membranes/tissues/muscle.html";
const ENVELOPE: &str = "Muscle tissue is composed of cells that have the special ability to shorten or contract in order to produce movement of the body parts.";
const SKELETAL: &str = "Skeletal muscle fibers are cylindrical, multinucleated, striated, and under voluntary control.";
const SMOOTH: &str = "Smooth muscle cells are spindle shaped, have a single, centrally located nucleus, and lack striations.";
const CARDIAC: &str = "Cardiac muscle has branching fibers, one nucleus per cell, striations, and intercalated disks.";
/// The second sentence the table used to cite for smooth muscle. It names no
/// striation, so it is not part of the smooth row's warrant.
const SMOOTH_SECOND_SENTENCE: &str = "They are called involuntary muscles.";

/// (muscle, striated, that row's own sentence)
const ROWS: [(&str, &str, &str); 3] = [
    ("skeletal", "yes", SKELETAL),
    ("smooth", "no", SMOOTH),
    ("cardiac", "yes", CARDIAC),
];

/// The whole citation a row's answer carries, as one contiguous run.
fn citation(sentence: &str) -> String {
    format!("\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]")
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/muscle-striation.adj"))
        .expect("read shipped muscle-striation.adj");
    adj[adj.find("table muscle_striated").expect("table")..].to_string()
}

#[test]
fn muscle_striation_recalls_cardiac_as_striated_with_citation() {
    let dir = scratch("cardiac");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-striation.adj\"\n\
         ? muscle_striated(cardiac, $Striated)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(
        out.contains("\"term\":\"muscle_striated(cardiac, yes)\""),
        "cardiac muscle should recall as striated: {out}"
    );
    // ONE CONTIGUOUS RUN, not `contains("seer.cancer.gov") && contains(trust)` (#15209).
    assert!(out.contains(&citation(CARDIAC)), "carries the cardiac sentence, whole: {out}");
}

#[test]
fn muscle_striation_backward_recalls_smooth_as_the_only_non_striated_type() {
    let dir = scratch("nonstriated");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-striation.adj\"\n\
         ? muscle_striated($Muscle, no)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        out.contains("\"term\":\"muscle_striated(smooth, no)\""),
        "smooth muscle should be the only recalled non-striated type: {out}"
    );
    assert!(
        !out.contains("muscle_striated(skeletal, no)"),
        "skeletal muscle is striated, not the negative recall: {out}"
    );
    assert!(
        !out.contains("muscle_striated(cardiac, no)"),
        "cardiac muscle is striated, not the negative recall: {out}"
    );
}

#[test]
fn muscle_striation_covers_all_three_types_without_abstention() {
    let dir = scratch("noabstain");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-striation.adj\"\n\
         ? muscle_striated(skeletal, $S1)\n\
         ? muscle_striated(smooth, $S2)\n\
         ? muscle_striated(cardiac, $S3)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(
        !out.contains("\"abstained\":true"),
        "all three muscle-tissue types have a striation fact on record -- no abstention expected: {out}"
    );
}

#[test]
fn every_type_answer_carries_its_own_sentence() {
    for (muscle, striated, sentence) in ROWS {
        let dir = scratch(&format!("row_{muscle}"));
        place_lib(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"muscle-striation.adj\"\n? muscle_striated({muscle}, $S)\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {muscle}: {out}");
        assert!(out.contains(&format!("\"S\":\"{striated}\"")), "{muscle} striated = {striated}: {out}");
        assert!(out.contains(&citation(sentence)), "{muscle}: its own sentence, whole: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != muscle {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {muscle}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {muscle}: {out}");
    }
}

#[test]
fn the_smooth_row_carries_one_sentence_not_two() {
    // The table used to cite "... lack striations. They are called involuntary
    // muscles." The second sentence names no striation, so it is not part of
    // the smooth row's warrant, and it appears nowhere in the table or answer.
    let body = shipped_table();
    assert!(!body.contains(SMOOTH_SECOND_SENTENCE), "the second smooth sentence is not in the table");
    let dir = scratch("smooth_one_sentence");
    place_lib(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-striation.adj\"\n? muscle_striated(smooth, $S)\n",
    )
    .unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains(&citation(SMOOTH)), "the smooth citation is the one-sentence form, whole: {out}");
    assert!(!out.contains(SMOOTH_SECOND_SENTENCE), "the second sentence reaches no answer: {out}");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (muscle, striated, sentence) in ROWS {
        let expected = format!("    row ({muscle}, {striated}) {{\n        source \"{sentence}\"\n    }}");
        assert!(body.contains(&expected), "row ({muscle}, {striated}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 3, "three row sources");
    assert!(!body.contains("cites "), "no corroboration anywhere in the table");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{SKELETAL}\"\n    locator")), "not the skeletal sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["skeletal", "smooth", "cardiac", "striat", "voluntary"] {
        assert!(!folded.contains(word), "the envelope must name no muscle type or striation, but contains {word:?}");
    }
}
