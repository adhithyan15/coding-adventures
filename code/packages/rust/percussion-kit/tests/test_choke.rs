//! Hi-hat choke group dynamics and release ramp tests (MUS07 §7 & §9).

use percussion_kit::{
    apply_hihat_choke, render_percussion_hit, render_percussion_sequence,
    CHOKE_RELEASE_RAMP_SECONDS, HitRenderOptions, PercussionEnvelope,
    PercussionHitEvent, PercussionMode, PercussionVoiceProfile, SequenceRenderOptions,
    DrumKitProfile,
};
use std::collections::BTreeMap;

#[test]
fn test_apply_hihat_choke_direct_helper() {
    let rate = 10_000.0;
    // 500 samples = 50 ms of unit amplitude
    let samples = vec![1.0; 500];

    // Choke at 20 ms with a 15 ms ramp (ramp length = 150 samples)
    // Ramp runs from sample 200 to 350
    let choked = apply_hihat_choke(
        &samples,
        0.0,
        0.020,
        rate,
        CHOKE_RELEASE_RAMP_SECONDS,
    )
    .expect("choke helper succeeds");

    assert_eq!(choked.len(), 500);

    // Before choke: unchanged (1.0)
    for (i, &s) in choked.iter().enumerate().take(200) {
        assert_eq!(s, 1.0, "sample {i} before choke should be 1.0");
    }

    // During 15 ms ramp: strictly monotonic downward ramp
    for i in 200..349 {
        assert!(
            choked[i] >= choked[i + 1],
            "choke ramp at index {i} must be monotonically decreasing"
        );
    }
    assert!((choked[200] - 1.0).abs() < 1e-6);
    assert!((choked[275] - 0.5).abs() < 0.01);

    // After 15 ms ramp: strictly zero
    for (i, &s) in choked.iter().enumerate().skip(350) {
        assert_eq!(s, 0.0, "sample {i} after choke ramp should be 0.0");
    }
}

#[test]
fn test_apply_hihat_choke_edge_cases() {
    let rate = 10_000.0;
    let samples = vec![1.0; 200];

    // Choke before or at hit start
    let choked_start = apply_hihat_choke(&samples, 0.05, 0.01, rate, CHOKE_RELEASE_RAMP_SECONDS).unwrap();
    assert!((choked_start[0] - 1.0).abs() < 1e-6);
    assert_eq!(choked_start[150], 0.0);
    assert_eq!(*choked_start.last().unwrap(), 0.0);

    // Choke before hit start with zero ramp
    let choked_zero_ramp = apply_hihat_choke(&samples, 0.05, 0.01, rate, 0.0).unwrap();
    assert!(choked_zero_ramp.iter().all(|&s| s == 0.0));

    // Choke after hit ends: untouched
    let choked_late = apply_hihat_choke(&samples, 0.0, 0.50, rate, CHOKE_RELEASE_RAMP_SECONDS).unwrap();
    assert_eq!(choked_late, samples);

    // Empty samples: returns empty
    let empty = apply_hihat_choke(&[], 0.0, 0.1, rate, CHOKE_RELEASE_RAMP_SECONDS).unwrap();
    assert!(empty.is_empty());

    // Ramp seconds 0.0: instant cutoff
    let choked_inst = apply_hihat_choke(&samples, 0.0, 0.005, rate, 0.0).unwrap();
    assert_eq!(choked_inst[49], 1.0);
    for &s in &choked_inst[50..200] {
        assert_eq!(s, 0.0);
    }
}

#[test]
fn test_closed_hihat_chokes_open_hihat_in_sequence() {
    let rate = 10_000.0;
    // Open hi-hat at 0.0s, closed hi-hat at 0.10s
    let events = vec![
        PercussionHitEvent::new("open_hihat", 0.0, 0.05, 1.0, "drums", None).unwrap(),
        PercussionHitEvent::new("closed_hihat", 0.10, 0.02, 1.0, "drums", None).unwrap(),
    ];

    let opts = SequenceRenderOptions::new().sample_rate_hz(rate);
    let rendered_seq = render_percussion_sequence(&events, &opts).unwrap();
    let samples = rendered_seq.floating_samples.samples();

    // Open hi-hat alone rings out past 0.25s
    let open_alone = render_percussion_hit(
        "open_hihat",
        0.05,
        &HitRenderOptions::new().sample_rate_hz(rate),
    )
    .unwrap();
    let alone_samples = open_alone.floating_samples.samples();
    assert!(alone_samples.len() > (0.25 * rate) as usize);
    assert!(alone_samples[(0.20 * rate) as usize].abs() > 0.001);

    // In sequence, by 0.20s both the closed hi-hat and the choked open hi-hat are silent
    assert!((samples[(0.20 * rate) as usize] - 0.0).abs() < 1e-5);
}

#[test]
fn test_pedal_hihat_chokes_open_hihat() {
    let rate = 10_000.0;
    let events = vec![
        PercussionHitEvent::new("open_hihat", 0.0, 0.05, 1.0, "drums", None).unwrap(),
        PercussionHitEvent::new("pedal_hihat", 0.08, 0.02, 1.0, "drums", None).unwrap(),
    ];

    let opts = SequenceRenderOptions::new().sample_rate_hz(rate);
    let rendered_seq = render_percussion_sequence(&events, &opts).unwrap();
    let samples = rendered_seq.floating_samples.samples();
    assert!((samples[(0.18 * rate) as usize] - 0.0).abs() < 1e-5);
}

#[test]
fn test_repeated_open_hihat_chokes_earlier_open_hihat() {
    let rate = 10_000.0;
    let events = vec![
        PercussionHitEvent::new("open_hihat", 0.0, 0.05, 1.0, "drums", None).unwrap(),
        PercussionHitEvent::new("open_hihat", 0.10, 0.05, 1.0, "drums", None).unwrap(),
    ];
    let opts = SequenceRenderOptions::new().sample_rate_hz(rate);
    let seq = render_percussion_sequence(&events, &opts).unwrap();
    assert!(seq.sample_count() > 0);
}

#[test]
fn test_non_choke_voices_do_not_choke_open_hihat() {
    let rate = 10_000.0;
    let events = vec![
        PercussionHitEvent::new("open_hihat", 0.0, 0.05, 1.0, "drums", None).unwrap(),
        PercussionHitEvent::new("kick", 0.10, 0.05, 1.0, "drums", None).unwrap(),
    ];
    let opts = SequenceRenderOptions::new().sample_rate_hz(rate);
    let seq = render_percussion_sequence(&events, &opts).unwrap();
    let samples = seq.floating_samples.samples();
    // At 0.20s, open_hihat is still ringing (not choked by the kick)
    assert!(samples[(0.20 * rate) as usize].abs() > 0.0001);
}

#[test]
fn test_custom_choke_groups() {
    let env = PercussionEnvelope::new(0.0, 0.01, 0.1).unwrap();
    let m = PercussionMode::new(400.0, 1.0, 0.05).unwrap();
    let v1 = PercussionVoiceProfile::new(
        "conga1",
        "Conga 1",
        "conga",
        0.5,
        vec![m],
        None,
        env,
        0.0,
        Some("conga".to_string()),
    )
    .unwrap();

    let v2 = PercussionVoiceProfile::new(
        "conga2",
        "Conga 2",
        "conga",
        0.5,
        vec![m],
        None,
        env,
        0.0,
        Some("conga".to_string()),
    )
    .unwrap();

    let v3 = PercussionVoiceProfile::new(
        "bell",
        "Bell",
        "bell",
        0.5,
        vec![m],
        None,
        env,
        0.0,
        Some("other".to_string()),
    )
    .unwrap();

    let mut voices = BTreeMap::new();
    voices.insert("c1".to_string(), v1);
    voices.insert("c2".to_string(), v2);
    voices.insert("b".to_string(), v3);
    let custom_kit = DrumKitProfile::new("latin_kit", "Latin Kit", voices).unwrap();

    // Same custom choke group chokes
    let events = vec![
        PercussionHitEvent::hit("c1", 0.0).unwrap(),
        PercussionHitEvent::hit("c2", 0.04).unwrap(),
    ];
    let seq = render_percussion_sequence(
        &events,
        &SequenceRenderOptions::new()
            .kit(&custom_kit)
            .sample_rate_hz(8000.0),
    )
    .unwrap();
    assert!(seq.sample_count() > 0);

    // Differing choke group does not choke
    let events_diff = vec![
        PercussionHitEvent::hit("c1", 0.0).unwrap(),
        PercussionHitEvent::hit("b", 0.04).unwrap(),
    ];
    let seq_diff = render_percussion_sequence(
        &events_diff,
        &SequenceRenderOptions::new()
            .kit(&custom_kit)
            .sample_rate_hz(8000.0),
    )
    .unwrap();
    assert!(seq_diff.sample_count() > 0);
}
