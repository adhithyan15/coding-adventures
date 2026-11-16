//! Adversarial and hostile input tests enforcing CWE-400 / CWE-190 / CWE-20 boundaries.

use percussion_kit::{
    apply_hihat_choke, get_drum_kit, get_percussion_voice, render_percussion_hit,
    render_percussion_sequence, safe_round_sample, HitRenderOptions, PercussionEnvelope,
    PercussionError, PercussionHitEvent, PercussionMode, PercussionNoiseProfile,
    PercussionSignal, PercussionVoiceProfile, SequenceRenderOptions,
    MAX_DURATION_SECONDS, MAX_HITS, MAX_PERCUSSION_MODES, MAX_SAMPLE_COUNT, MAX_SAMPLE_RATE_HZ,
};

#[test]
fn test_cwe400_mode_count_upper_bound() {
    let mode = PercussionMode::new(100.0, 1.0, 0.1).unwrap();
    let env = PercussionEnvelope::new(0.0, 0.01, 0.1).unwrap();

    // 33 modes rejected by PercussionVoiceProfile
    let too_many_modes = vec![mode; MAX_PERCUSSION_MODES + 1];
    let err = PercussionVoiceProfile::new(
        "test",
        "Test",
        "misc",
        0.5,
        too_many_modes,
        None,
        env,
        0.0,
        None,
    )
    .unwrap_err();
    assert!(matches!(err, PercussionError::ModeCountTooLarge { .. }));

    // render_percussion_hit rejects max_mode_count > MAX_PERCUSSION_MODES
    let opts = HitRenderOptions::new().max_mode_count(MAX_PERCUSSION_MODES + 1);
    let err = render_percussion_hit("kick", 0.05, &opts).unwrap_err();
    assert!(matches!(err, PercussionError::ModeCountTooLarge { .. }));

    // render_percussion_sequence rejects max_mode_count > MAX_PERCUSSION_MODES
    let ev = vec![PercussionHitEvent::hit("kick", 0.0).unwrap()];
    let s_opts = SequenceRenderOptions::new().max_mode_count(MAX_PERCUSSION_MODES + 1);
    let err = render_percussion_sequence(&ev, &s_opts).unwrap_err();
    assert!(matches!(err, PercussionError::ModeCountTooLarge { .. }));
}

#[test]
fn test_cwe400_duration_upper_bound() {
    let too_long = MAX_DURATION_SECONDS + 0.1;

    // render_percussion_hit rejects duration > MAX_DURATION_SECONDS
    let opts = HitRenderOptions::default();
    let err = render_percussion_hit("kick", too_long, &opts).unwrap_err();
    assert!(matches!(err, PercussionError::DurationTooLong { .. }));

    // PercussionSignal rejects duration > MAX_DURATION_SECONDS
    let voice = get_percussion_voice("kick", None).unwrap();
    let err = PercussionSignal::new(voice, too_long, 1.0, 44100.0).unwrap_err();
    assert!(matches!(err, PercussionError::DurationTooLong { .. }));

    // PercussionHitEvent rejects duration > MAX_DURATION_SECONDS
    let err = PercussionHitEvent::with_duration("kick", 0.0, too_long).unwrap_err();
    assert!(matches!(err, PercussionError::DurationTooLong { .. }));

    // PercussionMode decay > MAX_DURATION_SECONDS
    let err = PercussionMode::new(100.0, 1.0, too_long).unwrap_err();
    assert!(matches!(err, PercussionError::DurationTooLong { .. }));

    // PercussionEnvelope durations > MAX_DURATION_SECONDS
    assert!(PercussionEnvelope::new(too_long, 0.0, 0.1).is_err());
    assert!(PercussionEnvelope::new(0.0, too_long, 0.1).is_err());
    assert!(PercussionEnvelope::new(0.0, 0.0, too_long).is_err());
}

#[test]
fn test_cwe400_sample_count_upper_bound() {
    let opts = HitRenderOptions::new().max_sample_count(MAX_SAMPLE_COUNT + 1);
    let err = render_percussion_hit("kick", 0.05, &opts).unwrap_err();
    assert!(matches!(err, PercussionError::SampleCountTooLarge { .. }));

    let s_opts = SequenceRenderOptions::new().max_sample_count(MAX_SAMPLE_COUNT + 1);
    let err = render_percussion_sequence(&[], &s_opts).unwrap_err();
    assert!(matches!(err, PercussionError::SampleCountTooLarge { .. }));

    let huge_samples = vec![1.0; 10];
    let err = apply_hihat_choke(
        &huge_samples,
        0.0,
        0.01,
        8000.0,
        MAX_DURATION_SECONDS + 1.0,
    )
    .unwrap_err();
    assert!(matches!(err, PercussionError::DurationTooLong { .. }));
}

#[test]
fn test_cwe400_sequence_events_upper_bound() {
    let s_opts = SequenceRenderOptions::new().max_sequence_events(MAX_HITS + 1);
    let err = render_percussion_sequence(&[], &s_opts).unwrap_err();
    assert!(matches!(err, PercussionError::SequenceEventsTooLarge { .. }));

    let evs: Vec<PercussionHitEvent> = (0..10)
        .map(|_| PercussionHitEvent::hit("kick", 0.0).unwrap())
        .collect();
    let s_opts2 = SequenceRenderOptions::new().max_sequence_events(5);
    let err = render_percussion_sequence(&evs, &s_opts2).unwrap_err();
    assert!(matches!(err, PercussionError::SequenceEventsTooLarge { .. }));
}

#[test]
fn test_cwe400_unbounded_start_seconds_rejected_pre_flight() {
    // start_seconds = 1e12 would require gigabytes of timeline: rejected pre-flight!
    let ev = vec![PercussionHitEvent::hit("kick", 1e12).unwrap()];
    let s_opts = SequenceRenderOptions::new().sample_rate_hz(44100.0);
    let err = render_percussion_sequence(&ev, &s_opts).unwrap_err();
    assert!(matches!(err, PercussionError::SampleCountTooLarge { .. }));
}

#[test]
fn test_cwe20_unconditional_choke_ramp_validation() {
    // Validated unconditionally, even on empty sequences
    let s_opts_neg = SequenceRenderOptions::new().choke_ramp_seconds(-1.0);
    assert!(matches!(
        render_percussion_sequence(&[], &s_opts_neg).unwrap_err(),
        PercussionError::NegativeNumber { .. }
    ));

    let s_opts_nan = SequenceRenderOptions::new().choke_ramp_seconds(f64::NAN);
    assert!(matches!(
        render_percussion_sequence(&[], &s_opts_nan).unwrap_err(),
        PercussionError::NonFiniteNumber { .. }
    ));

    let s_opts_inf = SequenceRenderOptions::new().choke_ramp_seconds(f64::INFINITY);
    assert!(matches!(
        render_percussion_sequence(&[], &s_opts_inf).unwrap_err(),
        PercussionError::NonFiniteNumber { .. }
    ));

    let s_opts_toolong = SequenceRenderOptions::new().choke_ramp_seconds(MAX_DURATION_SECONDS + 1.0);
    assert!(matches!(
        render_percussion_sequence(&[], &s_opts_toolong).unwrap_err(),
        PercussionError::DurationTooLong { .. }
    ));
}

#[test]
fn test_cwe400_max_sample_rate_hz_enforced_everywhere() {
    let too_high = MAX_SAMPLE_RATE_HZ + 1.0;

    let opts = HitRenderOptions::new().sample_rate_hz(too_high);
    assert!(matches!(
        render_percussion_hit("kick", 0.05, &opts).unwrap_err(),
        PercussionError::SampleRateTooHigh { .. }
    ));

    let s_opts = SequenceRenderOptions::new().sample_rate_hz(too_high);
    assert!(matches!(
        render_percussion_sequence(&[], &s_opts).unwrap_err(),
        PercussionError::SampleRateTooHigh { .. }
    ));

    assert!(matches!(
        apply_hihat_choke(&[1.0], 0.0, 0.01, too_high, 0.015).unwrap_err(),
        PercussionError::SampleRateTooHigh { .. }
    ));

    let voice = get_percussion_voice("kick", None).unwrap();
    assert!(matches!(
        PercussionSignal::new(voice, 0.05, 1.0, too_high).unwrap_err(),
        PercussionError::SampleRateTooHigh { .. }
    ));
}

#[test]
fn test_cwe20_hostile_numeric_inputs_fail_closed() {
    let bad_values = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0];

    for &bad in &bad_values {
        // duration
        let opts = HitRenderOptions::default();
        assert!(render_percussion_hit("kick", bad, &opts).is_err());

        // amplitude
        let opts_amp = HitRenderOptions::new().amplitude(bad);
        assert!(render_percussion_hit("kick", 0.05, &opts_amp).is_err());

        // sample_rate
        let opts_rate = HitRenderOptions::new().sample_rate_hz(bad);
        assert!(render_percussion_hit("kick", 0.05, &opts_rate).is_err());

        // event
        assert!(PercussionHitEvent::new("kick", bad, 0.05, 1.0, "drums", None).is_err());
        assert!(PercussionHitEvent::new("kick", 0.0, bad, 1.0, "drums", None).is_err());
        assert!(PercussionHitEvent::new("kick", 0.0, 0.05, bad, "drums", None).is_err());

        // mode
        assert!(PercussionMode::new(bad, 1.0, 0.1).is_err());
        assert!(PercussionMode::new(100.0, bad, 0.1).is_err());
        assert!(PercussionMode::new(100.0, 1.0, bad).is_err());

        // noise
        assert!(PercussionNoiseProfile::new(bad, 0.1, 1).is_err());
        assert!(PercussionNoiseProfile::new(1.0, bad, 1).is_err());

        // envelope
        assert!(PercussionEnvelope::new(bad, 0.0, 0.1).is_err());
        assert!(PercussionEnvelope::new(0.0, bad, 0.1).is_err());
        assert!(PercussionEnvelope::new(0.0, 0.0, bad).is_err());
    }
}

#[test]
fn test_cwe20_unit_float_boundaries() {
    let opts_low = HitRenderOptions::new().amplitude(-0.01);
    assert!(matches!(
        render_percussion_hit("kick", 0.05, &opts_low).unwrap_err(),
        PercussionError::UnitFloatOutOfRange { .. }
    ));

    let opts_high = HitRenderOptions::new().amplitude(1.01);
    assert!(matches!(
        render_percussion_hit("kick", 0.05, &opts_high).unwrap_err(),
        PercussionError::UnitFloatOutOfRange { .. }
    ));

    assert!(matches!(
        PercussionHitEvent::new("kick", 0.0, 0.05, -0.01, "drums", None).unwrap_err(),
        PercussionError::UnitFloatOutOfRange { .. }
    ));

    assert!(matches!(
        PercussionHitEvent::new("kick", 0.0, 0.05, 1.01, "drums", None).unwrap_err(),
        PercussionError::UnitFloatOutOfRange { .. }
    ));
}

#[test]
fn test_cwe20_empty_and_unknown_identifiers_fail_closed() {
    // Empty hit id
    assert!(matches!(
        PercussionHitEvent::new("", 0.0, 0.05, 1.0, "drums", None).unwrap_err(),
        PercussionError::EmptyIdentifier { .. }
    ));

    // Empty track id
    assert!(matches!(
        PercussionHitEvent::new("kick", 0.0, 0.05, 1.0, "", None).unwrap_err(),
        PercussionError::EmptyIdentifier { .. }
    ));

    // Unknown voice
    let opts = HitRenderOptions::default();
    assert!(matches!(
        render_percussion_hit("nonexistent_instrument", 0.05, &opts).unwrap_err(),
        PercussionError::UnknownVoice { .. }
    ));

    // Unknown kit
    assert!(matches!(
        get_drum_kit("jazz_kit").unwrap_err(),
        PercussionError::UnknownKit { .. }
    ));
}

#[test]
fn test_cwe190_safe_round_sample_overflow_protection() {
    assert!(safe_round_sample(f64::NAN, 44100.0).is_err());
    assert!(safe_round_sample(1.0, f64::NAN).is_err());
    assert!(safe_round_sample(1e308, 192000.0).is_err());
    assert_eq!(safe_round_sample(0.5, 44100.0).unwrap(), 22050);
}
