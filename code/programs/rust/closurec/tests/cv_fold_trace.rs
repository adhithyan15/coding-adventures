//! CLOC31: inspect the replacement's parent graph, not token presence.
//!
//! A tokenizer alone can populate every source origin without connecting a
//! single optimized value to it. These tests therefore start at the result's
//! own fold contribution and follow actual parent links to operand origins.
//! Nested folds must retain intermediate transformations; unrelated tokens
//! must stay outside the ancestry. Output bytes must agree with tracing off.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// Run closurec at SIMPLE `--correlation_vector` on `src`; return
/// `(sidecar_json, emitted_js, input_path_string)`.
fn run(src: &str, level: &str) -> (serde_json::Value, String, String) {
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
            level,
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
    let untraced = dir.join("plain.js");
    let plain = Command::new(BINARY)
        .args([
            "--compilation_level",
            level,
            "--js",
            input.to_str().unwrap(),
            "--js_output_file",
            untraced.to_str().unwrap(),
        ])
        .output()
        .expect("run without tracing");
    assert!(plain.status.success(), "untraced compile failed: {plain:?}");
    assert_eq!(
        emitted,
        std::fs::read_to_string(untraced).unwrap(),
        "tracing changed emitted JavaScript"
    );
    let input_path = input.to_string_lossy().into_owned();
    let _ = std::fs::remove_dir_all(&dir);
    (json, emitted, input_path)
}

fn fold_id(j: &serde_json::Value, after: &str) -> String {
    let matches: Vec<_> = j["entries"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(id, e)| {
            e["contributions"].as_array().unwrap().iter().any(|c| {
                c["source"] == "constant-fold"
                    && c["tag"] == "folded"
                    && c["meta"]["after"] == after
                    && c["meta"]["new_cv"] == id.as_str()
            })
        })
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "expected one result-owned fold to {after:?}"
    );
    matches[0].0.clone()
}

/// Follow edges with separate active/visited sets: shared ancestors are valid,
/// cycles and missing parents are not. Return the reachable identities.
fn ancestry(j: &serde_json::Value, id: &str) -> std::collections::BTreeSet<String> {
    fn walk(
        j: &serde_json::Value,
        id: &str,
        active: &mut std::collections::BTreeSet<String>,
        seen: &mut std::collections::BTreeSet<String>,
    ) {
        assert!(!active.contains(id), "cyclic lineage at {id}");
        if !seen.insert(id.to_string()) {
            return;
        }
        let e = j["entries"]
            .get(id)
            .unwrap_or_else(|| panic!("dangling parent {id}"));
        active.insert(id.to_string());
        for p in e["parent_ids"].as_array().unwrap() {
            walk(j, p.as_str().unwrap(), active, seen);
        }
        active.remove(id);
    }
    let mut seen = std::collections::BTreeSet::new();
    walk(j, id, &mut std::collections::BTreeSet::new(), &mut seen);
    assert!(seen.len() > 1, "folded result has no parent lineage");
    seen
}

fn assert_operand_spans(j: &serde_json::Value, id: &str, path: &str, expected: &[&str]) {
    let found: std::collections::BTreeSet<_> = ancestry(j, id)
        .iter()
        .filter_map(|id| {
            let origin = &j["entries"][id]["origin"];
            (origin["source"].as_str() == Some(path))
                .then(|| origin["location"].as_str().unwrap().to_string())
        })
        .collect();
    assert_eq!(
        found,
        expected.iter().map(|s| s.to_string()).collect(),
        "result ancestry must contain its operand spans and no unrelated tokens"
    );
}

#[test]
fn folded_literal_is_traceable_to_its_source_span() {
    for level in ["SIMPLE", "ADVANCED"] {
        let (j, emitted, path) = run("report(\"abc\".length);\n", level);
        assert_eq!(emitted.trim(), "report(3);");
        let id = fold_id(&j, "3");
        assert_operand_spans(&j, &id, &path, &["1:8"]);
    }
}

#[test]
fn binary_fold_retains_both_operands() {
    let (j, emitted, path) = run("report(2+3);\n", "SIMPLE");
    assert_eq!(emitted.trim(), "report(5);");
    assert_operand_spans(&j, &fold_id(&j, "5"), &path, &["1:8", "1:10"]);
}

#[test]
fn unary_fold_retains_its_operand() {
    let (j, emitted, path) = run("report(-2);\n", "SIMPLE");
    assert_eq!(emitted.trim(), "report(-2);");
    assert_operand_spans(&j, &fold_id(&j, "-2"), &path, &["1:9"]);
}

#[test]
fn nested_fold_retains_intermediate_rewrite_and_all_operands() {
    let (j, emitted, path) = run("report(1+2*3);\n", "SIMPLE");
    assert_eq!(emitted.trim(), "report(7);");
    let outer = fold_id(&j, "7");
    let inner = fold_id(&j, "6");
    assert!(
        ancestry(&j, &outer).contains(&inner),
        "lost child fold history"
    );
    assert_operand_spans(&j, &outer, &path, &["1:8", "1:10", "1:12"]);
}

#[test]
fn declined_fold_does_not_claim_a_rewrite() {
    let (j, emitted, _) = run("report(1/0);\n", "SIMPLE");
    assert_eq!(emitted.trim(), "report(1/0);");
    assert!(!j["entries"]
        .as_object()
        .unwrap()
        .values()
        .any(|e| e["contributions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["source"] == "constant-fold" && c["tag"] == "folded")));
}

#[test]
fn equal_folded_branches_ignore_identity_and_keep_both_histories() {
    let (j, emitted, path) = run("report(flag?(1+1):(1+1));\n", "SIMPLE");
    assert_eq!(emitted.trim(), "report(2);");
    let entries = j["entries"].as_object().unwrap();
    let (id, _) = entries
        .iter()
        .find(|(id, e)| {
            e["contributions"].as_array().unwrap().iter().any(|c| {
                c["source"] == "constant-fold"
                    && c["tag"] == "folded"
                    && c["meta"]["before"] == "t ? X : X"
                    && c["meta"]["new_cv"] == id.as_str()
            })
        })
        .expect("collapsed primitive branches own a rewrite record");
    assert_operand_spans(&j, id, &path, &["1:8", "1:14", "1:16", "1:20", "1:22"]);
    let history = ancestry(&j, id);
    let branch_folds = history
        .iter()
        .filter(|ancestor| {
            entries[*ancestor]["contributions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| {
                    c["source"] == "constant-fold"
                        && c["meta"]["after"] == "2"
                        && c["meta"]["new_cv"] == ancestor.as_str()
                })
        })
        .count();
    assert_eq!(branch_folds, 2, "both folded branch histories must survive");
}

#[test]
fn equal_composite_branches_optimize_identically_with_tracing() {
    for level in ["SIMPLE", "ADVANCED"] {
        for (source, expected) in [
            ("report(flag?[1+1]:[1+1]);\n", "report([2]);"),
            ("report(flag?f(1+1):f(1+1));\n", "report(f(2));"),
        ] {
            let (_, emitted, _) = run(source, level);
            assert_eq!(emitted.trim(), expected, "{level}: {source}");
        }
    }
}

#[test]
fn opposite_signed_zero_branches_preserve_the_choice() {
    for level in ["SIMPLE", "ADVANCED"] {
        for (source, expected) in [
            ("report(flag?(-0):(0));\n", "report(flag?-0:0);"),
            ("report(flag?(0):(-0));\n", "report(flag?0:-0);"),
            ("report(flag?[-0]:[0]);\n", "report(flag?[-0]:[0]);"),
            ("report(flag?[0]:[-0]);\n", "report(flag?[0]:[-0]);"),
        ] {
            let (_, emitted, _) = run(source, level);
            assert_eq!(emitted.trim(), expected, "{level}: {source}");
        }
    }
}
