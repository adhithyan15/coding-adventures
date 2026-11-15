//! Unit tests for YIN and HPS pitch detection (MUS08, VER15).

use coding_adventures_music_analysis::{
    hps_pitch, yin_pitch, yin_pitch_simple, yin_pitch_track, AnalysisError,
};

fn synthesize_sine(freq_hz: f64, sample_rate_hz: f64, duration_sec: f64) -> Vec<f64> {
    let n = (sample_rate_hz * duration_sec) as usize;
    let phase_inc = 2.0 * std::f64::consts::PI * freq_hz / sample_rate_hz;
    (0..n).map(|i| ((i as f64) * phase_inc).sin()).collect()
}

fn synthesize_harmonic(
    f0: f64,
    harmonics: &[f64],
    sample_rate_hz: f64,
    duration_sec: f64,
) -> Vec<f64> {
    let n = (sample_rate_hz * duration_sec) as usize;
    let mut out = vec![0.0f64; n];
    for (h_idx, &amp) in harmonics.iter().enumerate() {
        let h = (h_idx + 1) as f64;
        let freq = h * f0;
        if freq < sample_rate_hz / 2.0 {
            let phase_inc = 2.0 * std::f64::consts::PI * freq / sample_rate_hz;
            for (i, val) in out.iter_mut().enumerate() {
                *val += amp * ((i as f64) * phase_inc).sin();
            }
        }
    }
    out
}

#[test]
fn test_yin_pitch_pure_sines() {
    let sr = 44100.0;
    for &target_freq in &[110.0, 220.0, 440.0, 880.0] {
        let sig = synthesize_sine(target_freq, sr, 0.1);
        let est = yin_pitch(&sig, sr, 50.0, 1200.0, 0.15).expect("pitch should detect");
        let rel_err = (est - target_freq).abs() / target_freq;
        assert!(
            rel_err < 0.01,
            "target {target_freq}, got {est}, rel_err {rel_err}"
        );
    }
}

#[test]
fn test_yin_pitch_harmonic_signal() {
    let sr = 44100.0;
    let target_f0 = 261.63; // C4
    let sig = synthesize_harmonic(target_f0, &[1.0, 0.6, 0.4, 0.2], sr, 0.1);
    let est = yin_pitch(&sig, sr, 80.0, 600.0, 0.15).expect("harmonic pitch should detect");
    let rel_err = (est - target_f0).abs() / target_f0;
    assert!(rel_err < 0.01);
}

#[test]
fn test_yin_pitch_silence() {
    let sr = 44100.0;
    let sig = vec![0.0; 2000];
    let est = yin_pitch_simple(&sig, sr).expect("silence pitch");
    assert_eq!(est, 0.0);
}

#[test]
fn test_yin_pitch_track() {
    let sr = 44100.0;
    // Two note sequence: 0.1s of 220 Hz followed by 0.1s of 440 Hz
    let sig1 = synthesize_sine(220.0, sr, 0.1);
    let sig2 = synthesize_sine(440.0, sr, 0.1);
    let combined = [sig1, sig2].concat();

    let track =
        yin_pitch_track(&combined, sr, 1024, 256, 40.0, 2000.0, 0.15).expect("pitch track");
    assert!(track.len() > 5);

    // Early frames should detect near 220 Hz
    let early_pitches: Vec<f64> = track[..4].iter().copied().filter(|&p| p > 0.0).collect();
    assert!(early_pitches
        .iter()
        .any(|&p| (p - 220.0).abs() / 220.0 < 0.02));

    // Later frames should detect near 440 Hz
    let late_pitches: Vec<f64> = track[track.len() - 4..]
        .iter()
        .copied()
        .filter(|&p| p > 0.0)
        .collect();
    assert!(late_pitches
        .iter()
        .any(|&p| (p - 440.0).abs() / 440.0 < 0.02));
}

#[test]
fn test_hps_pitch_harmonic_signal() {
    let sr = 44100.0;
    let target_f0 = 440.0;
    let sig = synthesize_harmonic(target_f0, &[1.0, 0.8, 0.6, 0.4, 0.2], sr, 0.15);
    let est = hps_pitch(&sig, sr, 5, 100.0, 1000.0, None).expect("HPS pitch");
    let rel_err = (est - target_f0).abs() / target_f0;
    assert!(
        rel_err < 0.02,
        "target {target_f0}, got {est}, rel_err {rel_err}"
    );
}

#[test]
fn test_yin_pitch_short_signal() {
    let sr = 44100.0;
    // Very short signal where tau_max <= tau_min
    let sig = [0.1, -0.1, 0.2, -0.2];
    assert_eq!(yin_pitch_simple(&sig, sr).unwrap(), 0.0);

    // Short signal in pitch track
    let track = yin_pitch_track(&[0.5; 100], sr, 512, 128, 40.0, 2000.0, 0.15).unwrap();
    assert_eq!(track.len(), 1);
}

#[test]
fn test_hps_pitch_silence() {
    let sr = 44100.0;
    let sig = vec![0.0; 2048];
    assert_eq!(
        hps_pitch(&sig, sr, 5, 40.0, 2000.0, None).unwrap(),
        0.0
    );
}

#[test]
fn test_hps_pitch_explicit_fft() {
    let sr = 44100.0;
    let sig = synthesize_harmonic(440.0, &[1.0, 0.8, 0.6, 0.4], sr, 0.15);
    let est = hps_pitch(&sig, sr, 5, 40.0, 2000.0, Some(2048)).unwrap();
    let rel_err = (est - 440.0).abs() / 440.0;
    assert!(rel_err < 0.03);

    // Inverted min/max search bin condition
    assert_eq!(
        hps_pitch(&sig, sr, 5, 440.0, 440.1, Some(512)).unwrap(),
        0.0
    );
}

#[test]
fn test_pitch_invalid_inputs() {
    assert!(matches!(
        yin_pitch_simple(&[], 44100.0),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        yin_pitch_simple(&[1.0], -44100.0),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        yin_pitch(&[1.0; 100], 44100.0, 1000.0, 500.0, 0.15),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        hps_pitch(&[1.0; 100], 44100.0, 5, 1000.0, 500.0, None),
        Err(AnalysisError::InvalidInput(_))
    ));
}
