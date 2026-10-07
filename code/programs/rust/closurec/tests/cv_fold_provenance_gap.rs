//! CLOC27 lexer/leaf plumbing smoke test, retained from the original gap probe.
//!
//! This checks token origins and the scheduler-derived pass summary. Token
//! presence alone does not prove a fold's lineage. `cv_fold_trace.rs` provides
//! the stronger CLOC31 contract: result-owned events and graph paths to operands.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// Run closurec at SIMPLE with `--correlation_vector` on `src` and return the
/// parsed sidecar JSON together with the input file's path string (the string
/// the parser's CV tokenizer uses as each token's `Origin.source`, so the test
/// can match per-token source-span origins). Uses a unique temp dir per call
/// (no predictable shared path; safe under parallel `cargo test`).
fn run_cv_sidecar(src: &str) -> (serde_json::Value, String) {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "closurec_cvgap_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed),
    ));
    std::fs::create_dir(&dir).expect("mk temp dir");
    let input = dir.join("a.js");
    std::fs::write(&input, src).expect("write input");
    let out = dir.join("out.js");

    let res = Command::new(BINARY)
        .args([
            "--compilation_level",
            "SIMPLE",
            "--correlation_vector",
            "--js",
            input.to_str().unwrap(),
            "--js_output_file",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("run closurec");
    assert!(
        res.status.success(),
        "closurec failed: exit {:?}, stderr {}",
        res.status.code(),
        String::from_utf8_lossy(&res.stderr),
    );

    // Sidecar is written next to the output as `<output>.cv.json`.
    let cv_path = std::path::PathBuf::from(format!("{}.cv.json", out.display()));
    let text = std::fs::read_to_string(&cv_path)
        .unwrap_or_else(|e| panic!("read sidecar {}: {e}", cv_path.display()));
    let json: serde_json::Value = serde_json::from_str(&text).expect("parse sidecar JSON");
    let input_path = input.to_string_lossy().into_owned();
    let _ = std::fs::remove_dir_all(&dir);
    (json, input_path)
}

/// Lex/file-level origin sources — the only ones the SIMPLE trace currently
/// emits. A source outside this set means per-fold (or per-pass) provenance
/// started reaching the sidecar.
const LEX_FILE_ORIGINS: &[&str] = &[
    "lexer_token",
    "input_file",
    "js_output_file",
    "concatenated_combined_source",
];

/// True iff `loc` looks like a per-token `line:col` span (both 1-based ints) —
/// the location the parser's CV tokenizer stamps on each token's `Origin`.
/// Distinguishes a real source-span origin from the coarse `0:0` program-root
/// / file-level locations.
fn is_line_col(loc: &str) -> bool {
    matches!(loc.split_once(':'), Some((l, c))
        if !l.is_empty() && !c.is_empty()
        && l.bytes().all(|b| b.is_ascii_digit())
        && c.bytes().all(|b| b.is_ascii_digit()))
}

#[test]
fn typed_pipeline_records_per_token_origins_and_pass_summary() {
    // `"abc".length` folds to `3`; `report(...)` keeps it referenced so the
    // value survives remove-unused-vars/treeshake and the fold is real.
    let (j, input_path) = run_cv_sidecar("report(\"abc\".length);\n");
    let entries = j["entries"].as_object().expect("sidecar `entries` object");
    assert!(!entries.is_empty(), "sidecar should have entries");

    // (1) The constant-fold pass ran — recorded in the coarse simple_v2 summary.
    let mut saw_constant_fold_pass = false;
    let mut origin_sources: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // (2) Per-token source-span origins — minted by the parser's CV tokenizer
    // (CLOC27 P2/P3), `Origin.source == <input path>`, `location == line:col`.
    // This asserts source-token inventory only. It cannot establish a path
    // from a transformed result; the graph tests in cv_fold_trace do that.
    let mut per_token_span_origins = 0usize;
    for (_id, e) in entries {
        if let Some(src) = e["origin"]["source"].as_str() {
            origin_sources.insert(src.to_string());
            let loc = e["origin"]["location"].as_str().unwrap_or("");
            if src == input_path && is_line_col(loc) {
                per_token_span_origins += 1;
            }
        }
        for c in e["contributions"].as_array().into_iter().flatten() {
            if c["tag"].as_str() == Some("simple_v2") {
                if let Some(passes) = c["meta"]["passes"].as_array() {
                    if passes.iter().any(|p| p.as_str() == Some("constant-fold")) {
                        saw_constant_fold_pass = true;
                    }
                }
            }
        }
    }
    assert!(
        saw_constant_fold_pass,
        "expected the simple_v2 contribution to list `constant-fold` among its passes; \
         origins seen: {origin_sources:?}",
    );

    // (2) Leaf-token origins coexist with the file/pass-summary records.
    // Their presence does not imply that any replacement reaches them.
    assert!(
        per_token_span_origins > 0,
        "expected at least one per-token source-span origin (source == {input_path:?}, \
         location == line:col) proving leaf-token origins reach the sidecar; \
         origins seen: {origin_sources:?}",
    );

    // The coarse lex/file-level origins still coexist — per-token provenance was
    // *added* to the trace, not substituted for the file/pass-summary records.
    let has_lex_file = origin_sources
        .iter()
        .any(|s| LEX_FILE_ORIGINS.contains(&s.as_str()));
    assert!(
        has_lex_file,
        "coarse lex/file origins ({LEX_FILE_ORIGINS:?}) should still be present alongside \
         the per-token spans; origins seen: {origin_sources:?}",
    );
}
