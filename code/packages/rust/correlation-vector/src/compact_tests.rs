use super::*;

#[test]
fn compact_allocations_share_one_sequence_and_keep_parent_edges() {
    let mut log = CVLog::new_compact(true);
    let a = log
        .try_create(Some(Origin {
            source: "example.js".into(),
            location: "3:4".into(),
            timestamp: Some("2026-10-07".into()),
            meta: [("span".into(), serde_json::json!({"start":4,"end":8}))].into(),
        }))
        .unwrap();
    let b = log.try_derive(&a, None).unwrap();
    let c = log.try_create(None).unwrap();
    let m = log.try_merge(&[&b, &c, &b], None).unwrap();
    assert_eq!(a, "cv1.0000000000000001");
    assert_eq!(b, "cv1.0000000000000002");
    assert_eq!(c, "cv1.0000000000000003");
    assert_eq!(m, "cv1.0000000000000004");
    assert_eq!(log.get(&m).unwrap().parent_ids, vec![b.clone(), c, b]);
    log.contribute(&a, "pass", "changed", HashMap::new())
        .unwrap();
    log.delete(&a, "dce", "removed", HashMap::new());
    let loaded = CVLog::from_json_string(&log.to_json_string().unwrap()).unwrap();
    let origin = loaded.get(&a).unwrap().origin.as_ref().unwrap();
    assert_eq!(origin.source, "example.js");
    assert_eq!(origin.location, "3:4");
    assert_eq!(origin.timestamp.as_deref(), Some("2026-10-07"));
    assert_eq!(origin.meta["span"], serde_json::json!({"start":4,"end":8}));
    assert_eq!(loaded.history(&a)[0].tag, "changed");
    assert_eq!(
        loaded.get(&a).unwrap().deleted.as_ref().unwrap().reason,
        "removed"
    );
}

#[test]
fn reload_keeps_allocations_that_were_not_stored() {
    for initially_enabled in [false, true] {
        let mut log = CVLog::new_compact(initially_enabled);
        let first = log.create(None);
        log.enabled = false;
        let hidden = log.derive(&first, None);
        let snapshot = log.to_json_string().unwrap();
        let value: Value = serde_json::from_str(&snapshot).unwrap();
        assert_eq!(value["identity"]["last_sequence"], "0000000000000002");
        let mut loaded = CVLog::from_json_string(&snapshot).unwrap();
        loaded.enabled = true;
        let next = loaded.create(None);
        assert_eq!(next, "cv1.0000000000000003");
        assert_ne!(next, first);
        assert_ne!(next, hidden);
    }
}

#[test]
fn malformed_compact_identity_state_is_rejected() {
    let mut log = CVLog::new_compact(true);
    let id = log.create(None);
    let valid: Value = serde_json::from_str(&log.to_json_string().unwrap()).unwrap();
    let mut cases = Vec::new();
    let mut bad = valid.clone();
    bad["identity"]["scheme"] = "compact-v2".into();
    cases.push(bad);
    let mut bad = valid.clone();
    bad["identity"]["last_sequence"] = "1".into();
    cases.push(bad);
    let mut bad = valid.clone();
    bad["identity"]["last_sequence"] = "0000000000000000".into();
    cases.push(bad);
    let mut bad = valid.clone();
    bad["entries"][&id]["id"] = "cv1.0000000000000002".into();
    cases.push(bad);
    let mut bad = valid.clone();
    bad["entries"][&id]["parent_ids"] = serde_json::json!(["cv1.0000000000000009"]);
    cases.push(bad);
    let mut bad = valid.clone();
    bad["view"] = serde_json::json!({"filtered":true,"complete":false});
    cases.push(bad);
    for view in [
        serde_json::json!({"filtered":false,"complete":false}),
        Value::Bool(false),
        Value::Null,
    ] {
        let mut bad = valid.clone();
        bad["view"] = view;
        cases.push(bad);
    }
    let mut bad = valid;
    bad.as_object_mut().unwrap().remove("identity");
    cases.push(bad);
    for case in cases {
        assert!(
            CVLog::from_json_string(&case.to_string()).is_err(),
            "accepted {case}"
        );
    }
}

#[test]
fn allocation_exhaustion_and_collision_leave_log_unchanged() {
    let mut log = CVLog::new_compact(true);
    log.compact_sequence = Some(u64::MAX);
    let before: Value = serde_json::from_str(&log.to_json_string().unwrap()).unwrap();
    assert!(log.try_create(None).is_err());
    assert!(log.try_derive("parent", None).is_err());
    assert!(log.try_merge(&["parent"], None).is_err());
    assert_eq!(
        before,
        serde_json::from_str::<Value>(&log.to_json_string().unwrap()).unwrap()
    );
    let mut log = CVLog::new_compact(true);
    let id = log.create(None);
    log.compact_sequence = Some(0);
    assert!(log.try_create(None).is_err());
    assert_eq!(log.compact_sequence, Some(0));
    assert_eq!(log.entries.len(), 1);
    assert!(log.entries.contains_key(&id));
}

#[test]
fn legacy_counters_do_not_wrap_or_change_on_failed_allocation() {
    let mut log = CVLog::new(true);
    log.base_counters.insert("00000000".into(), u32::MAX);
    log.child_counters.insert("parent".into(), u32::MAX);
    assert!(log.try_create(None).is_err());
    assert!(log.try_merge(&[], None).is_err());
    assert!(log.try_derive("parent", None).is_err());
    assert_eq!(log.base_counters["00000000"], u32::MAX);
    assert_eq!(log.child_counters["parent"], u32::MAX);
    assert!(log.entries.is_empty());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| log.create(None))).is_err());
    assert_eq!(log.base_counters["00000000"], u32::MAX);
    assert!(log.entries.is_empty());
}

#[test]
fn direct_deep_chains_have_constant_ids_and_linear_identity_storage() {
    let mut sizes = Vec::new();
    for depth in [1024, 2048, 4096] {
        let mut log = CVLog::new_compact(true);
        let mut last = log.create(None);
        for _ in 0..depth {
            last = log.derive(&last, None);
        }
        assert!(log.entries.keys().all(|id| id.len() == 20));
        let bytes = log.to_json_string().unwrap().len();
        assert!(bytes < (depth + 1) * 200);
        sizes.push(bytes);
        let mut restored = CVLog::from_json_string(&log.to_json_string().unwrap()).unwrap();
        assert_ne!(restored.derive(&last, None), last);
    }
    assert!(sizes.windows(2).all(|pair| pair[1] < pair[0] * 2 + 200));
}
