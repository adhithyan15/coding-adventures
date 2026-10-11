//! Every presentation must preserve chronology or explicitly declare its
//! projection partial. NDJSON reconstructs through a tested adapter, not a
//! fictional library loader; its footer retains state without another event list.
use coding_adventures_correlation_vector::{CVLog, GraphLimits, SnapshotFormat, SourceFilter};
use serde_json::{json, Value};
use std::collections::HashMap;

fn log() -> CVLog {
    let mut log = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    let parent = log.try_create(None).unwrap();
    let child = log.try_derive(&parent, None).unwrap();
    log.contribute(&child, "selected", "used", HashMap::new())
        .unwrap();
    log
}
#[test]
fn json_and_pretty_snapshot_preserve_complete_journal() {
    let log = log();
    let canonical = log.to_json_string().unwrap();
    for format in [SnapshotFormat::CompactJson, SnapshotFormat::PrettyJson] {
        let wire = log
            .export_snapshot(format, SourceFilter::default())
            .unwrap();
        let imported = CVLog::from_checked_json(&wire, GraphLimits::default()).unwrap();
        assert_eq!(imported.to_json_string().unwrap(), canonical);
    }
}
#[test]
fn ndjson_has_event_frames_and_state_only_footer_with_lossless_reload() {
    let log = log();
    let lines = log
        .export_snapshot(SnapshotFormat::Ndjson, SourceFilter::default())
        .unwrap();
    let mut entries = serde_json::Map::new();
    let mut events = Vec::new();
    let mut footer = None;
    for line in lines.lines() {
        let mut value: Value = serde_json::from_str(line).unwrap();
        if value.get("_meta").is_some() {
            footer = Some(value["_meta"].take());
        } else if value.get("_event").is_some() {
            events.push(value["_event"].take());
        } else {
            entries.insert(value["id"].as_str().unwrap().to_owned(), value);
        }
    }
    assert_eq!(events.len(), 3);
    let mut wire = footer.unwrap();
    assert!(wire["journal"].get("events").is_none());
    assert_eq!(wire["journal"]["last_sequence"], "0000000000000003");
    wire["entries"] = Value::Object(entries);
    wire["journal"]["events"] = Value::Array(events);
    let imported = CVLog::from_checked_json(
        &serde_json::to_string(&wire).unwrap(),
        GraphLimits::default(),
    )
    .unwrap();
    assert_eq!(
        imported.to_json_string().unwrap(),
        log.to_json_string().unwrap()
    );
}
#[test]
fn selected_entity_keeps_original_sequence_gaps_and_partial_watermark() {
    let sources = ["selected".to_owned()];
    let wire = log()
        .export_snapshot(
            SnapshotFormat::CompactJson,
            SourceFilter {
                sources: &sources,
                ..Default::default()
            },
        )
        .unwrap();
    let value: Value = serde_json::from_str(&wire).unwrap();
    assert_eq!(value["journal"]["coverage"], "partial");
    assert_eq!(value["journal"]["last_sequence"], "0000000000000003");
    assert_eq!(value["journal"]["events"].as_array().unwrap().len(), 2);
    assert_eq!(
        value["journal"]["events"][0]["sequence"],
        "0000000000000002"
    );
    assert_eq!(
        value["journal"]["events"][1]["sequence"],
        "0000000000000003"
    );
    assert_eq!(value["view"]["complete"], json!(false));
    assert!(CVLog::from_checked_json(&wire, GraphLimits::default()).is_err());
    assert!(CVLog::from_json_string(&wire).is_err());
}

#[test]
fn selecting_every_entity_still_declares_a_partial_journal_view() {
    let mut log = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    let root = log
        .try_create(Some(coding_adventures_correlation_vector::Origin {
            source: "selected".into(),
            location: "1:1".into(),
            timestamp: None,
            meta: HashMap::new(),
        }))
        .unwrap();
    let child = log.try_derive(&root, None).unwrap();
    log.contribute(&child, "selected", "used", HashMap::new())
        .unwrap();
    let sources = ["selected".to_owned()];
    let text = log
        .export_snapshot(
            SnapshotFormat::CompactJson,
            SourceFilter {
                sources: &sources,
                include_origin: true,
                invert: false,
            },
        )
        .unwrap();
    let wire: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        wire["entries"].as_object().unwrap().len(),
        log.entries().len()
    );
    assert_eq!(
        wire["journal"]["events"].as_array().unwrap().len(),
        log.journal().unwrap().events().len()
    );
    assert_eq!(wire["journal"]["coverage"], "partial");
    assert!(CVLog::from_json_string(&text).is_err());
    assert!(CVLog::from_checked_json(&text, GraphLimits::default()).is_err());
}
