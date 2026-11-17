//! Unit tests for harmonic profile extraction and re-synthesis parity (MUS08, MUS05).

use coding_adventures_music_analysis::{extract_harmonic_profile, AnalysisError};
use musical_instruments::{get_instrument, render_instrument_note, RenderOptions};

fn synthesize_known_instrument(
    f0: f64,
    partial_amps: &[f64],
    sr: f64,
    duration_sec: f64,
) -> Vec<f64> {
    let n = (sr * duration_sec) as usize;
    let mut out = vec![0.0f64; n];
    for (i, slot) in out.iter_mut().enumerate() {
        let t = (i as f64) / sr;
        // Attack - decay - sustain - release envelope
        let env = if t < 0.05 {
            t / 0.05
        } else if t < 0.15 {
            1.0 - 0.3 * ((t - 0.05) / 0.10)
        } else if t < 0.35 {
            0.70
        } else {
            0.70 * (1.0 - (t - 0.35) / 0.05).max(0.0)
        };

        let mut sample = 0.0;
        for (h_idx, &amp) in partial_amps.iter().enumerate() {
            let h = (h_idx + 1) as f64;
            sample += amp * (2.0 * std::f64::consts::PI * h * f0 * t).sin();
        }
        *slot = env * sample;
    }
    out
}

#[test]
fn test_extract_harmonic_profile_synthetic() {
    let sr = 44100.0;
    let f0 = 440.0;
    // Fundamental (1.0), 2nd harmonic (0.5), 3rd harmonic (0.25)
    let known_partials = [1.0, 0.5, 0.25];
    let sig = synthesize_known_instrument(f0, &known_partials, sr, 0.4);

    let profile = extract_harmonic_profile(
        &sig,
        sr,
        Some(f0),
        4,
        "test_synth",
        "Test Synth",
    )
    .expect("profile extraction should succeed");

    assert_eq!(profile.id, "test_synth");
    assert_eq!(profile.synthesis_kind, "additive");
    assert!(profile.harmonic_profile.len() >= 3);

    // Fundamental should have multiplier 1.0 and amplitude near 1.0
    let h1 = &profile.harmonic_profile[0];
    assert_eq!(h1.frequency_multiplier, 1.0);
    assert!((h1.amplitude - 1.0).abs() < 0.05);

    // 2nd harmonic should be near 0.5
    let h2 = &profile.harmonic_profile[1];
    assert_eq!(h2.frequency_multiplier, 2.0);
    assert!((h2.amplitude - 0.5).abs() < 0.1);

    // 3rd harmonic should be near 0.25
    let h3 = &profile.harmonic_profile[2];
    assert_eq!(h3.frequency_multiplier, 3.0);
    assert!((h3.amplitude - 0.25).abs() < 0.1);

    // ADSR envelope validation
    let env = &profile.envelope_profile;
    assert!(env.attack_seconds > 0.0);
    assert!(env.sustain_level > 0.0 && env.sustain_level <= 1.0);
}

#[test]
fn test_extract_harmonic_profile_resynthesis_parity() {
    let sr = 44100.0;
    // Render an original note from flute preset in musical_instruments
    let preset = get_instrument("flute_naive").expect("flute preset");
    let opts = RenderOptions::new().instrument(preset).sample_rate_hz(sr);
    let original_render =
        render_instrument_note("A4", 0.3, &opts).expect("render note should succeed");

    // Extract profile from rendered samples
    let extracted_profile = extract_harmonic_profile(
        original_render.floating_samples.samples(),
        sr,
        Some(440.0),
        6,
        "extracted_flute",
        "Extracted Flute",
    )
    .expect("extraction should succeed");

    assert_eq!(extracted_profile.synthesis_kind, "additive");
    assert!(!extracted_profile.harmonic_profile.is_empty());
    assert_eq!(extracted_profile.harmonic_profile[0].amplitude, 1.0);

    // Re-synthesize using the extracted profile
    let resynth_opts = RenderOptions::new()
        .instrument(extracted_profile)
        .sample_rate_hz(sr);
    let resynth_render =
        render_instrument_note("A4", 0.3, &resynth_opts).expect("resynthesis should succeed");

    // Confirm re-synthesis succeeded, is non-silent, and has reasonable energy
    let resynth_samples = resynth_render.floating_samples.samples();
    assert!(!resynth_samples.is_empty());
    let resynth_energy: f64 = resynth_samples.iter().map(|&s| s * s).sum();
    assert!(resynth_energy > 0.0);
}

#[test]
fn test_extract_harmonic_profile_long_buffer_auto_f0() {
    let sr = 44100.0;
    let n = 6000usize;
    let phase_inc = 2.0 * std::f64::consts::PI * 440.0 / sr;
    let sig: Vec<f64> = (0..n).map(|i| 0.8 * ((i as f64) * phase_inc).sin()).collect();

    let profile = extract_harmonic_profile(
        &sig,
        sr,
        None, // auto-detect via yin_pitch
        30,   // will hit Nyquist break
        "extracted_auto",
        "Extracted Auto",
    )
    .expect("auto f0 extraction");
    assert_eq!(profile.synthesis_kind, "additive");
    assert!(!profile.harmonic_profile.is_empty());
}

#[test]
fn test_extract_harmonic_profile_errors() {
    let sr = 44100.0;
    assert!(matches!(
        extract_harmonic_profile(&vec![0.0; 2000], sr, None, 16, "id", "label"),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        extract_harmonic_profile(&[], sr, None, 16, "id", "label"),
        Err(AnalysisError::InvalidInput(_))
    ));
}
