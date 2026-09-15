//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/ear-parts.adj`) driven through the built CLI:
//! a native `table` of ear structure → ear region resolves binding-query
//! recalls with the source's NIDCD "How Do We Hear?" citation, runs the
//! relation backward (region → structure, recalling the three middle-ear
//! ossicles), and abstains on a non-listed structure (the pinna) — 0 model
//! calls.
//!
//! EACH ROW CARRIES THE PROVENANCE ITS SENTENCES SUPPORT (RS-5e, #14986). The
//! envelope used to be the cochlea's own sentence, the primary source of all
//! five answers. The cochlea row now takes that sentence as its `source`. The
//! ear canal and the three ossicles `cites` their sentences, because none of
//! those sentences states the row alone, and the envelope -- now the page's
//! framing sentence -- stays their primary source. Every citation is pinned
//! whole.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factsear_{tag}_{}", std::process::id()));
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

const LOCATOR: &str = "https://www.nidcd.nih.gov/health/how-do-we-hear";
const ENVELOPE: &str = "Hearing depends on a series of complex steps that change sound waves in the air into electrical signals.";
const CANAL: &str = "Sound waves enter the outer ear and travel through a narrow passageway called the ear canal, which leads to the eardrum.";
const BONES_IN_MIDDLE_EAR: &str = "The eardrum vibrates from the incoming sound waves and sends these vibrations to three tiny bones in the middle ear.";
const BONES_NAMED: &str = "These bones are called the malleus, incus, and stapes.";
const COCHLEA: &str = "The bones in the middle ear amplify, or increase, the sound vibrations and send them to the cochlea, a snail-shaped structure filled with fluid, in the inner ear.";

/// (structure, region, "source" or "cites", that row's sentences in page order)
const ROWS: [(&str, &str, &str, &[&str]); 5] = [
    ("ear_canal", "outer_ear", "cites", &[CANAL]),
    ("malleus", "middle_ear", "cites", &[BONES_IN_MIDDLE_EAR, BONES_NAMED]),
    ("incus", "middle_ear", "cites", &[BONES_IN_MIDDLE_EAR, BONES_NAMED]),
    ("stapes", "middle_ear", "cites", &[BONES_IN_MIDDLE_EAR, BONES_NAMED]),
    ("cochlea", "inner_ear", "source", &[COCHLEA]),
];
const ALL_SENTENCES: [&str; 4] = [CANAL, BONES_IN_MIDDLE_EAR, BONES_NAMED, COCHLEA];

/// The whole citation run a row's answer carries, for its shape.
fn row_citation(kind: &str, spans: &[&str]) -> String {
    if kind == "source" {
        format!("\"source\":\"{}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]", spans[0])
    } else {
        let corr: Vec<String> = spans
            .iter()
            .map(|s| format!("{{\"source\":\"{s}\",\"locator\":\"{LOCATOR}\"}}"))
            .collect();
        format!(
            "\"source\":\"{ENVELOPE}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[{}]",
            corr.join(",")
        )
    }
}

fn place(dir: &Path) {
    let src = facts_stdlib().join("anatomy/ear-parts.adj");
    std::fs::copy(&src, dir.join("ear-parts.adj")).expect("copy shipped ear-parts.adj");
}

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/ear-parts.adj")).expect("read shipped ear-parts.adj");
    adj[adj.find("table ear_structure_region").expect("table")..].to_string()
}

#[test]
fn anatomy_ear_parts_recall_binds_region_with_citation() {
    let dir = scratch("earparts");
    place(&dir);
    std::fs::write(
        dir.join("case.adj"),
        "import \"ear-parts.adj\"\n\
         ? ear_structure_region(cochlea, $R)\n\
         ? ear_structure_region(malleus, $R)\n\
         ? ear_structure_region(ear_canal, $R)\n\
         ? ear_structure_region($S, middle_ear)\n\
         ? ear_structure_region(pinna, $R)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    // The cochlea sits in the inner ear; the malleus (an ossicle) in the middle
    // ear; the ear canal in the outer ear — the recalled regions, each a plain
    // token verbatim from the NIDCD path-of-sound description.
    assert!(out.contains("\"R\":\"inner_ear\""), "cochlea → inner_ear: {out}");
    assert!(out.contains("\"R\":\"middle_ear\""), "malleus → middle_ear: {out}");
    assert!(out.contains("\"R\":\"outer_ear\""), "ear_canal → outer_ear: {out}");
    // The relation runs backward: the region middle_ear recalls the three tiny
    // bones — malleus, incus, and stapes.
    assert!(
        out.contains("\"S\":\"malleus\"")
            && out.contains("\"S\":\"incus\"")
            && out.contains("\"S\":\"stapes\""),
        "middle_ear → malleus/incus/stapes (reverse recall): {out}"
    );
    // The cochlea answer carries its own NIDCD sentence as ONE contiguous run,
    // not `contains("nidcd.nih.gov") && contains(trust)` (#15209).
    assert!(
        out.contains(&row_citation("source", &[COCHLEA])),
        "the cochlea carries its own sentence, whole: {out}"
    );
    // The pinna is not a listed structure — honest abstention, never a
    // fabricated region.
    assert!(
        out.contains("\"abstained\":true"),
        "unknown structure abstains: {out}"
    );
}

#[test]
fn every_structure_answer_carries_the_citation_shape_its_sentences_support() {
    for (structure, region, kind, spans) in ROWS {
        let dir = scratch(&format!("row_{structure}"));
        place(&dir);
        std::fs::write(
            dir.join("case.adj"),
            format!("import \"ear-parts.adj\"\n? ear_structure_region({structure}, $R)\n"),
        )
        .unwrap();
        let (ok, out) = run(&dir.join("case.adj"));
        assert!(ok, "cli should succeed: {out}");
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {structure}: {out}");
        assert!(out.contains(&format!("\"R\":\"{region}\"")), "{structure} is in the {region}: {out}");
        assert!(
            out.contains(&row_citation(kind, spans)),
            "{structure} ({kind}): the measured citation shape, whole: {out}"
        );
        for s in ALL_SENTENCES {
            if !spans.contains(&s) {
                assert!(!out.contains(s), "a sentence that is not {structure}'s must not reach it: {out}");
            }
        }
        if kind == "source" {
            assert!(!out.contains(ENVELOPE), "the envelope is not primary for {structure}: {out}");
        }
    }
}

#[test]
fn the_table_shape_matches_the_measured_rows() {
    let body = shipped_table();
    for (structure, region, kind, spans) in ROWS {
        let mut expected = format!("    row ({structure}, {region}) {{\n");
        for s in spans.iter() {
            if kind == "source" {
                expected.push_str(&format!("        source \"{s}\"\n"));
            } else {
                expected.push_str(&format!("        cites \"{s}\" locator \"{LOCATOR}\"\n"));
            }
        }
        expected.push_str("    }");
        assert!(body.contains(&expected), "row ({structure}, {region}) is shipped in its measured shape");
    }
    assert_eq!(body.matches("\n        source \"").count(), 1, "one row source (the cochlea)");
    assert_eq!(body.matches("\n        cites \"").count(), 7, "seven row corroborations");
    assert!(!body.contains("\n    cites "), "no table-level corroboration");
    assert!(
        !body.contains("\n        locator ") && !body.contains("\n        trust "),
        "no row restates a locator line or trust"
    );
    assert!(
        body.contains(&format!("\n    source \"{ENVELOPE}\"\n    locator \"{LOCATOR}\"\n    trust authoritative\n")),
        "the envelope is the page's framing sentence"
    );
    assert!(!body.contains(&format!("\n    source \"{COCHLEA}\"\n    locator")), "not the cochlea sentence as the envelope again");
    let folded = ENVELOPE.to_lowercase();
    for word in ["canal", "malleus", "incus", "stapes", "cochlea", "outer ear", "middle ear", "inner ear"] {
        assert!(!folded.contains(word), "the envelope must name no structure or region, but contains {word:?}");
    }
}
