//! ADR-5 — the capstone worked example for the decompose-argument-graph arc
//! (ADJ-ARGUMENT-IR §7, §8). It drives the **committed** worked example under
//! `code/specs/data/adj-argument-ir/` through the built binaries to prove the whole
//! pipeline end to end on a paragraph of prose:
//!
//! 1. a technical paragraph (`axle-fatigue.source.txt`) is decomposed into an
//!    `argument { premise… infer… }` program (`axle-fatigue.adj`) whose premises cite
//!    **verbatim byte slices** of that paragraph (`quote "…" at <offset> snapshot "<hex>"`);
//! 2. `adj-lang-cli` **derives** the paragraph's thesis — the axle failed by *fatigue* —
//!    by chaining the inference rules over the premise facts (no argument-specific
//!    evaluator: the argument desugared to facts + rules);
//! 3. `adj-verify --snapshots` **byte-anchors every citation** against the pinned source
//!    (each of the 4 premises and 2 inference warrants) and re-derives the thesis —
//!    so the argument the paragraph makes is machine-checkable back to its own bytes.
//!
//! This is the closing rung of the arc: spec → surface → grounding gate → verify →
//! worked example.

use logic_engine::ContentHash;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The committed worked-example directory, relative to this crate.
fn data_dir() -> PathBuf {
    // adj-lang-cli → rust → packages → code, then specs/data/adj-argument-ir.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/data/adj-argument-ir")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("argwe_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_engine_derives_the_paragraphs_thesis() {
    let adj = data_dir().join("axle-fatigue.adj");
    let out = Command::new(env!("CARGO_BIN_EXE_adj-lang-cli"))
        .arg(&adj)
        .output()
        .expect("run adj-lang-cli");
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success(), "the worked example must compile and run:\n{s}");
    // The paragraph's conclusion — `failed_by(axle, fatigue)` — is DERIVED by chaining
    // the inference rules over the premise facts, not asserted.
    assert!(
        s.contains("\"Mechanism\":\"fatigue\""),
        "the engine must derive the paragraph's thesis (mechanism = fatigue):\n{s}"
    );
}

#[test]
fn adj_verify_byte_anchors_every_citation_to_the_pinned_source() {
    let data = data_dir();
    let doc = std::fs::read(data.join("axle-fatigue.source.txt")).expect("read source doc");
    // Place the committed source document as a content-addressed snapshot, exactly the
    // hex the .adj's pins name — so --snapshots resolves them and verify_quote can run.
    let snaps = scratch("snaps");
    let hex = ContentHash::of(&doc).as_hex().to_string();
    std::fs::write(snaps.join(&hex), &doc).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_adj-verify"))
        .arg("--snapshots")
        .arg(&snaps)
        .arg(data.join("axle-fatigue.adj"))
        .output()
        .expect("run adj-verify");
    let s = String::from_utf8(out.stdout).unwrap();

    assert!(out.status.success(), "the pinned worked example must verify:\n{s}");
    assert!(s.contains("\"verified\":true"), "{s}");
    // Every one of the argument's cited elements — 4 premises + 2 inference warrants —
    // is a verbatim slice of the pinned source, so all six citations are byte-anchored.
    assert!(
        s.contains("\"quotes_verified\":6"),
        "all six of the argument's citations must byte-anchor to the source:\n{s}"
    );
    // The pins that resolved re-check verbatim at their recorded offsets.
    assert!(s.contains("\"status\":\"verified\""), "{s}");
    // And the thesis re-derives (the proof DAG is the argument, re-executed).
    assert!(
        s.contains("\"kind\":\"FromRule\"") && s.contains("\"logic\":\"rechecked\""),
        "the thesis must re-derive from the premises via the inference rules:\n{s}"
    );
}
