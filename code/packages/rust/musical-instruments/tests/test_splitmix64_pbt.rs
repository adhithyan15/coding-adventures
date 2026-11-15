//! Property-based testing and golden vectors for SplitMix64 PRNG.

use musical_instruments::{
    render_instrument_note, RenderOptions, SplitMix64, VariationProfile,
};

#[test]
fn test_splitmix64_golden_reference_vectors() {
    // Seed 0 golden sequence
    let mut rng0 = SplitMix64::new(0);
    assert_eq!(rng0.next_u64(), 16294208416658607535);
    assert_eq!(rng0.next_u64(), 7960286522194355700);
    assert_eq!(rng0.next_u64(), 487617019471545679);
    assert_eq!(rng0.next_u64(), 17909611376780542444);

    // Seed 18446744073709551615 (2^64 - 1) golden sequence
    let mut rng_max = SplitMix64::new(18446744073709551615);
    assert_eq!(rng_max.next_u64(), 16490336266968443936);
    assert_eq!(rng_max.next_u64(), 16834447057089888969);
    assert_eq!(rng_max.next_u64(), 4048727598324417001);

    // Seed 1234567 golden sequence
    let mut rng_sub = SplitMix64::new(1234567);
    assert_eq!(rng_sub.next_u64(), 6457827717110365317);
    assert_eq!(rng_sub.next_u64(), 3203168211198807973);
    assert_eq!(rng_sub.next_u64(), 9817491932198370423);
}

#[test]
fn test_splitmix64_float_generation() {
    let mut rng = SplitMix64::new(42);
    for _ in 0..1000 {
        let fs = rng.next_float_signed();
        assert!((-1.0..=1.0).contains(&fs), "fs={fs}");
        let fu = rng.next_float_unit();
        assert!((0.0..=1.0).contains(&fu), "fu={fu}");
    }
}

#[test]
fn test_pbt_splitmix64_note_render_bounds_and_determinism() {
    let mut prng = SplitMix64::new(0xFEED_FACE_2026);
    let instruments = ["sine", "flute_naive", "violin_naive", "piano_naive"];
    let notes = ["C4", "E4", "G4", "A4", "C5"];

    for _ in 0..20 {
        let inst = instruments[(prng.next_u64() as usize) % instruments.len()];
        let note = notes[(prng.next_u64() as usize) % notes.len()];
        let seed_val = prng.next_u64() & 0x7FFF_FFFF;
        let dur = 0.01 + (prng.next_float_unit() * 0.04);

        let var = VariationProfile::new(10.0, 0.1, 2.0, Some(seed_val), 0.05).unwrap();

        let opts = RenderOptions::new()
            .instrument_id(inst)
            .sample_rate_hz(16_000.0)
            .variation_profile(var);

        let r1 = render_instrument_note(note, dur, &opts).unwrap();
        let r2 = render_instrument_note(note, dur, &opts).unwrap();

        // Determinism invariant
        assert_eq!(r1.floating_samples.samples(), r2.floating_samples.samples());
        assert_eq!(r1.pcm_buffer.samples(), r2.pcm_buffer.samples());

        // Sample bounds invariants
        for &s in r1.floating_samples.samples() {
            assert!((-1.0..=1.0).contains(&s), "sample out of bounds: {s}");
        }
        for &p in r1.pcm_buffer.samples() {
            assert!((-32768..=32767).contains(&p), "pcm out of bounds: {p}");
        }
    }
}
