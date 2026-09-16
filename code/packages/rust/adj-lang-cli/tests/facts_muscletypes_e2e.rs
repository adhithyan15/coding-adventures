//! End-to-end test for the biology FACTS library
//! (`adj-facts-stdlib/biology/muscle-types.adj`) driven through the built CLI:
//! a native `table` of the three muscle-tissue types → the ONE distinctive
//! characteristic the source assigns each resolves binding-query recalls
//! (forward AND backward) with the source's NCI SEER Training Modules citation,
//! and abstains on a word that is not one of the three muscle-tissue types
//! (bone) — 0 model calls.
//!
//! EACH ROW CARRIES ITS OWN SENTENCE (RS-5e, #14986). The envelope used to be
//! the skeletal sentence, the primary source of all three answers; it is now
//! the page's framing sentence, which every row overrides. The smooth row's
//! trait, "involuntary", is only in the second of its two sentences ("They are
//! called involuntary muscles."), so its source is the two-sentence span.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsm_{tag}_{}", std::process::id()));
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

const LOCATOR: &str = "https://training.seer.cancer.gov/anatomy/cells_tissues_membranes/tissues/muscle.html";
const ENVELOPE: &str = "Muscle tissue is composed of cells that have the special ability to shorten or contract in order to produce movement of the body parts.";
const SKELETAL: &str = "Skeletal muscle fibers are cylindrical, multinucleated, striated, and under voluntary control.";
const SMOOTH: &str = "Smooth muscle cells are spindle shaped, have a single, centrally located nucleus, and lack striations. They are called involuntary muscles.";
const CARDIAC: &str = "Cardiac muscle has branching fibers, one nucleus per cell, striations, and intercalated disks.";

/// (muscle, trait, that row's own span)
const ROWS: [(&str, &str, &str); 3] = [
    ("skeletal", "voluntary", SKELETAL),
    ("smooth", "involuntary", SMOOTH),
    ("cardiac", "intercalated_disks", CARDIAC),
];

/// The whole citation a row's answer carries, as one contiguous run.
fn citation(span: &str) -> String {
    format!("\"source\":\"{span}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]")
}

fn place(dir: &Path) {
    let src = facts_stdlib().join("biology/muscle-types.adj");
    std::fs::copy(&src, dir.join("muscle-types.adj")).expect("copy shipped muscle-types.adj");
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("biology/muscle-types.adj")).expect("read shipped muscle-types.adj");
    adj[adj.find("table muscle_trait").expect("table")..].to_string()
}

#[test]
fn biology_muscle_types_recall_binds_trait_with_citation() {
    let dir = scratch("muscletypes");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"muscle-types.adj\"\n\
         ? muscle_trait(skeletal, $Trait)\n\
         ? muscle_trait(smooth, $Trait)\n\
         ? muscle_trait(cardiac, $Trait)\n\
         ? muscle_trait($Muscle, involuntary)\n\
         ? muscle_trait(bone, $Trait)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // Skeletal muscle is voluntary, smooth muscle is involuntary, and cardiac
    // muscle is the one distinguished by intercalated disks — the recalled
    // traits (forward binds).
    assert!(
        out.contains("\"Trait\":\"voluntary\""),
        "skeletal → voluntary: {out}"
    );
    assert!(
        out.contains("\"Trait\":\"involuntary\""),
        "smooth → involuntary: {out}"
    );
    assert!(
        out.contains("\"Trait\":\"intercalated_disks\""),
        "cardiac → intercalated_disks: {out}"
    );
    // The relation runs BACKWARD: bind the trait `involuntary`, recall its
    // muscle type.
    assert!(
        out.contains("\"Muscle\":\"smooth\""),
        "involuntary → smooth (reverse recall): {out}"
    );
    // ONE CONTIGUOUS RUN for the skeletal answer, not
    // `contains("training.seer.cancer.gov") && contains(trust)` (#15209).
    assert!(out.contains(&citation(SKELETAL)), "carries the skeletal sentence, whole: {out}");
    // Bone is not one of the three muscle-tissue types — honest abstention,
    // never a fabricated trait.
    assert!(out.contains("\"abstained\":true"), "bone abstains: {out}");
}

#[test]
fn every_muscle_answer_carries_its_own_span() {
    for (muscle, trait_atom, span) in ROWS {
        let dir = scratch(&format!("row_{muscle}"));
        place(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"muscle-types.adj\"\n? muscle_trait({muscle}, $T)\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {muscle}: {out}");
        assert!(out.contains(&format!("\"T\":\"{trait_atom}\"")), "{muscle} → {trait_atom}: {out}");
        assert!(out.contains(&citation(span)), "{muscle}: its own span, whole: {out}");
        for (other, _, other_span) in ROWS {
            if other != muscle {
                assert!(!out.contains(other_span), "the {other} span must not reach {muscle}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the envelope is not primary for {muscle}: {out}");
    }
}

#[test]
fn the_smooth_row_carries_both_sentences_because_the_trait_is_in_the_second() {
    // "involuntary" is only in "They are called involuntary muscles.", whose
    // "They" needs the sentence before it -- so the row's source is the whole
    // two-sentence span, not either sentence alone.
    let body = shipped_table();
    assert!(
        body.contains(&format!("    row (smooth, involuntary) {{\n        source \"{SMOOTH}\"\n    }}")),
        "the smooth row carries the two-sentence span"
    );
    assert!(SMOOTH.contains("They are called involuntary muscles."), "the span includes the sentence naming the trait");
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    // Every row overrides the envelope, so the envelope's wording reaches no
    // answer; this file-shape test is what pins it.
    let body = shipped_table();
    for (muscle, trait_atom, span) in ROWS {
        let expected = format!("    row ({muscle}, {trait_atom}) {{\n        source \"{span}\"\n    }}");
        assert!(body.contains(&expected), "row ({muscle}, {trait_atom}) is shipped in its measured shape");
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
    for word in ["skeletal", "smooth", "cardiac", "voluntary", "intercalated"] {
        assert!(!folded.contains(word), "the envelope must name no muscle type or trait, but contains {word:?}");
    }
}
