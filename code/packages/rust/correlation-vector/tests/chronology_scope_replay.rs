//! Full imported journals rebuild schedule progress; balanced contexts alone
//! cannot establish which sweeps or pass positions actually ran.
use coding_adventures_correlation_vector::{
    CVLog, GraphLimits, JournalPolicy, PassOutcome, PipelineOutcome,
};
use serde_json::{json, Value};
use std::collections::HashMap;

fn snapshot() -> Value {
    let mut log = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    let root = log.try_create(None).unwrap();
    log.with_pipeline(|cv| {
        cv.record_schedule(
            &[
                ("first", JournalPolicy::FixedPoint),
                ("second", JournalPolicy::OneShot),
            ],
            2,
        )
        .unwrap();
        for (slot, name) in ["first", "second"].iter().enumerate() {
            cv.with_pass(0, slot as u64, |cv| {
                cv.contribute(&root, name, "observed", HashMap::new())
                    .unwrap();
                Ok::<_, (String, PassOutcome)>(((), PassOutcome::Accepted { changed: false }))
            })
            .unwrap();
        }
        Ok::<_, (String, PipelineOutcome)>(((), PipelineOutcome::Converged))
    })
    .unwrap();
    serde_json::from_str(&log.to_json_string().unwrap()).unwrap()
}
fn rejects(wire: &Value) {
    let text = serde_json::to_string(wire).unwrap();
    assert!(
        CVLog::from_checked_json(&text, GraphLimits::default()).is_err(),
        "checked accepted {text}"
    );
    assert!(
        CVLog::from_json_string(&text).is_err(),
        "compatibility accepted {text}"
    );
}
#[test]
fn closed_or_non_top_context_and_wrong_end_reference_reject() {
    for (record, value) in [
        (4, "0000000000000002"),
        (7, "0000000000000004"),
        (5, "0000000000000002"),
    ] {
        let mut wire = snapshot();
        wire["journal"]["events"][record]["context"] = json!(value);
        rejects(&wire);
    }
    let mut wire = snapshot();
    wire["journal"]["events"][5]["event"]["begin"] = json!("0000000000000002");
    rejects(&wire);
}
#[test]
fn missing_repeated_and_impossible_schedule_positions_reject() {
    for (record, field, value) in [
        (6, "slot", "0000000000000000"),
        (3, "slot", "0000000000000001"),
        (3, "sweep", "0000000000000001"),
        (3, "slot", "ffffffffffffffff"),
    ] {
        let mut wire = snapshot();
        wire["journal"]["events"][record]["event"]["scope"][field] = json!(value);
        rejects(&wire);
    }
    let mut truncated = snapshot();
    truncated["journal"]["events"][2]["event"]["passes"]
        .as_array_mut()
        .unwrap()
        .pop();
    rejects(&truncated);
    let mut empty = snapshot();
    empty["journal"]["events"][2]["event"]["passes"] = json!([]);
    rejects(&empty);
    let mut zero_cap = snapshot();
    zero_cap["journal"]["events"][2]["event"]["sweep_cap"] = json!("0000000000000000");
    rejects(&zero_cap);
}
#[test]
fn false_convergence_cap_and_cross_kind_outcomes_reject() {
    let mut changed = snapshot();
    changed["journal"]["events"][5]["event"]["outcome"]["changed"] = json!(true);
    rejects(&changed);
    for (record, kind) in [
        (9, "cap"),
        (5, "converged"),
        (9, "accepted"),
        (5, "recording_failure"),
        (9, "scheduling_failure"),
    ] {
        let mut wire = snapshot();
        wire["journal"]["events"][record]["event"]["outcome"] = json!({"kind":kind});
        rejects(&wire);
    }
    let mut unclosed = snapshot();
    unclosed["journal"]["events"].as_array_mut().unwrap().pop();
    unclosed["journal"]["last_sequence"] = json!("0000000000000009");
    rejects(&unclosed);
}
#[test]
fn closed_schedule_scope_and_outcome_schemas_reject_extra_or_duplicate_claims() {
    let mut name = snapshot();
    name["journal"]["events"][2]["event"]["passes"][1]["name"] = json!("first");
    rejects(&name);
    let mut policy = snapshot();
    policy["journal"]["events"][2]["event"]["passes"][0]["policy"] = json!("invented");
    rejects(&policy);
    let mut scope = snapshot();
    scope["journal"]["events"][3]["event"]["scope"]["name"] = json!("forged");
    rejects(&scope);
    let mut outcome = snapshot();
    outcome["journal"]["events"][9]["event"]["outcome"]["changed"] = json!(false);
    rejects(&outcome);
    let mut duplicate = snapshot();
    let mut record = duplicate["journal"]["events"][2].clone();
    record["sequence"] = json!("0000000000000004");
    duplicate["journal"]["events"]
        .as_array_mut()
        .unwrap()
        .insert(3, record);
    duplicate["journal"]["last_sequence"] = json!("000000000000000b");
    let error = CVLog::from_checked_json(&duplicate.to_string(), GraphLimits::default())
        .err()
        .unwrap();
    assert!(error.contains("duplicate CV pipeline schedule"), "{error}");
    rejects(&duplicate);
}
#[test]
fn schedule_descriptors_and_names_are_charged_on_full_reload() {
    let text = snapshot().to_string();
    // C2 + J10 + S2 = 14; counting only journal records would undercharge.
    assert!(CVLog::from_checked_json(
        &text,
        GraphLimits {
            max_events: 13,
            ..Default::default()
        }
    )
    .is_err());
    CVLog::from_checked_json(
        &text,
        GraphLimits {
            max_events: 14,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(CVLog::from_checked_json(
        &text,
        GraphLimits {
            max_metadata_bytes: 0,
            ..Default::default()
        }
    )
    .is_err());
}
