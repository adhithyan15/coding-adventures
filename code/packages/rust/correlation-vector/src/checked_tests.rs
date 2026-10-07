use super::*;

#[test]
fn checked_import_charges_key_work_before_decoding_the_key() {
    // Root consumes the entire allowance. Entering the key decoder would
    // report the malformed escape; budget exhaustion must stop it first.
    let error = CVLog::from_checked_json(
        r#"{"\uZZZZ":false}"#,
        GraphLimits {
            max_work: 1,
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("operation work limit"), "{error}");
}

#[test]
fn allocator_import_cannot_launder_lost_metadata_or_defaulted_fields() {
    let duplicate = r#"{"enabled":true,"entries":{"cv1.0000000000000001":{"id":"cv1.0000000000000001","parent_ids":[],"origin":{"source":"s","location":"l","timestamp":null,"meta":{"x":1,"\u0078":2}},"contributions":[],"deleted":null}},"identity":{"scheme":"compact-v1","last_sequence":"0000000000000001"},"pass_order":[]}"#;
    let missing = r#"{"enabled":true,"entries":{"cv1.0000000000000001":{"id":"cv1.0000000000000001"}},"identity":{"scheme":"compact-v1","last_sequence":"0000000000000001"},"pass_order":[]}"#;
    for raw in [duplicate, missing] {
        assert!(CVLog::from_checked_json(raw, GraphLimits::default()).is_err());
        let mut unchecked = CVLog::from_json_string(raw).unwrap();
        assert!(unchecked.try_lineage("cv1.0000000000000001").is_err());
        assert!(unchecked.validate_graph().is_err());
        unchecked.try_create(None).unwrap();
        let normalized = unchecked.to_json_string().unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&normalized).unwrap()["unchecked_import"],
            true
        );
        assert!(CVLog::from_checked_json(&normalized, GraphLimits::default()).is_err());
        let reloaded = CVLog::from_json_string(&normalized).unwrap();
        assert!(reloaded.try_lineage("cv1.0000000000000002").is_err());
        assert!(CVLog::from_checked_json(
            &reloaded.to_json_string().unwrap(),
            GraphLimits::default()
        )
        .is_err());
    }
}

#[test]
fn compatibility_import_cannot_discard_projection_or_stage_evidence() {
    let mut legacy = CVLog::new(true);
    let id = legacy.try_create(None).unwrap();
    let mut snapshot: Value = serde_json::from_str(&legacy.to_json_string().unwrap()).unwrap();
    for view in [
        serde_json::json!({"complete":false,"filtered":true}),
        Value::Null,
    ] {
        snapshot["view"] = view;
        assert!(CVLog::from_json_string(&snapshot.to_string()).is_err());
    }
    for mut log in [CVLog::new(true), CVLog::new_compact(true)] {
        let root = log.try_create(None).unwrap();
        log.contribute(&root, "pass", "changed", HashMap::new())
            .unwrap();
        let mut snapshot: Value = serde_json::from_str(&log.to_json_string().unwrap()).unwrap();
        snapshot["pass_order"] = serde_json::json!([]);
        let incomplete = CVLog::from_json_string(&snapshot.to_string()).unwrap();
        assert!(incomplete.try_lineage(&root).is_err());
        assert!(incomplete.try_ancestors(&root).is_err());
        assert!(incomplete.try_descendants(&root).is_err());
    }
    legacy
        .try_delete(&id, "dce", "removed", HashMap::new())
        .unwrap();
    let text = legacy.to_json_string().unwrap();
    let incomplete = CVLog::from_json_string(&text).unwrap();
    assert!(
        incomplete.try_lineage(&id).is_err(),
        "historical undeclared deletion stage is not complete evidence"
    );
}

#[test]
fn checked_queries_reject_disabled_and_gapped_compatibility_recording() {
    let mut partial = CVLog::new_compact(false);
    partial.try_create(None).unwrap(); // allocated but never recorded
    partial.set_enabled(true).unwrap();
    let recorded = partial.try_create(None).unwrap();
    let text = partial.to_json_string().unwrap(); // allocator-only export remains valid
    let imported = CVLog::from_json_string(&text).unwrap();
    assert!(imported.try_lineage(&recorded).is_err());
    assert!(imported.try_ancestors(&recorded).is_err());
    assert!(imported.try_descendants(&recorded).is_err());
    for mut disabled in [CVLog::new(true), CVLog::new_compact(true)] {
        let id = disabled.try_create(None).unwrap();
        disabled.set_enabled(false).unwrap();
        assert!(disabled.try_lineage(&id).is_err());
        assert!(disabled.try_ancestors(&id).is_err());
        assert!(disabled.try_descendants(&id).is_err());
    }
}

#[test]
fn checked_import_counts_malformed_items_before_reading_their_bodies() {
    let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    let id = log.try_create(None).unwrap();
    let mut snapshot: Value = serde_json::from_str(&log.to_json_string().unwrap()).unwrap();
    snapshot["entries"][&id]["contributions"] = serde_json::json!([null]);
    let error = CVLog::from_checked_json(
        &snapshot.to_string(),
        GraphLimits {
            max_events: 0,
            ..GraphLimits::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("events limit"), "{error}");
    snapshot["entries"][&id]["contributions"] = serde_json::json!([]);
    snapshot["entries"][&id]["parent_ids"] = serde_json::json!([null]);
    let error = CVLog::from_checked_json(
        &snapshot.to_string(),
        GraphLimits {
            max_edges: 0,
            ..GraphLimits::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("parent edges limit"), "{error}");
    snapshot["entries"][&id]["parent_ids"] = serde_json::json!([]);
    let error = CVLog::from_checked_json(
        &snapshot.to_string(),
        GraphLimits {
            max_nodes: 0,
            ..GraphLimits::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("nodes limit"), "{error}");
}

#[test]
fn checked_queries_reject_invalid_legacy_graphs_and_key_mismatches() {
    let mut log = CVLog::new(true);
    let root = log.create(None);
    let child = log.derive(&root, None);
    log.entries
        .get_mut(&root)
        .unwrap()
        .parent_ids
        .push(child.clone());
    for id in [&root, &child] {
        assert!(log.try_lineage(id).is_err());
        assert!(log.try_ancestors(id).is_err());
        assert!(log.try_descendants(id).is_err());
    }
    log.entries.get_mut(&root).unwrap().parent_ids.clear();
    log.entries
        .get_mut(&child)
        .unwrap()
        .parent_ids
        .push("unknown".into());
    assert!(
        log.try_lineage(&root).is_err(),
        "validation covers unrelated nodes too"
    );
    log.entries.get_mut(&child).unwrap().parent_ids.pop();
    log.entries.get_mut(&child).unwrap().id = "wrong".into();
    assert!(log.try_descendants(&root).is_err());
}

#[test]
fn checked_export_shares_validation_and_encoding_work() {
    let mut baseline = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    baseline.try_create(None).unwrap();
    // Validation takes four visits; root, entry-map key, entry and identity
    // encoding take four more, even without origin/contribution payloads.
    let expected = baseline.to_json_string().unwrap();
    let mut log = CVLog::new_checked_compact(GraphLimits {
        max_work: 7,
        ..GraphLimits::default()
    })
    .unwrap();
    log.try_create(None).unwrap();
    assert!(log.to_json_string().is_err());
    let mut log = CVLog::new_checked_compact(GraphLimits {
        max_work: 8,
        ..GraphLimits::default()
    })
    .unwrap();
    log.try_create(None).unwrap();
    assert_eq!(log.to_json_string().unwrap(), expected);
}

#[test]
fn checked_metadata_and_input_limits_accept_exact_and_reject_one_beyond() {
    let origin = || {
        Some(Origin {
            source: String::new(),
            location: String::new(),
            timestamp: None,
            meta: HashMap::from([("k".into(), Value::String("λ\n\u{0001}".into()))]),
        })
    };
    // Two quoted empty strings plus {"k":"λ\n\u0001"} encode to 22 bytes.
    let exact = GraphLimits {
        max_metadata_bytes: 22,
        max_metadata_values: 2,
        max_metadata_depth: 2,
        ..GraphLimits::default()
    };
    let mut log = CVLog::new_checked_compact(exact.clone()).unwrap();
    log.try_create(origin()).unwrap();
    log.validate_graph().unwrap();
    let text = log.to_json_string().unwrap();
    let import_exact = GraphLimits {
        max_input_bytes: text.len(),
        ..exact.clone()
    };
    assert!(CVLog::from_checked_json(&text, import_exact).is_ok());
    assert!(CVLog::from_checked_json(
        &text,
        GraphLimits {
            max_input_bytes: text.len() - 1,
            ..exact.clone()
        }
    )
    .is_err());
    for limits in [
        GraphLimits {
            max_metadata_bytes: 21,
            ..exact.clone()
        },
        GraphLimits {
            max_metadata_values: 1,
            ..exact.clone()
        },
        GraphLimits {
            max_metadata_depth: 1,
            ..exact
        },
    ] {
        let mut rejected = CVLog::new_checked_compact(limits.clone()).unwrap();
        assert!(rejected.try_create(origin()).is_err());
        assert_eq!(rejected.compact_sequence, Some(0));
        assert!(rejected.entries().is_empty());
        assert_eq!(
            rejected.checked.as_ref().unwrap().usage,
            checked::Usage::default()
        );
        assert!(CVLog::from_checked_json(&text, limits).is_err());
    }
}

#[test]
fn metadata_depth_has_a_hard_ceiling_and_mutation_work_is_transactional() {
    let origin = |arrays| {
        let mut value = Value::Null;
        for _ in 0..arrays {
            value = Value::Array(vec![value]);
        }
        Some(Origin {
            source: String::new(),
            location: String::new(),
            timestamp: None,
            meta: HashMap::from([("k".into(), value)]),
        })
    };
    let mut log = CVLog::new_checked_compact(GraphLimits {
        max_work: 128,
        ..GraphLimits::default()
    })
    .unwrap();
    log.try_create(origin(62)).unwrap(); // metadata root + 62 arrays + null: depth 64
    let mut limited = CVLog::new_checked_compact(GraphLimits {
        max_work: 127,
        ..GraphLimits::default()
    })
    .unwrap();
    assert!(limited.try_create(origin(62)).is_err());
    assert_eq!(limited.compact_sequence, Some(0));
    assert_eq!(
        limited.checked.as_ref().unwrap().usage,
        checked::Usage::default()
    );
    let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    log.try_create(origin(62)).unwrap();
    let text = log.to_json_string().unwrap();
    assert!(CVLog::from_checked_json(&text, GraphLimits::default()).is_ok());
    assert!(log.try_create(origin(63)).is_err());
    assert_eq!(log.compact_sequence, Some(1));
    assert!(CVLog::new_checked_compact(GraphLimits {
        max_metadata_depth: 65,
        ..GraphLimits::default()
    })
    .is_err());
}

#[test]
fn checked_deletion_is_permanent_and_has_reloadable_stage_evidence() {
    let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    let id = log.try_create(None).unwrap();
    log.try_delete(&id, "dce", "original reason", HashMap::new())
        .unwrap();
    let before = log.to_json_string().unwrap();
    assert_eq!(log.pass_order(), ["dce"]);
    assert!(log
        .try_delete(&id, "replacement", "wrong reason", HashMap::new())
        .is_err());
    assert!(log
        .contribute(&id, "later", "changed", HashMap::new())
        .is_err());
    assert!(log.set_enabled(false).is_err());
    assert_eq!(log.to_json_string().unwrap(), before);
    let mut restored = CVLog::from_checked_json(&before, GraphLimits::default()).unwrap();
    restored.try_derive(&id, None).unwrap();
    restored.validate_graph().unwrap();
}

#[test]
fn resource_accounting_detects_integer_overflow_before_assignment() {
    let mut count = usize::MAX;
    assert!(checked::add_bounded(&mut count, 1, usize::MAX, "test").is_err());
    assert_eq!(count, usize::MAX);
    let mut count = 3;
    assert!(checked::add_bounded(&mut count, 2, 4, "test").is_err());
    assert_eq!(count, 3);
}

#[test]
fn deep_and_wide_queries_are_iterative_deterministic_and_borrow_entries() {
    for size in [4096, 8192] {
        let started = std::time::Instant::now();
        let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let root = log.try_create(None).unwrap();
        let mut last = root.clone();
        for _ in 1..size {
            last = log.try_derive(&last, None).unwrap();
        }
        let lineage = log.try_lineage(&last).unwrap();
        assert_eq!(lineage.len(), size);
        assert!(std::ptr::eq(lineage[0], log.get(&root).unwrap()));
        assert!(std::ptr::eq(lineage[size - 1], log.get(&last).unwrap()));
        for pair in lineage.windows(2) {
            assert_eq!(pair[1].parent_ids, [pair[0].id.clone()]);
        }
        assert_eq!(log.try_ancestors(&last).unwrap().len(), size - 1);
        assert_eq!(log.try_descendants(&root).unwrap().len(), size - 1);
        let text = log.to_json_string().unwrap();
        let restored = CVLog::from_checked_json(&text, GraphLimits::default()).unwrap();
        assert_eq!(restored.try_lineage(&last).unwrap().len(), size);
        println!(
            "checked chain nodes={size} bytes={} elapsed_ms={}",
            text.len(),
            started.elapsed().as_millis()
        );
    }
    let started = std::time::Instant::now();
    let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    let root = log.try_create(None).unwrap();
    let mut children = Vec::new();
    for _ in 0..10_000 {
        children.push(log.try_derive(&root, None).unwrap());
    }
    assert_eq!(log.try_descendants(&root).unwrap(), children);
    let parents: Vec<_> = children.iter().map(String::as_str).collect();
    let merged = log.try_merge(&parents, None).unwrap();
    assert_eq!(log.try_lineage(&merged).unwrap().len(), 10_002);
    log.validate_graph().unwrap();
    println!(
        "checked wide nodes=10002 elapsed_ms={}",
        started.elapsed().as_millis()
    );
}

#[test]
fn rejected_owned_metadata_is_disposed_without_recursive_drop() {
    const CHILD: &str = "CV02_DEEP_OWNED_METADATA_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "checked_tests::rejected_owned_metadata_is_disposed_without_recursive_drop",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "isolated cleanup failed: {:?}\n{}\n{}",
            result.status,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let deep = || {
                let mut value = Value::Null;
                for _ in 0..65_536 {
                    value = Value::Array(vec![value]);
                }
                HashMap::from([("deep".to_string(), value)])
            };
            let origin = || {
                Some(Origin {
                    source: "s".into(),
                    location: "l".into(),
                    timestamp: None,
                    meta: deep(),
                })
            };
            let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
            assert!(log.try_create(origin()).is_err());
            assert!(log.try_derive("missing", origin()).is_err());
            assert!(log.try_merge(&["missing"], origin()).is_err());
            assert!(log.contribute("missing", "s", "t", deep()).is_err());
            assert!(log.try_delete("missing", "s", "r", deep()).is_err());
            assert_eq!(log.entries().len(), 0);
            assert_eq!(log.try_create(None).unwrap(), "cv1.0000000000000001");
            let mut disabled = CVLog::new_compact(false);
            disabled.try_create(origin()).unwrap();
            disabled.try_derive("missing", origin()).unwrap();
            disabled.try_merge(&["missing"], origin()).unwrap();
            disabled.contribute("missing", "s", "t", deep()).unwrap();
            disabled.try_delete("missing", "s", "r", deep()).unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn checked_import_cannot_restart_the_work_allowance_between_phases() {
    let text = CVLog::new_checked_compact(GraphLimits::default())
        .unwrap()
        .to_json_string()
        .unwrap();
    // Empty snapshot: 13 visits for parsing root/identity keys and values.
    // Typed conversion reserves those visits again before taking ownership.
    let too_small = GraphLimits {
        max_work: 25,
        ..GraphLimits::default()
    };
    assert!(CVLog::from_checked_json(&text, too_small).is_err());
    let exact = GraphLimits {
        max_work: 26,
        ..GraphLimits::default()
    };
    assert!(CVLog::from_checked_json(&text, exact).is_ok());
}

#[test]
fn checked_export_enforces_exact_byte_cap_and_validates_full_graph() {
    let baseline = CVLog::new_checked_compact(GraphLimits::default())
        .unwrap()
        .to_json_string()
        .unwrap();
    let exact = CVLog::new_checked_compact(GraphLimits {
        max_output_bytes: baseline.len(),
        ..GraphLimits::default()
    })
    .unwrap();
    assert_eq!(exact.to_json_string().unwrap().len(), baseline.len());
    let too_small = CVLog::new_checked_compact(GraphLimits {
        max_output_bytes: baseline.len() - 1,
        ..GraphLimits::default()
    })
    .unwrap();
    assert!(too_small.to_json_string().is_err());
    let mut invalid = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    let root = invalid.try_create(None).unwrap();
    invalid
        .entries
        .get_mut(&root)
        .unwrap()
        .parent_ids
        .push(root);
    assert!(invalid.to_json_string().is_err());
}

#[test]
fn canonical_export_preserves_nested_metadata_numbers_and_array_order() {
    let make = |reverse: bool| {
        let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
        let pairs = [
            ("zeta", serde_json::json!([u64::MAX, -0.0, "λ\n\u{0001}"])),
            ("alpha", serde_json::json!({"z":2,"a":[3,1]})),
        ];
        let mut meta = HashMap::new();
        let indices = if reverse { [1, 0] } else { [0, 1] };
        for i in indices {
            meta.insert(pairs[i].0.into(), pairs[i].1.clone());
        }
        for _ in 0..8 {
            log.try_create(Some(Origin {
                source: "source".into(),
                location: "1:2".into(),
                timestamp: None,
                meta: meta.clone(),
            }))
            .unwrap();
        }
        log
    };
    let a = make(false);
    let b = make(true);
    let text = a.to_json_string().unwrap();
    assert_eq!(text, b.to_json_string().unwrap());
    let value: Value = serde_json::from_str(&text).unwrap();
    let meta = &value["entries"]["cv1.0000000000000001"]["origin"]["meta"];
    assert_eq!(meta["zeta"][0].as_u64(), Some(u64::MAX));
    assert!(meta["zeta"][1].as_f64().unwrap().is_sign_negative());
    assert_eq!(meta["alpha"]["a"], serde_json::json!([3, 1]));
    assert_eq!(meta["zeta"][2], "λ\n\u{0001}");
    assert!(text.find("\"alpha\"").unwrap() < text.find("\"zeta\"").unwrap());
}

#[test]
fn checked_import_rejects_nested_duplicate_keys_and_unknown_or_missing_fields() {
    let raw = r#"{"enabled":true,"entries":{"cv1.0000000000000001":{"id":"cv1.0000000000000001","parent_ids":[],"origin":{"source":"s","location":"l","timestamp":null,"meta":{"x":1,"\u0078":2}},"contributions":[],"deleted":null}},"identity":{"scheme":"compact-v1","last_sequence":"0000000000000001"},"pass_order":[]}"#;
    let error = CVLog::from_checked_json(raw, GraphLimits::default())
        .err()
        .expect("accepted nested duplicate");
    assert!(error.contains("duplicate"));
    let valid = raw.replace(r#""x":1,"\u0078":2"#, r#""x":1"#);
    assert!(CVLog::from_checked_json(&valid, GraphLimits::default()).is_ok());
    let mut value: Value = serde_json::from_str(&valid).unwrap();
    value["entries"]["cv1.0000000000000001"]
        .as_object_mut()
        .unwrap()
        .remove("parent_ids");
    assert!(CVLog::from_checked_json(&value.to_string(), GraphLimits::default()).is_err());
    let mut value: Value = serde_json::from_str(&valid).unwrap();
    value["unknown"] = true.into();
    assert!(CVLog::from_checked_json(&value.to_string(), GraphLimits::default()).is_err());
}

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
