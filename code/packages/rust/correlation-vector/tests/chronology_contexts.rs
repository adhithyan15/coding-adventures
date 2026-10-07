//! Scopes purchase their terminal record before a callback starts. A rejected
//! intervening graph mutation cannot steal that capacity; nesting restores the
//! actual caller context even when a child returns an error that is caught.
use coding_adventures_correlation_vector::{
    CVLog, GraphLimits, JournalPolicy, PassOutcome, PipelineOutcome, ScopeError,
};
use serde_json::Value;
use std::collections::HashMap;

fn new(cap: usize) -> CVLog {
    CVLog::new_checked_chronology(GraphLimits {
        max_events: cap,
        ..Default::default()
    })
    .unwrap()
}
fn accepted_pipeline(log: &mut CVLog, root: &str) -> Result<(), ScopeError<String>> {
    log.with_pipeline(|cv| {
        cv.record_schedule(&[("same", JournalPolicy::OneShot)], 1)
            .map_err(|e| (e, PipelineOutcome::RecordingFailure))?;
        cv.with_pass(0, 0, |cv| {
            cv.contribute(root, "same", "accepted", HashMap::new())
                .map_err(|e| (e, PassOutcome::AcceptanceFailure))?;
            Ok(((), PassOutcome::Accepted { changed: false }))
        })
        .map_err(|e| (format!("{e:?}"), PipelineOutcome::AcceptanceFailure))?;
        Ok(((), PipelineOutcome::Converged))
    })
}

#[test]
fn exact_reserved_end_capacity_survives_rejected_callback_mutation() {
    let mut log = new(7);
    let root = log.try_create(None).unwrap();
    log.with_pipeline(|cv| {
        cv.record_schedule(&[("same", JournalPolicy::OneShot)], 1)
            .unwrap();
        cv.with_pass(0, 0, |cv| {
            // Seven charges already buy all six records plus one descriptor.
            let before = cv.journal().unwrap().last_sequence();
            assert!(cv
                .contribute(&root, "same", "rejected", HashMap::new())
                .is_err());
            assert_eq!(cv.journal().unwrap().last_sequence(), before);
            assert!(
                cv.to_json_string().is_err(),
                "an active scope cannot export full evidence"
            );
            Ok::<_, (String, PassOutcome)>(((), PassOutcome::Accepted { changed: false }))
        })
        .unwrap();
        Ok::<_, (String, PipelineOutcome)>(((), PipelineOutcome::Converged))
    })
    .unwrap();
    assert_eq!(log.journal().unwrap().events().len(), 6);
    assert!(log.get(&root).unwrap().contributions.is_empty());
    log.validate_graph().unwrap();
    let encoded = log.to_json_string().unwrap();
    let restored = CVLog::from_checked_json(
        &encoded,
        GraphLimits {
            max_events: 7,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(restored.to_json_string().unwrap(), encoded);
}

#[test]
fn begin_rejection_does_not_invoke_callback_or_consume_a_record() {
    let mut log = new(1);
    let before = log.to_json_string().unwrap();
    let mut invoked = false;
    let result = log.with_pipeline(|_| {
        invoked = true;
        Ok::<_, (String, PipelineOutcome)>(((), PipelineOutcome::Converged))
    });
    assert!(matches!(result, Err(ScopeError::Recording(_))));
    assert!(!invoked);
    assert_eq!(log.to_json_string().unwrap(), before);
}

#[test]
fn nested_caught_error_then_retry_restores_outer_pass_context() {
    let mut log = new(64);
    let root = log.try_create(None).unwrap();
    log.with_pipeline(|cv| {
        cv.record_schedule(&[("same", JournalPolicy::OneShot)], 1)
            .unwrap();
        cv.with_pass(0, 0, |cv| {
            let outer = cv.journal().unwrap().last_sequence();
            let failed = cv.with_pipeline(|inner| {
                inner
                    .record_schedule(&[("same", JournalPolicy::OneShot)], 1)
                    .unwrap();
                let failure = inner.with_pass(0, 0, |_| {
                    Err::<((), PassOutcome), _>((
                        "original".to_owned(),
                        PassOutcome::CallbackFailure,
                    ))
                });
                assert!(matches!(failure, Err(ScopeError::Callback(ref e)) if e == "original"));
                Err::<((), PipelineOutcome), _>((
                    "inner".to_owned(),
                    PipelineOutcome::CallbackFailure,
                ))
            });
            assert!(matches!(failed, Err(ScopeError::Callback(ref e)) if e == "inner"));
            cv.contribute(&root, "same", "after-failure", HashMap::new())
                .unwrap();
            assert_eq!(
                cv.journal().unwrap().events().last().unwrap().context(),
                Some(outer)
            );
            accepted_pipeline(cv, &root).unwrap();
            cv.contribute(&root, "same", "after-retry", HashMap::new())
                .unwrap();
            assert_eq!(
                cv.journal().unwrap().events().last().unwrap().context(),
                Some(outer)
            );
            Ok::<_, (String, PassOutcome)>(((), PassOutcome::Accepted { changed: false }))
        })
        .unwrap();
        Ok::<_, (String, PipelineOutcome)>(((), PipelineOutcome::Converged))
    })
    .unwrap();
    log.contribute(&root, "outside", "unscoped", HashMap::new())
        .unwrap();
    assert_eq!(
        log.journal().unwrap().events().last().unwrap().context(),
        None
    );
    let encoded = log.to_json_string().unwrap();
    for imported in [
        CVLog::from_json_string(&encoded).unwrap(),
        CVLog::from_checked_json(&encoded, GraphLimits::default()).unwrap(),
    ] {
        assert_eq!(imported.to_json_string().unwrap(), encoded);
    }
    let wire: Value = serde_json::from_str(&encoded).unwrap();
    let failed = wire["journal"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["event"]["outcome"]["kind"] == "callback_failure")
        .count();
    assert_eq!(
        failed, 2,
        "failed child remains failed after a successful enclosing run"
    );
}

#[test]
fn absent_chronology_preserves_callbacks_without_context_allocation() {
    for enabled in [true, false] {
        let mut log = CVLog::new(enabled);
        log.with_pipeline(|cv| {
            cv.record_schedule(&[("same", JournalPolicy::OneShot)], 1)
                .unwrap();
            cv.with_pass(0, 0, |_| {
                Ok::<_, (String, PassOutcome)>((17, PassOutcome::Accepted { changed: true }))
            })
            .unwrap();
            Ok::<_, (String, PipelineOutcome)>(((), PipelineOutcome::Converged))
        })
        .unwrap();
        assert!(log.journal().is_none());
    }
}

#[test]
fn schedule_name_cannot_relabel_retained_pass_contributions() {
    let mut log = new(64); let root = log.try_create(None).unwrap();
    accepted_pipeline(&mut log, &root).unwrap();
    let mut wire: Value = serde_json::from_str(&log.to_json_string().unwrap()).unwrap();
    let schedule = wire["journal"]["events"].as_array_mut().unwrap().iter_mut()
        .find(|r| r["event"]["kind"] == "schedule").unwrap();
    schedule["event"]["passes"][0]["name"] = Value::String("forged".into());
    let encoded = serde_json::to_string(&wire).unwrap();
    assert!(CVLog::from_checked_json(&encoded, GraphLimits::default()).is_err());
    assert!(CVLog::from_json_string(&encoded).is_err());
}

#[test]
fn wrong_active_pass_source_rejects_before_graph_or_journal_mutation() {
    let mut log = new(64); let root = log.try_create(None).unwrap();
    log.with_pipeline(|cv| {
        cv.record_schedule(&[("same", JournalPolicy::OneShot)], 1).unwrap();
        cv.with_pass(0, 0, |cv| {
            let sequence = cv.journal().unwrap().last_sequence();
            assert!(cv.contribute(&root, "forged", "wrong", HashMap::new()).is_err());
            assert!(cv.try_delete(&root, "forged", "wrong", HashMap::new()).is_err());
            assert_eq!(cv.journal().unwrap().last_sequence(), sequence);
            Ok::<_, (String, PassOutcome)>(((), PassOutcome::Accepted { changed: false }))
        }).unwrap();
        Ok::<_, (String, PipelineOutcome)>(((), PipelineOutcome::Converged))
    }).unwrap();
    log.validate_graph().unwrap();
    assert!(log.get(&root).unwrap().contributions.is_empty());
    assert!(log.get(&root).unwrap().deleted.is_none());
}
