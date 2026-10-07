use super::*;

#[test]
fn sequence_admission_protects_all_future_terminal_reservations() {
    let mut log = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    log.journal.as_mut().unwrap().last_sequence = u64::MAX - 1;
    let before = (
        log.entries.len(),
        log.compact_sequence,
        log.checked.as_ref().unwrap().usage,
    );
    let mut entered = false;
    assert!(matches!(
        log.with_pipeline(|_| {
            entered = true;
            Ok::<_, ((), PipelineOutcome)>(((), PipelineOutcome::Converged))
        }),
        Err(ScopeError::Recording(_))
    ));
    assert!(!entered);
    assert_eq!(
        before,
        (
            log.entries.len(),
            log.compact_sequence,
            log.checked.as_ref().unwrap().usage
        )
    );
    assert_eq!(log.journal().unwrap().last_sequence(), u64::MAX - 1);
    assert!(log.journal().unwrap().events().is_empty());
    assert!(log.journal.as_ref().unwrap().active.is_empty());
    log.journal.as_mut().unwrap().last_sequence = u64::MAX;
    assert!(log.try_create(None).is_err());
    assert_eq!(
        before,
        (
            log.entries.len(),
            log.compact_sequence,
            log.checked.as_ref().unwrap().usage
        )
    );
}

#[test]
fn prior_probe_work_cannot_restart_at_the_checked_parser_boundary() {
    let text = CVLog::new_checked_chronology(GraphLimits::default())
        .unwrap()
        .to_json_string()
        .unwrap();
    let limits = GraphLimits {
        max_work: 1000,
        ..Default::default()
    };
    CVLog::from_checked_json(&text, limits.clone()).unwrap();
    let mut prior = Work::new(limits.max_work);
    prior.take(limits.max_work - 1).unwrap();
    let error = CVLog::from_checked_json_with_work(&text, limits, prior)
        .err()
        .unwrap();
    assert!(error.contains("operation work limit"), "{error}");
}

#[test]
fn journal_and_descriptor_charges_precede_malformed_array_bodies() {
    let raw = r#"{"enabled":true,"entries":{},"pass_order":[],"identity":{"scheme":"compact-v1","last_sequence":"0000000000000000"},"journal":{"version":"chronology-v1","coverage":"full","last_sequence":"0000000000000001","events":[{"\uZZZZ":false}]}}"#;
    let error = CVLog::from_checked_json(
        raw,
        GraphLimits {
            max_events: 0,
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("events limit"), "{error}");
    let raw = r#"{"enabled":true,"entries":{},"pass_order":[],"identity":{"scheme":"compact-v1","last_sequence":"0000000000000000"},"journal":{"version":"chronology-v1","coverage":"full","last_sequence":"0000000000000001","events":[{"sequence":"0000000000000001","context":null,"event":{"kind":"schedule","sweep_cap":"0000000000000001","passes":[{"\uZZZZ":false}]}}]}}"#;
    let error = CVLog::from_checked_json(
        raw,
        GraphLimits {
            max_events: 1,
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("events limit"), "{error}");
}

#[test]
fn names_use_encoded_byte_charges_and_failed_schedule_is_atomic() {
    for cap in [5, 6] {
        let limits = GraphLimits {
            max_metadata_bytes: cap,
            ..Default::default()
        };
        let mut log = CVLog::new_checked_chronology(limits.clone()).unwrap();
        let result = log.with_pipeline(|cv| {
            cv.record_schedule(&[("same", JournalPolicy::OneShot)], 1)
                .map_err(|e| (e, PipelineOutcome::RecordingFailure))?;
            cv.with_pass(0, 0, |_| {
                Ok::<_, (String, PassOutcome)>(((), PassOutcome::Accepted { changed: false }))
            })
            .unwrap();
            Ok(((), PipelineOutcome::Converged))
        });
        assert_eq!(result.is_ok(), cap == 6);
        assert_eq!(
            log.checked.as_ref().unwrap().usage.bytes,
            if cap == 6 { 6 } else { 0 }
        );
        let text = log.to_json_string().unwrap();
        assert_eq!(
            CVLog::from_checked_json(&text, limits)
                .unwrap()
                .to_json_string()
                .unwrap(),
            text
        );
    }
}

#[test]
fn caught_panic_leaves_explicitly_unexportable_active_context() {
    let mut log = CVLog::new_checked_chronology(GraphLimits::default()).unwrap();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = log.with_pipeline::<(), ()>(|_| panic!("unsupported callback panic"));
    }))
    .is_err());
    assert!(log.to_json_string().is_err());
}
