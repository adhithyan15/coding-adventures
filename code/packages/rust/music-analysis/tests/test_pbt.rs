//! Property-based testing for music analysis using SplitMix64 PRNG (MUS08, VER15).

use coding_adventures_music_analysis::{
    frequency_to_note_a440, rms_energy, spectral_centroid, spectral_rolloff, yin_pitch_simple,
    zero_crossing_rate,
};
use coding_adventures_test_bootstrap::prng::SplitMix64;
use note_frequency::note_to_frequency;

fn next_f64(rng: &mut SplitMix64) -> f64 {
    (rng.next_u64() >> 11) as f64 * (1.0 / ((1u64 << 53) as f64))
}

#[test]
fn test_pbt_frequency_to_note_invertibility() {
    let mut prng = SplitMix64::new(0xFEEDFACE11223344);
    for _ in 0..50 {
        // Sample random frequency in [50 Hz, 4000 Hz]
        let hz = 50.0 + next_f64(&mut prng) * 3950.0;
        let matched = frequency_to_note_a440(hz).expect("conversion");

        // Parse note back via note_frequency
        let nominal_hz = note_to_frequency(&matched.note).expect("valid note string");

        // Reconstruct frequency: f = nominal * 2^(cents / 1200)
        let reconstructed_hz = nominal_hz * 2.0f64.powf(matched.cents / 1200.0);
        let rel_err = (hz - reconstructed_hz).abs() / hz;
        assert!(
            rel_err < 1e-3,
            "hz: {hz}, reconstructed: {reconstructed_hz}, rel_err: {rel_err}"
        );
    }
}

#[test]
fn test_pbt_feature_value_invariants() {
    let mut prng = SplitMix64::new(0xDEADBEEFCAFEBABE);
    let sr = 44100.0;
    for _ in 0..20 {
        let length = (next_f64(&mut prng) * 200.0) as usize + 20;
        let samples: Vec<f64> = (0..length)
            .map(|_| (next_f64(&mut prng) * 2.0) - 1.0)
            .collect();

        let rms = rms_energy(&samples).expect("rms");
        assert!(rms >= 0.0);

        let zcr = zero_crossing_rate(&samples).expect("zcr");
        assert!((0.0..=1.0).contains(&zcr));

        // Positive spectrum
        let spec: Vec<f64> = (0..length).map(|_| next_f64(&mut prng) * 10.0).collect();
        let centroid = spectral_centroid(&spec, sr).expect("centroid");
        assert!(centroid >= 0.0 && centroid <= (sr / 2.0));
    }
}

#[test]
fn test_pbt_rolloff_monotonicity() {
    let mut prng = SplitMix64::new(0x55AA55AA33CC33CC);
    let sr = 44100.0;
    for _ in 0..20 {
        let k_bins = (next_f64(&mut prng) * 100.0) as usize + 10;
        let spec: Vec<f64> = (0..k_bins).map(|_| next_f64(&mut prng) * 5.0).collect();

        let r50 = spectral_rolloff(&spec, sr, 0.50).expect("r50");
        let r85 = spectral_rolloff(&spec, sr, 0.85).expect("r85");
        let r95 = spectral_rolloff(&spec, sr, 0.95).expect("r95");

        assert!(r50 <= r85 + 1e-6);
        assert!(r85 <= r95 + 1e-6);
    }
}

#[test]
fn test_pbt_determinism() {
    let mut prng = SplitMix64::new(0x1357924680BDFE);
    let sr = 44100.0;
    let sig: Vec<f64> = (0..1024)
        .map(|_| (next_f64(&mut prng) * 2.0) - 1.0)
        .collect();

    let p1 = yin_pitch_simple(&sig, sr).expect("yin1");
    let p2 = yin_pitch_simple(&sig, sr).expect("yin2");
    assert_eq!(p1, p2);

    let r1 = rms_energy(&sig).expect("rms1");
    let r2 = rms_energy(&sig).expect("rms2");
    assert_eq!(r1, r2);
}
