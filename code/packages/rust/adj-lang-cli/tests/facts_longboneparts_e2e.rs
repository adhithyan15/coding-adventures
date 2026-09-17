//! End-to-end test for the anatomy FACTS library
//! (`adj-facts-stdlib/anatomy/long-bone-parts.adj`) driven through the built
//! CLI: a native `table` of long-bone region → defining descriptor resolves a
//! binding-query recall with the source's NCBI Bookshelf "Anatomy, Bones"
//! citation, runs the relation backward (descriptor → region), and abstains on
//! a non-region (a tendon) — 0 model calls.
//!
//! Each row carries its OWN sentence as its `source`: every sentence names its
//! part. The envelope used to be the diaphysis line, and so was the primary
//! source of the other four answers. One span was never on the page: the
//! periosteum sentence has a NON-BREAKING SPACE (U+00A0) between "surrounds"
//! and "the", which the table shipped as an ordinary space.

use std::path::{Path, PathBuf};
use std::process::Command;

fn facts_stdlib() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-facts-stdlib")
        .canonicalize()
        .expect("shipped adj-facts-stdlib must exist")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adjcli_factslbp_{tag}_{}", std::process::id()));
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

const LOCATOR: &str = "https://www.ncbi.nlm.nih.gov/books/NBK537199/";
const ENVELOPE: &str = "Long bones evolve via endochondral ossification.";
/// The periosteum string as it used to ship: an ordinary space where the page
/// has U+00A0. It occurs zero times on the page.
const PERIOSTEUM_BEFORE: &str = "The periosteum surrounds the bone surface.";

/// (part, description, that row's own `source`) -- generated from the
/// converter's page-verified spans, not retyped.
const ROWS: [(&str, &str, &str); 5] = [
    ("diaphysis", "shaft", "Diaphysis: Also known as the shaft."),
    ("epiphysis", "tip_of_bone", "Epiphysis: Located at the tip of the long bone, typically responsible for articulation."),
    ("metaphysis", "between_diaphysis_and_epiphysis", "Metaphysis: The region between the diaphysis and epiphysis that contains the epiphyseal plate in children."),
    ("periosteum", "surrounds_bone_surface", "The periosteum surrounds\u{a0}the bone surface."),
    ("epiphyseal_plate", "linear_bone_growth", "Epiphyseal plates are responsible for linear bone growth and remain cartilaginous until after puberty."),
];

fn shipped_table() -> String {
    let adj = std::fs::read_to_string(facts_stdlib().join("anatomy/long-bone-parts.adj"))
        .expect("read shipped long-bone-parts.adj");
    adj[adj.find("table long_bone_part").expect("table")..].to_string()
}

/// The whole citation a row's answer must carry. The serializer writes a
/// non-ASCII character as itself, so U+00A0 appears literally in the output.
fn row_citation(sentence: &str) -> String {
    format!(
        "\"source\":\"{sentence}\",\"locator\":\"{LOCATOR}\",\"trust\":\"authoritative\",\"corroborations\":[]"
    )
}

fn ask(tag: &str, query: &str) -> String {
    assert!(
        tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "scratch tags are path components: {tag:?}"
    );
    let dir = scratch(tag);
    std::fs::copy(facts_stdlib().join("anatomy/long-bone-parts.adj"), dir.join("long-bone-parts.adj"))
        .expect("copy shipped long-bone-parts.adj");
    std::fs::write(dir.join("case.adj"), format!("import \"long-bone-parts.adj\"\n? {query}\n")).unwrap();
    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    out
}

#[test]
fn anatomy_long_bone_parts_recall_binds_descriptor_with_citation() {
    let dir = scratch("longboneparts");
    let src = facts_stdlib().join("anatomy/long-bone-parts.adj");
    std::fs::copy(&src, dir.join("long-bone-parts.adj")).expect("copy shipped long-bone-parts.adj");
    std::fs::write(
        dir.join("case.adj"),
        "import \"long-bone-parts.adj\"\n\
         ? long_bone_part(diaphysis, $D)\n\
         ? long_bone_part(epiphysis, $D)\n\
         ? long_bone_part(metaphysis, $D)\n\
         ? long_bone_part(periosteum, $D)\n\
         ? long_bone_part(epiphyseal_plate, $D)\n\
         ? long_bone_part($P, shaft)\n\
         ? long_bone_part(tendon, $D)\n",
    )
    .unwrap();

    let (ok, out) = run(&dir.join("case.adj"));
    assert!(ok, "cli should succeed: {out}");
    assert!(out.contains("\"recall\""), "has a recall section: {out}");
    assert!(out.contains("\"D\":\"shaft\""), "diaphysis → shaft: {out}");
    assert!(out.contains("\"D\":\"tip_of_bone\""), "epiphysis → tip_of_bone: {out}");
    assert!(
        out.contains("\"D\":\"between_diaphysis_and_epiphysis\""),
        "metaphysis → between_diaphysis_and_epiphysis: {out}"
    );
    assert!(out.contains("\"D\":\"surrounds_bone_surface\""), "periosteum → surrounds_bone_surface: {out}");
    assert!(out.contains("\"D\":\"linear_bone_growth\""), "epiphyseal_plate → linear_bone_growth: {out}");
    assert!(out.contains("\"P\":\"diaphysis\""), "shaft → diaphysis (reverse recall): {out}");
    // WHOLE CONTIGUOUS RUNS, not `contains(host) && contains(trust)` (#15209):
    // every one of the five sentences reaches an answer as a primary source.
    for (part, _, sentence) in ROWS {
        assert!(out.contains(&row_citation(sentence)), "{part}'s sentence is a primary source: {out}");
    }
    assert!(out.contains("\"abstained\":true"), "unknown region abstains: {out}");
}

#[test]
fn every_part_is_warranted_by_its_own_sentence() {
    for (part, desc, sentence) in ROWS {
        let out = ask(&format!("row_{part}"), &format!("long_bone_part({part}, $D)"));
        assert_eq!(out.matches("\"citations\":[").count(), 1, "one answer for {part}: {out}");
        assert!(out.contains(&format!("\"D\":\"{desc}\"")), "{part} → {desc}: {out}");
        assert!(out.contains(&row_citation(sentence)), "{part}: its own sentence is the primary source: {out}");
        for (other, _, other_sentence) in ROWS {
            if other != part {
                assert!(!out.contains(other_sentence), "the {other} sentence must not reach {part}: {out}");
            }
        }
        assert!(!out.contains(ENVELOPE), "the framing envelope warrants no row, including {part}: {out}");
    }
}

#[test]
fn the_periosteum_sentence_carries_the_pages_non_breaking_space() {
    let (_, _, sentence) = ROWS[3];
    assert_eq!(sentence.matches('\u{a0}').count(), 1, "exactly one U+00A0 in the periosteum sentence");
    assert_eq!(sentence.replace('\u{a0}', " "), PERIOSTEUM_BEFORE, "it differs from the old string only there");
    let out = ask("nbsp", "long_bone_part(periosteum, $D)");
    assert!(out.contains(&row_citation(sentence)), "the answer carries the page's character: {out}");
    assert!(!out.contains(PERIOSTEUM_BEFORE), "the ordinary-space string, never on the page, reaches no answer: {out}");
    // SCOPED TO `source "` LINES (#15415), the correction #15337, #15338 and
    // #15417 made for their tables. `shipped_table()` runs to end of file and
    // four `%` comment lines sit inside this block, so read against the slice
    // this arm FAILED A CORRECT FILE when a comment quoted the old sentence --
    // measured by mutant, not reasoned. A comment is not a shipped citation.
    //
    // The trade: nothing now pins "not anywhere in the block", only "no
    // `source` line carries it".
    let source_lines: String = shipped_table()
        .lines()
        .filter(|l| l.trim_start().starts_with("source \""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!source_lines.contains(PERIOSTEUM_BEFORE), "and no `source` line ships it: {source_lines}");
}

#[test]
fn the_table_shape_is_five_row_sources_one_framing_envelope() {
    let body = shipped_table();
    let row_sources: Vec<&str> = body.lines().filter(|l| l.starts_with("        source \"")).collect();
    assert_eq!(row_sources.len(), 5, "one `source` per row: {row_sources:?}");
    assert_eq!(
        body.lines().filter(|l| l.trim_start().starts_with("source \"")).count(),
        6,
        "five row sources and one envelope, at any indentation"
    );
    assert!(body.contains(&format!("\n    source \"{ENVELOPE}\"\n")), "the envelope is the framing sentence");
    assert!(
        !body.lines().any(|l| l.starts_with("        locator ") || l.starts_with("        trust ")),
        "no row restates the envelope's locator or trust (ADJ-TABLES §4)"
    );
    assert!(!body.lines().any(|l| l.trim_start().starts_with("cites ")), "no corroboration anywhere");
    let folded = ENVELOPE.to_lowercase();
    let mut keys = 0;
    for line in body.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("row (") {
            let key = rest.split(',').next().expect("row key").trim();
            keys += 1;
            assert!(!folded.contains(&key.replace('_', " ")), "the envelope must name no part, but names {key:?}");
        }
    }
    assert_eq!(keys, 5, "all five keys were actually checked");
    for (part, desc, sentence) in ROWS {
        assert!(
            body.contains(&format!("    row ({part}, {desc}) {{\n")) && body.contains(&format!("        source \"{sentence}\"\n    }}")),
            "row ({part}, {desc}) is shipped with its own sentence"
        );
    }
}
