//! AC-2 — the worked WHOLE-PAPER example for ADJ-ARGUMENT-COMPOSITION. It drives the
//! committed multi-paragraph example under `code/specs/data/adj-argument-ir/composition/`
//! through the built binaries to prove whole-paper composition end to end:
//!
//! 1. three short paragraphs (`p2-loading`, `p3-fractography`, `p4-discussion`), each its
//!    own pinned source document, are composed into ONE `argument` (`axle-paper.adj`)
//!    whose seven citations each name **their own paragraph's** `snapshot`;
//! 2. `adj-lang-cli` **derives** the paper's thesis — `failed_by(axle, fatigue)` — by
//!    chaining across the three paragraphs (`i3` ← `i1` + `i2` ← their premises); and
//! 3. `adj-verify --snapshots` **byte-anchors all seven citations across the three
//!    snapshots** (the multi-snapshot proof) and re-derives the thesis.
//!
//! This is the empirical proof of the AC-1 finding: whole-paper composition needs zero new
//! constructs, and grounding stays per-paragraph.

use logic_engine::ContentHash;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The committed worked-example directory, relative to this crate.
fn data_dir() -> PathBuf {
    // adj-lang-cli → rust → packages → code, then specs/data/adj-argument-ir/composition.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../specs/data/adj-argument-ir/composition")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("compwe_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Place every `*.source.txt` paragraph in `data_dir` as a content-addressed snapshot
/// (filename = its SHA-256 hex) in `snaps` — so `--snapshots` resolves each paragraph's pins.
fn place_all_snapshots(snaps: &Path) -> usize {
    let mut n = 0;
    for entry in std::fs::read_dir(data_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("txt")
            && path.file_name().unwrap().to_string_lossy().ends_with(".source.txt")
        {
            let bytes = std::fs::read(&path).unwrap();
            let hex = ContentHash::of(&bytes).as_hex().to_string();
            std::fs::write(snaps.join(&hex), &bytes).unwrap();
            n += 1;
        }
    }
    n
}

#[test]
fn the_engine_derives_the_papers_thesis_across_paragraphs() {
    let adj = data_dir().join("axle-paper.adj");
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(&adj)
        .output()
        .expect("run adj-lang-cli");
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success(), "the whole-paper example must compile and run:\n{s}");
    // The paper's thesis — `failed_by(axle, fatigue)` — is DERIVED by chaining the three
    // paragraphs' inferences (i3 ← i1 + i2), not asserted.
    assert!(
        s.contains("\"Mechanism\":\"fatigue\""),
        "the engine must derive the paper's thesis (mechanism = fatigue):\n{s}"
    );
}

#[test]
fn adj_verify_byte_anchors_all_citations_across_three_snapshots() {
    let snaps = scratch("snaps");
    let placed = place_all_snapshots(&snaps);
    assert_eq!(placed, 3, "the paper has three paragraph source snapshots");

    let out = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .arg("--snapshots")
        .arg(&snaps)
        .arg(data_dir().join("axle-paper.adj"))
        .output()
        .expect("run adj-verify");
    let s = String::from_utf8(out.stdout).unwrap();

    assert!(out.status.success(), "the pinned whole-paper example must verify:\n{s}");
    assert!(s.contains("\"verified\":true"), "{s}");
    // All seven citations — 4 premises + 3 inference warrants, spread across THREE paragraph
    // snapshots — byte-anchor. This is the multi-snapshot proof: each paragraph's citations
    // re-check against the paragraph they came from.
    assert!(
        s.contains("\"quotes_verified\":7"),
        "all seven citations must byte-anchor across the three snapshots:\n{s}"
    );
    // The pins that resolved re-check verbatim at their recorded offsets, and the thesis
    // re-derives (the cross-paragraph proof DAG, re-executed).
    assert!(s.contains("\"status\":\"verified\""), "{s}");
    assert!(
        s.contains("\"kind\":\"FromRule\"") && s.contains("\"logic\":\"rechecked\""),
        "the paper thesis must re-derive from the paragraphs' inferences:\n{s}"
    );
}

#[test]
fn explain_renders_the_cross_paragraph_chain_with_per_paragraph_provenance() {
    let adj = data_dir().join("axle-paper.adj");
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg("--explain")
        .arg(&adj)
        .output()
        .expect("run adj-lang-cli --explain");
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success(), "--explain must succeed:\n{s}");

    // The conclusion, and the two intermediate paragraph conclusions it rests on, each
    // render as inference steps — the cross-paragraph chain.
    assert!(s.contains("failed_by(axle, fatigue)  <= inference"), "{s}");
    assert!(s.contains("exceeds_endurance(axle)  <= inference"), "{s}");
    assert!(s.contains("fatigue_indicated(axle)  <= inference"), "{s}");
    // Each step keeps ITS OWN paragraph's provenance — the discussion thesis, the loading
    // and fractography intermediates are attributed to their distinct paragraphs.
    assert!(
        s.contains("source \"p4-discussion\"")
            && s.contains("source \"p2-loading\"")
            && s.contains("source \"p3-fractography\""),
        "each step must carry its own paragraph's provenance:\n{s}"
    );
}
