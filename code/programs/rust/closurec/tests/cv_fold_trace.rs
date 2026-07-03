//! Golden trace test for **per-fold correlation-vector provenance** on the
//! SIMPLE pipeline (CLOC27 P4/P5).
//!
//! This is the positive counterpart to `cv_fold_provenance_gap.rs`: where that
//! test once pinned the *absence* of per-fold lineage (and now pins its
//! presence), this one nails the headline guarantee precisely —
//!
//! > running `report("abc".length);` at `--compilation_level SIMPLE
//! > --correlation_vector` emits `report(3);`, and the folded `3` is traceable
//! > through the correlation-vector sidecar back to the **exact source span**
//! > the value came from: the `"abc"` string literal at line 1, column 8.
//!
//! How the link exists: the parser's CV tokenizer mints a CvId per token with a
//! root `Origin{ source: <input file>, location: "line:col" }` (CLOC03); the
//! bridge stamps that CvId onto the leaf `StringLiteral` (CLOC27 P2/P3); and the
//! constant-fold pass, running against the run's real CVLog (CLOC27 P4),
//! `derive`s the folded `3` from that leaf id. So the `3` has an ancestor whose
//! `Origin` is the `"abc"` source span — the chain reaches the bytes.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// Run closurec at SIMPLE `--correlation_vector` on `src`; return
/// `(sidecar_json, emitted_js, input_path_string)`.
fn run(src: &str) -> (serde_json::Value, String, String) {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "closurec_cvtrace_{}_{}",
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

    let emitted = std::fs::read_to_string(&out).expect("read output js");
    let cv_path = std::path::PathBuf::from(format!("{}.cv.json", out.display()));
    let text = std::fs::read_to_string(&cv_path)
        .unwrap_or_else(|e| panic!("read sidecar {}: {e}", cv_path.display()));
    let json: serde_json::Value = serde_json::from_str(&text).expect("parse sidecar JSON");
    let input_path = input.to_string_lossy().into_owned();
    let _ = std::fs::remove_dir_all(&dir);
    (json, emitted, input_path)
}

#[test]
fn folded_literal_is_traceable_to_its_source_span() {
    let (j, emitted, input_path) = run("report(\"abc\".length);\n");

    // The fold actually happened: `"abc".length` ⇒ `3`.
    assert_eq!(
        emitted.trim(),
        "report(3);",
        "expected the SIMPLE pipeline to fold `\"abc\".length` to `3`",
    );

    let entries = j["entries"].as_object().expect("sidecar `entries` object");

    // The headline link: a CV entry rooted at the `"abc"` string-literal span.
    // `report(` is 7 chars, so the opening `"` of `"abc"` is at column 8 of
    // line 1. The parser's CV tokenizer stamps `Origin{ source: <input file>,
    // location: "1:8" }` on that token; the bridge carries it onto the leaf
    // `StringLiteral`; the fold derives the `3` from it. Its presence in the
    // sidecar is the proof the folded value traces to the bytes it came from.
    let abc_span = entries.values().find(|e| {
        e["origin"]["source"].as_str() == Some(input_path.as_str())
            && e["origin"]["location"].as_str() == Some("1:8")
    });
    assert!(
        abc_span.is_some(),
        "expected a CV entry whose Origin is the `\"abc\"` source span \
         (source == {input_path:?}, location == \"1:8\"); without it the folded \
         `3` would not be traceable to its source bytes. entries: {}",
        serde_json::to_string(&j["entries"]).unwrap_or_default(),
    );
}
