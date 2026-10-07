//! Mutating an otherwise real accepted snapshot tests the actual replay
//! boundary. A well-typed record is insufficient if its order contradicts facts.
use coding_adventures_correlation_vector::{CVLog, GraphLimits};
use serde_json::{json, Value};
use std::collections::HashMap;

fn snapshot() -> Value {
    let mut log = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    let root = log.try_create(None).unwrap();
    log.contribute(&root, "first", "one", HashMap::new())
        .unwrap();
    let child = log.try_derive(&root, None).unwrap();
    log.contribute(&child, "second", "two", HashMap::new())
        .unwrap();
    log.try_delete(&root, "third", "gone", HashMap::new())
        .unwrap();
    serde_json::from_str(&log.to_json_string().unwrap()).unwrap()
}
fn rejects(wire: Value) {
    let encoded = serde_json::to_string(&wire).unwrap();
    assert!(
        CVLog::from_checked_json(&encoded, GraphLimits::default()).is_err(),
        "checked accepted: {encoded}"
    );
    assert!(
        CVLog::from_json_string(&encoded).is_err(),
        "compatibility accepted: {encoded}"
    );
}

#[test]
fn malformed_state_and_fixed_scalars_reject() {
    for watermark in [
        json!(0),
        json!("0000000000000004"),
        json!("0000000000000006"),
        json!("000000000000000A"),
    ] {
        let mut wire = snapshot();
        wire["journal"]["last_sequence"] = watermark;
        rejects(wire);
    }
    for sequence in [
        json!("0000000000000000"),
        json!("0000000000000002"),
        json!(1),
        json!("1"),
    ] {
        let mut wire = snapshot();
        wire["journal"]["events"][0]["sequence"] = sequence;
        rejects(wire);
    }
    for coverage in ["partial", "unknown"] {
        let mut wire = snapshot();
        wire["journal"]["coverage"] = json!(coverage);
        rejects(wire);
    }
}

#[test]
fn graph_fact_coverage_and_operation_arity_reject() {
    let mut missing = snapshot();
    missing["journal"]["events"].as_array_mut().unwrap().pop();
    missing["journal"]["last_sequence"] = json!("0000000000000004");
    rejects(missing);
    let mut wrong_create = snapshot();
    wrong_create["journal"]["events"][2]["event"]["kind"] = json!("create");
    rejects(wrong_create);
    let mut wrong_derive = snapshot();
    wrong_derive["journal"]["events"][0]["event"]["kind"] = json!("derive");
    rejects(wrong_derive);
    let mut duplicate = snapshot();
    duplicate["journal"]["events"][1]["event"] = duplicate["journal"]["events"][0]["event"].clone();
    rejects(duplicate);
    let mut unknown = snapshot();
    unknown["journal"]["events"][0]["event"]["entity"] = json!("cv1.0000000000000003");
    rejects(unknown);
}

#[test]
fn invalid_references_and_closed_record_fields_reject() {
    let mut wrong_index = snapshot();
    wrong_index["journal"]["events"][1]["event"]["index"] = json!("0000000000000001");
    rejects(wrong_index);
    let mut stale_context = snapshot();
    stale_context["journal"]["events"][1]["context"] = json!("0000000000000001");
    rejects(stale_context);
    let mut extra = snapshot();
    extra["journal"]["events"][0]["event"]["claimed"] = json!(true);
    rejects(extra);
    let mut missing = snapshot();
    missing["journal"]["events"][0]
        .as_object_mut()
        .unwrap()
        .remove("context");
    rejects(missing);
    let mut unknown = snapshot();
    unknown["journal"]["events"][0]["event"]["kind"] = json!("invented");
    rejects(unknown);
    let mut missing_index = snapshot();
    missing_index["journal"]["events"][1]["event"]
        .as_object_mut()
        .unwrap()
        .remove("index");
    rejects(missing_index);
}

#[test]
fn limits_include_the_graph_facts_and_separate_journal_records() {
    let encoded = serde_json::to_string(&snapshot()).unwrap();
    // C=2, D=1, J=5. Seven cannot retain all eight charges.
    assert!(CVLog::from_checked_json(
        &encoded,
        GraphLimits {
            max_events: 7,
            ..Default::default()
        }
    )
    .is_err());
    CVLog::from_checked_json(
        &encoded,
        GraphLimits {
            max_events: 8,
            ..Default::default()
        },
    )
    .unwrap();
}

#[test]
fn empty_known_journal_and_escaped_presence_reload_and_append() {
    let log = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    let encoded = log.to_json_string().unwrap();
    let escaped = encoded.replace("\"journal\"", "\"\\u006aournal\"");
    for text in [&encoded, &escaped] {
        let mut imported = CVLog::from_json_string(text).unwrap();
        assert_eq!(imported.to_json_string().unwrap(), encoded);
        imported.try_create(None).unwrap();
        assert_eq!(imported.journal().unwrap().last_sequence(), 1);
    }
}

#[test]
fn first_contributor_inventory_matches_the_actual_journal() {
    let mut wire = snapshot();
    wire["pass_order"].as_array_mut().unwrap().swap(0, 1);
    rejects(wire);
}

#[test]
fn deletion_before_retained_contribution_rejects() {
    let mut wire = snapshot();
    let records = wire["journal"]["events"].as_array_mut().unwrap();
    records.swap(1, 4);
    for (index, record) in records.iter_mut().enumerate() {
        record["sequence"] = json!(format!("{:016x}", index + 1));
    }
    rejects(wire);
}

#[test]
fn duplicate_decoded_journal_record_keys_survive_to_the_real_parser() {
    let text = serde_json::to_string(&snapshot()).unwrap();
    let duplicate = text.replacen(
        "\"context\":null",
        "\"context\":null,\"\\u0063ontext\":null",
        1,
    );
    assert!(CVLog::from_checked_json(&duplicate, GraphLimits::default()).is_err());
    assert!(CVLog::from_json_string(&duplicate).is_err());
}
