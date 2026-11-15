//! Unit tests for multi-resolution onset detection (MUS08, VER15).

use coding_adventures_music_analysis::{
    detect_onsets, detect_onsets_samples, detect_onsets_simple, AnalysisError,
};

fn make_tone_burst(freq: f64, duration_sec: f64, sample_rate_hz: f64) -> Vec<f64> {
    let n = (sample_rate_hz * duration_sec) as usize;
    let phase_inc = 2.0 * std::f64::consts::PI * freq / sample_rate_hz;
    (0..n)
        .map(|i| {
            let t = (i as f64) / sample_rate_hz;
            let env = (-15.0 * t).exp();
            env * ((i as f64) * phase_inc).sin()
        })
        .collect()
}

#[test]
fn test_detect_onsets_single_burst() {
    let sr = 44100.0;
    let burst = make_tone_burst(440.0, 0.2, sr);
    let onsets = detect_onsets_simple(&burst, sr).expect("onset detection should run");
    assert!(!onsets.is_empty());
    // First onset should be near 0.0s
    assert!((onsets[0] - 0.0).abs() < 0.03);
}

#[test]
fn test_detect_onsets_multiple_events() {
    let sr = 44100.0;
    let silence = vec![0.0; (0.15 * sr) as usize];
    let burst1 = make_tone_burst(440.0, 0.1, sr);
    let burst2 = make_tone_burst(880.0, 0.1, sr);
    let burst3 = make_tone_burst(550.0, 0.1, sr);

    // 3 bursts separated by silence
    let mut sig = Vec::new();
    sig.extend_from_slice(&burst1);
    sig.extend_from_slice(&silence);
    sig.extend_from_slice(&burst2);
    sig.extend_from_slice(&silence);
    sig.extend_from_slice(&burst3);

    let onsets = detect_onsets(&sig, sr, 512, 1024, 1.5, 0.1).expect("multiple onsets");
    assert_eq!(onsets.len(), 3);
    // First onset near 0.0s
    assert!((onsets[0] - 0.0).abs() < 0.03);
    // Second onset near 0.25s
    assert!((onsets[1] - 0.25).abs() < 0.05);
    // Third onset near 0.50s
    assert!((onsets[2] - 0.50).abs() < 0.05);
}

#[test]
fn test_detect_onsets_return_samples() {
    let sr = 44100.0;
    let burst = make_tone_burst(440.0, 0.2, sr);
    let sample_onsets = detect_onsets_samples(&burst, sr, 512, 1024, 1.5, 0.08).unwrap();
    assert!(!sample_onsets.is_empty());
    assert_eq!(sample_onsets[0], 0);
}

#[test]
fn test_detect_onsets_silence() {
    let sr = 44100.0;
    let sig = vec![0.0; 4096];
    let onsets = detect_onsets_simple(&sig, sr).unwrap();
    assert!(onsets.is_empty());
}

#[test]
fn test_detect_onsets_short_signal() {
    let sr = 44100.0;
    let short_sig = [0.5, 0.8, -0.4, 0.2];
    let onsets = detect_onsets_simple(&short_sig, sr).unwrap();
    assert_eq!(onsets, vec![0.0]);
}

#[test]
fn test_detect_onsets_invalid_inputs() {
    assert!(matches!(
        detect_onsets_simple(&[], 44100.0),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        detect_onsets_simple(&[1.0], -44100.0),
        Err(AnalysisError::InvalidInput(_))
    ));
}
