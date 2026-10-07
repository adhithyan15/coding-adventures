//! Acceptance checks for borrowed views and their shared work/output boundary.
use super::{CVLog, GraphLimits, Origin, SnapshotFormat as F, SourceFilter, SummaryFormat as S};
use serde_json::{json, Value};
use std::collections::HashMap;

fn graph() -> CVLog {
    let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    let root = log
        .try_create(Some(Origin {
            source: "input".into(),
            location: "a.js".into(),
            timestamp: None,
            meta: HashMap::new(),
        }))
        .unwrap();
    let child = log.try_derive(&root, None).unwrap();
    log.contribute(
        &child,
        "fold",
        "changed",
        HashMap::from([
            ("z".into(), json!({"z":1,"a":[true,2,"x"]})),
            ("a".into(), json!(0)),
        ]),
    )
    .unwrap();
    log.try_delete(&child, "emit", "EOF", HashMap::new())
        .unwrap();
    log
}
fn cap(log: &mut CVLog, bytes: usize) {
    log.checked.as_mut().unwrap().limits.max_output_bytes = bytes;
}
fn work(log: &mut CVLog, units: usize) {
    log.checked.as_mut().unwrap().limits.max_work = units;
}

#[test]
fn canonical_views_retain_every_root_field_and_record_without_cloned_json() {
    let log = graph();
    let compact = log
        .export_snapshot(F::CompactJson, SourceFilter::default())
        .unwrap();
    assert_eq!(compact, log.to_json_string().unwrap());
    let pretty = log
        .export_snapshot(F::PrettyJson, SourceFilter::default())
        .unwrap();
    let value: Value = serde_json::from_str(&compact).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&pretty).unwrap(), value);
    assert!(compact.contains("\"a\":[true,2,\"x\"],\"z\":1"));
    let lines = log
        .export_snapshot(F::Ndjson, SourceFilter::default())
        .unwrap();
    let mut records: Vec<Value> = lines
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let mut root = records.pop().unwrap()["_meta"].clone();
    let entries = records
        .into_iter()
        .map(|e| (e["id"].as_str().unwrap().to_string(), e))
        .collect();
    root["entries"] = Value::Object(entries);
    assert_eq!(root, value);
    for format in [F::CompactJson, F::PrettyJson, F::Ndjson] {
        let expected = log
            .export_snapshot(format, SourceFilter::default())
            .unwrap();
        for _ in 0..5 {
            assert_eq!(
                log.export_snapshot(format, SourceFilter::default())
                    .unwrap(),
                expected
            );
        }
    }
}

#[test]
fn snapshot_and_summary_sinks_accept_exact_bytes_and_reject_one_less() {
    for format in [F::CompactJson, F::PrettyJson, F::Ndjson] {
        let mut log = graph();
        let expected = log
            .export_snapshot(format, SourceFilter::default())
            .unwrap();
        cap(&mut log, expected.len());
        assert_eq!(
            log.export_snapshot(format, SourceFilter::default())
                .unwrap(),
            expected
        );
        cap(&mut log, expected.len() - 1);
        assert!(log
            .export_snapshot(format, SourceFilter::default())
            .unwrap_err()
            .contains("output bytes limit"));
        cap(&mut log, 0);
        assert!(log
            .export_snapshot(format, SourceFilter::default())
            .is_err());
    }
    for format in [S::Text, S::Json, S::Kv] {
        let mut log = graph();
        let expected = log
            .export_summary(format, SourceFilter::default(), Some("a \"路径\"\n.cv"))
            .unwrap();
        assert!(expected.ends_with('\n'));
        cap(&mut log, expected.len());
        assert_eq!(
            log.export_summary(format, SourceFilter::default(), Some("a \"路径\"\n.cv"))
                .unwrap(),
            expected
        );
        cap(&mut log, expected.len() - 1);
        assert!(log
            .export_summary(format, SourceFilter::default(), Some("a \"路径\"\n.cv"))
            .unwrap_err()
            .contains("output bytes limit"));
    }
}

#[test]
fn projections_keep_original_parents_metadata_and_declare_partial_coverage() {
    let log = graph();
    for (sources, origin, invert, count) in [
        (vec!["fold".to_string()], false, false, 1),
        (vec!["input".into()], true, false, 1),
        (vec!["fold".into()], false, true, 1),
        (vec!["absent".into()], false, false, 0),
        (vec!["input".into(), "fold".into()], true, false, 2),
    ] {
        let filter = SourceFilter {
            sources: &sources,
            include_origin: origin,
            invert,
        };
        let body = log.export_snapshot(F::CompactJson, filter).unwrap();
        let value: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value["entries"].as_object().unwrap().len(), count);
        assert_eq!(value["view"], json!({"complete":false,"filtered":true}));
        assert_eq!(value["pass_order"], json!(["fold", "emit"]));
        assert!(CVLog::from_checked_json(&body, GraphLimits::default()).is_err());
        let summary = log.export_summary(S::Json, filter, None).unwrap();
        let summary: Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(summary["cv_sidecar"]["entries"], count);
        assert_eq!(summary["cv_sidecar"]["pass_order"], json!(["fold", "emit"]));
        let lines = log.export_snapshot(F::Ndjson, filter).unwrap();
        let footer: Value = serde_json::from_str(lines.lines().last().unwrap()).unwrap();
        assert_eq!(footer["_meta"]["view"], value["view"]);
    }
    assert_eq!(
        log.export_snapshot(
            F::CompactJson,
            SourceFilter {
                invert: true,
                include_origin: true,
                ..Default::default()
            }
        )
        .unwrap(),
        log.to_json_string().unwrap()
    );
}

#[test]
fn complete_validation_precedes_filters_and_summary_counts() {
    let mut invalid = graph();
    invalid
        .entries
        .values_mut()
        .next()
        .unwrap()
        .parent_ids
        .push("missing".into());
    let sources = vec!["absent".into()];
    let filter = SourceFilter {
        sources: &sources,
        ..Default::default()
    };
    for format in [F::CompactJson, F::PrettyJson, F::Ndjson] {
        assert!(invalid.export_snapshot(format, filter).is_err());
    }
    for format in [S::Text, S::Json, S::Kv] {
        assert!(invalid.export_summary(format, filter, None).is_err());
    }
    let disabled = CVLog::new_compact(false);
    assert!(disabled
        .export_snapshot(F::CompactJson, SourceFilter::default())
        .unwrap_err()
        .contains("disabled"));
    let untrusted = CVLog::from_json_string(&graph().to_json_string().unwrap()).unwrap();
    assert!(untrusted
        .export_summary(S::Text, SourceFilter::default(), None)
        .unwrap_err()
        .contains("allocator-only"));
}

#[test]
fn summary_quotes_stage_characters_and_counts_selected_records() {
    let mut log = graph();
    let id = log
        .entries
        .values()
        .find(|entry| entry.deleted.is_none())
        .unwrap()
        .id
        .clone();
    log.contribute(&id, "quote\"\\\n\r\t\u{8}\u{c}\u{1}λ", "x", HashMap::new())
        .unwrap();
    let stages = log.pass_order.join(",");
    let kv = log
        .export_summary(S::Kv, SourceFilter::default(), Some("some path"))
        .unwrap();
    let quoted = kv
        .split("cv_sidecar.pass_order=")
        .nth(1)
        .unwrap()
        .trim_end();
    assert_eq!(serde_json::from_str::<String>(quoted).unwrap(), stages);
    let json: Value = serde_json::from_str(
        &log.export_summary(S::Json, SourceFilter::default(), None)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(json["cv_sidecar"]["contributions"], 2);
    assert_eq!(json["cv_sidecar"]["tombstones"], 1);
    assert_eq!(json["cv_sidecar"]["path"], Value::Null);
    assert_eq!(json["cv_sidecar"]["skipped"], true);
}

#[test]
fn export_work_includes_validation_selection_sorting_and_encoding() {
    let sources = vec!["input".into(), "fold".into()];
    let filter = SourceFilter {
        sources: &sources,
        include_origin: true,
        invert: false,
    };
    for format in [F::CompactJson, F::PrettyJson, F::Ndjson] {
        let mut log = graph();
        let mut low = 0;
        let mut high = 10000;
        while low < high {
            let mid = (low + high) / 2;
            work(&mut log, mid);
            if log.export_snapshot(format, filter).is_ok() {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        work(&mut log, low);
        assert!(log.export_snapshot(format, filter).is_ok());
        work(&mut log, low - 1);
        assert!(log
            .export_snapshot(format, filter)
            .unwrap_err()
            .contains("work limit"));
        // Passing full validation alone does not purchase filtering/encoding.
        assert!(low > sources.len());
    }
}
