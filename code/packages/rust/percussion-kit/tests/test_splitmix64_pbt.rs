//! Deterministic SplitMix64 PRNG, property-based testing (PBT), and fuzzing.

use percussion_kit::{
    all_standard_hit_ids, render_percussion_sequence, PercussionHitEvent,
    SequenceRenderOptions, SplitMix64,
};

#[test]
fn test_splitmix64_prng_determinism() {
    let mut rng1 = SplitMix64::new(0x1234_5678_9ABC_DEF0);
    let mut rng2 = SplitMix64::new(0x1234_5678_9ABC_DEF0);

    for _ in 0..100 {
        assert_eq!(rng1.next_u64(), rng2.next_u64());
        assert_eq!(rng1.next_f64(), rng2.next_f64());
        assert_eq!(rng1.next_float_signed(), rng2.next_float_signed());
    }

    let mut rng_diff = SplitMix64::new(0xDEAD_BEEF_CAFE_BABE);
    assert_ne!(rng1.next_u64(), rng_diff.next_u64());
}

#[test]
fn test_pbt_splitmix64_determinism_across_random_sequences() {
    let mut prng = SplitMix64::new(0xCAFE_BABE_2026);
    let all_hits = all_standard_hit_ids();

    for _ in 0..15 {
        let event_count = (prng.next_u64() % 4) + 1;
        let mut events = Vec::new();
        for _ in 0..event_count {
            let h_id = *prng.choice(all_hits);
            let t = prng.uniform(0.0, 0.10);
            let d = prng.uniform(0.01, 0.04);
            let v = prng.uniform(0.1, 1.0);
            events.push(PercussionHitEvent::new(h_id, t, d, v, "drums", None).unwrap());
        }

        let opts = SequenceRenderOptions::new().sample_rate_hz(8000.0);
        let run_a = render_percussion_sequence(&events, &opts).unwrap();
        let run_b = render_percussion_sequence(&events, &opts).unwrap();

        assert_eq!(
            run_a.floating_samples.samples(),
            run_b.floating_samples.samples()
        );
        assert_eq!(
            run_a.pcm_buffer.samples(),
            run_b.pcm_buffer.samples()
        );
    }
}

#[test]
fn test_pbt_timeline_clamping_invariant() {
    let all_hits = all_standard_hit_ids();

    // 15 simultaneous loud hits at t = 0
    let pile: Vec<PercussionHitEvent> = all_hits
        .iter()
        .map(|&h| PercussionHitEvent::new(h, 0.0, 0.05, 1.0, "drums", None).unwrap())
        .collect();

    let opts = SequenceRenderOptions::new().sample_rate_hz(8000.0);
    let res = render_percussion_sequence(&pile, &opts).unwrap();

    // Clamping invariant: every sample strictly in [-1.0, 1.0]
    for &s in res.floating_samples.samples() {
        assert!(
            (-1.0..=1.0).contains(&s),
            "sample {s} must be clamped to [-1.0, 1.0]"
        );
    }

    // PCM invariant: every sample in [-32768, 32767]
    for &p in res.pcm_buffer.samples() {
        assert!(
            (-32768..=32767).contains(&p),
            "PCM sample {p} must be within 16-bit signed range"
        );
    }
}

#[test]
fn test_streaming_memory_overlapping_events() {
    // 200 overlapping events mixed iteratively
    let events: Vec<PercussionHitEvent> = (0..200)
        .map(|_| PercussionHitEvent::new("kick", 0.0, 0.0001, 1.0, "drums", None).unwrap())
        .collect();

    let opts = SequenceRenderOptions::new().sample_rate_hz(8000.0);
    let seq = render_percussion_sequence(&events, &opts).unwrap();
    assert!(seq.sample_count() > 0);
}
