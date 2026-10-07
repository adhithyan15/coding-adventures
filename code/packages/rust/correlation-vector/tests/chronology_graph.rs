//! Independent operation order, deliberately different from entity allocation
//! order and first-contributor inventory. Wire assertions exercise the real
//! exporter and both import boundaries, not a second simulated journal.
use coding_adventures_correlation_vector::{CVLog, GraphLimits};
use std::collections::HashMap;

#[test]
fn accepted_graph_operations_have_one_independent_global_clock() {
    let mut log = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    let a = log.try_create(None).unwrap();
    let b = log.try_create(None).unwrap();
    log.contribute(&b, "inspect", "read", HashMap::new())
        .unwrap();
    let child = log.try_derive(&a, None).unwrap();
    let merged = log.try_merge(&[&a, &b, &a], None).unwrap();
    log.try_delete(&a, "drop", "removed", HashMap::new())
        .unwrap();
    let retained_parent = log.try_merge(&[&a], None).unwrap();
    log.contribute(&child, "inner", "rewritten", HashMap::new())
        .unwrap();
    let empty_merge = log.try_merge(&[], None).unwrap();
    log.validate_graph().unwrap();
    assert_eq!(log.journal().unwrap().last_sequence(), 9);
    assert_eq!(log.journal().unwrap().events().len(), 9);
    let encoded = log.to_json_string().unwrap();
    let wire: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    let journal = &wire["journal"];
    assert_eq!(journal["version"], "chronology-v1");
    assert_eq!(journal["coverage"], "full");
    assert_eq!(journal["last_sequence"], "0000000000000009");
    let events = journal["events"].as_array().unwrap();
    let expected = [
        ("create", &a),
        ("create", &b),
        ("contribution", &b),
        ("derive", &child),
        ("merge", &merged),
        ("deletion", &a),
        ("merge", &retained_parent),
        ("contribution", &child),
        ("merge", &empty_merge),
    ];
    for (index, (record, (kind, entity))) in events.iter().zip(expected).enumerate() {
        assert_eq!(record["sequence"], format!("{:016x}", index + 1));
        assert!(record["context"].is_null());
        assert_eq!(record["event"]["kind"], kind);
        assert_eq!(record["event"]["entity"], entity.as_str());
    }
    assert_eq!(events[2]["event"]["index"], "0000000000000000");
    assert_eq!(events[7]["event"]["index"], "0000000000000000");
    assert_eq!(log.get(&merged).unwrap().parent_ids, [&a, &b, &a]);
    assert!(log.get(&a).unwrap().deleted.is_some());
    assert_eq!(log.get(&retained_parent).unwrap().parent_ids, [&a]);
    assert!(log.get(&empty_merge).unwrap().parent_ids.is_empty());
    for checked in [true, false] {
        let mut imported = if checked {
            CVLog::from_checked_json(&encoded, GraphLimits::default()).unwrap()
        } else {
            CVLog::from_json_string(&encoded).unwrap()
        };
        assert_eq!(imported.to_json_string().unwrap(), encoded);
        assert_eq!(imported.try_create(None).unwrap(), "cv1.0000000000000007");
        assert_eq!(imported.journal().unwrap().last_sequence(), 10);
        imported.validate_graph().unwrap();
    }
}

#[test]
fn rejected_graph_and_journal_charge_preserves_both_watermarks() {
    // One creation J plus one contribution fact C and its journal record J.
    let mut log = CVLog::new_checked_chronology(GraphLimits {
        max_events: 3,
        ..Default::default()
    })
    .unwrap();
    let a = log.try_create(None).unwrap();
    log.contribute(&a, "inspect", "read", HashMap::new())
        .unwrap();
    let before = log.to_json_string().unwrap();
    assert!(log.try_create(None).unwrap_err().contains("events limit"));
    assert_eq!(log.to_json_string().unwrap(), before);
    assert!(log
        .contribute(&a, "inspect", "again", HashMap::new())
        .is_err());
    assert_eq!(log.to_json_string().unwrap(), before);
    log.validate_graph().unwrap();
}

#[test]
fn rejected_parent_and_repeated_deletion_do_not_record_attempts() {
    let mut log = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    let a = log.try_create(None).unwrap();
    let before = log.to_json_string().unwrap();
    assert!(log.try_derive("cv1.0000000000000002", None).is_err());
    assert_eq!(log.to_json_string().unwrap(), before);
    log.try_delete(&a, "drop", "removed", HashMap::new())
        .unwrap();
    let before = log.to_json_string().unwrap();
    assert!(log
        .try_delete(&a, "drop", "different", HashMap::new())
        .is_err());
    assert!(log
        .contribute(&a, "inspect", "late", HashMap::new())
        .is_err());
    assert_eq!(log.to_json_string().unwrap(), before);
}
