use super::*;

fn snapshot(log: &CVLog) -> Value {
    serde_json::from_str(&log.to_json_string().unwrap()).unwrap()
}

#[test]
fn checked_unknown_and_future_parents_fail_before_allocation() {
    let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    let before = snapshot(&log);
    assert!(log.try_derive("cv1.0000000000000001", None).is_err());
    assert!(log.try_merge(&["cv1.0000000000000001"], None).is_err());
    assert_eq!(snapshot(&log), before);
    let root = log.try_create(None).unwrap();
    assert_eq!(root, "cv1.0000000000000001");
    let before = snapshot(&log);
    assert!(log.try_derive("cv1.0000000000000002", None).is_err());
    assert!(log
        .try_merge(&[&root, "cv1.0000000000000002"], None)
        .is_err());
    assert_eq!(snapshot(&log), before);
    assert_eq!(log.try_derive(&root, None).unwrap(), "cv1.0000000000000002");
}

#[test]
fn shared_ancestor_lineage_is_deterministic_and_parent_before_child() {
    let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    let a = log.try_create(None).unwrap();
    let b = log.try_derive(&a, None).unwrap();
    let c = log.try_derive(&b, None).unwrap();
    let independent = log.try_create(None).unwrap();
    let m = log.try_merge(&[&a, &c, &c, &independent], None).unwrap();
    let expected = vec![a, b, c, independent, m.clone()];
    for _ in 0..3 {
        let lineage = log.try_lineage(&m).unwrap();
        let ids: Vec<_> = lineage.iter().map(|entry| entry.id.clone()).collect();
        assert_eq!(ids, expected);
        for (child_index, entry) in lineage.iter().enumerate() {
            for parent in &entry.parent_ids {
                let parent_index = ids.iter().position(|id| id == parent).unwrap();
                assert!(parent_index < child_index);
            }
        }
    }
    assert!(log.try_lineage("missing").is_err());
    assert!(log.try_ancestors("missing").is_err());
    assert!(log.try_descendants("missing").is_err());
}

#[test]
fn checked_node_edge_and_event_caps_reject_without_changing_state() {
    let limits = GraphLimits {
        max_nodes: 3,
        max_edges: 2,
        max_events: 1,
        ..GraphLimits::default()
    };
    let mut log = CVLog::new_checked_compact(limits).unwrap();
    let root = log.try_create(None).unwrap();
    let merged = log.try_merge(&[&root, &root], None).unwrap();
    assert_eq!(
        log.get(&merged).unwrap().parent_ids,
        vec![root.clone(), root.clone()]
    );
    let before = snapshot(&log);
    assert!(log.try_derive(&root, None).is_err());
    assert_eq!(snapshot(&log), before);
    let third = log.try_create(None).unwrap();
    assert_eq!(third, "cv1.0000000000000003");
    let before = snapshot(&log);
    assert!(log.try_create(None).is_err());
    assert_eq!(snapshot(&log), before);
    log.contribute(&root, "pass", "changed", HashMap::new())
        .unwrap();
    let before = snapshot(&log);
    assert!(log
        .contribute(&third, "pass", "changed", HashMap::new())
        .is_err());
    assert!(log
        .try_delete(&third, "pass", "removed", HashMap::new())
        .is_err());
    assert_eq!(snapshot(&log), before);
}

#[test]
fn checked_import_rejects_cycles_dangling_edges_and_disabled_history_gaps() {
    let limits = GraphLimits::default();
    let mut raw = CVLog::new_compact(true);
    let root = raw.create(None);
    let child = raw.derive(&root, None);
    let valid = snapshot(&raw);
    let checked = CVLog::from_checked_json(&valid.to_string(), limits.clone()).unwrap();
    assert_eq!(checked.try_lineage(&child).unwrap().len(), 2);
    let mut cycle = valid.clone();
    cycle["entries"][&root]["parent_ids"] = serde_json::json!([child]);
    assert!(CVLog::from_checked_json(&cycle.to_string(), limits.clone()).is_err());
    let mut dangling = valid.clone();
    dangling["entries"].as_object_mut().unwrap().remove(&root);
    assert!(CVLog::from_checked_json(&dangling.to_string(), limits.clone()).is_err());
    let mut gap = valid;
    gap["identity"]["last_sequence"] = "0000000000000003".into();
    assert!(CVLog::from_checked_json(&gap.to_string(), limits.clone()).is_err());
    let mut disabled = CVLog::new_compact(false);
    disabled.create(None);
    let text = disabled.to_json_string().unwrap();
    assert!(
        CVLog::from_json_string(&text).is_ok(),
        "allocator-only reload remains supported"
    );
    assert!(CVLog::from_checked_json(&text, limits).is_err());
}
