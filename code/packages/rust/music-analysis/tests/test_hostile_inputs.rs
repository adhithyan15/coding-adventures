//! Hostile input testing for CWE-400, CWE-20, CWE-190, CWE-674 bounds (VER15).

use coding_adventures_music_analysis::{
    detect_onsets, extract_harmonic_profile, frequency_to_note, frequency_to_note_a440, hps_pitch,
    rms_energy, rms_energy_framed, spectral_centroid, spectral_flux, spectral_flux_sequence,
    spectral_rolloff, transcribe_pcm_buffer, transcribe_samples, yin_pitch, yin_pitch_track,
    zero_crossing_rate, zero_crossing_rate_framed, AnalysisError, TranscriptionOptions, MAX_BPM,
    MAX_FFT_LEN, MAX_FRAMES, MAX_PARTIALS, MAX_SAMPLE_RATE_HZ, MIN_BPM, MIN_SAMPLE_RATE_HZ,
};
use pcm_audio::{PcmAudio, PcmFormat};

#[test]
fn test_hostile_max_fft_len_bound() {
    assert!(matches!(
        hps_pitch(&[1.0; 100], 44100.0, 5, 40.0, 2000.0, Some(MAX_FFT_LEN + 1)),
        Err(AnalysisError::BudgetExceeded(_))
    ));
    assert!(matches!(
        detect_onsets(&[1.0; 100], 44100.0, 512, MAX_FFT_LEN + 1, 1.5, 0.08),
        Err(AnalysisError::BudgetExceeded(_))
    ));
}

#[test]
fn test_hostile_max_frames_bound() {
    let dummy_frames = vec![vec![1.0, 2.0]; MAX_FRAMES + 1];
    assert!(matches!(
        spectral_flux_sequence(&dummy_frames, true),
        Err(AnalysisError::BudgetExceeded(_))
    ));

    // For framed time-domain features, hop=1 with ~1M elements triggers frame budget before alloc
    let sig = vec![0.5; MAX_FRAMES + 2];
    assert!(matches!(
        zero_crossing_rate_framed(&sig, 1, 1),
        Err(AnalysisError::BudgetExceeded(_))
    ));
    assert!(matches!(
        rms_energy_framed(&sig, 1, 1),
        Err(AnalysisError::BudgetExceeded(_))
    ));
    assert!(matches!(
        yin_pitch_track(&sig, 44100.0, 1, 1, 40.0, 2000.0, 0.15),
        Err(AnalysisError::BudgetExceeded(_))
    ));

    // detect_onsets frame budget check before STFT allocation
    let sig_onsets = vec![0.5; MAX_FRAMES + 1026];
    assert!(matches!(
        detect_onsets(&sig_onsets, 44100.0, 1, 1024, 1.5, 0.08),
        Err(AnalysisError::BudgetExceeded(_))
    ));
}

#[test]
fn test_hostile_max_partials_bound() {
    assert!(matches!(
        extract_harmonic_profile(&[1.0; 500], 44100.0, None, MAX_PARTIALS + 1, "id", "label"),
        Err(AnalysisError::BudgetExceeded(_))
    ));
    assert!(matches!(
        hps_pitch(&[0.5; 200], 44100.0, MAX_PARTIALS + 1, 40.0, 2000.0, None),
        Err(AnalysisError::BudgetExceeded(_))
    ));
}

#[test]
fn test_hostile_max_bpm_bound() {
    let bad_opts = TranscriptionOptions {
        bpm: MAX_BPM + 1.0,
        ..Default::default()
    };
    assert!(matches!(
        transcribe_samples(&[1.0; 500], 44100.0, Some(&bad_opts)),
        Err(AnalysisError::BudgetExceeded(_))
    ));
}

#[test]
fn test_hostile_max_sample_rate_bound() {
    assert!(matches!(
        frequency_to_note(440.0, MAX_SAMPLE_RATE_HZ + 10.0),
        Err(AnalysisError::BudgetExceeded(_))
    ));
    assert!(matches!(
        frequency_to_note((MAX_SAMPLE_RATE_HZ / 2.0) + 1.0, 440.0),
        Err(AnalysisError::BudgetExceeded(_))
    ));
    assert!(matches!(
        spectral_centroid(&[1.0, 2.0], MAX_SAMPLE_RATE_HZ + 1.0),
        Err(AnalysisError::BudgetExceeded(_))
    ));
    assert!(matches!(
        yin_pitch(&[1.0; 100], MAX_SAMPLE_RATE_HZ + 1.0, 40.0, 2000.0, 0.15),
        Err(AnalysisError::BudgetExceeded(_))
    ));
    assert!(matches!(
        hps_pitch(&[1.0; 100], MAX_SAMPLE_RATE_HZ + 1.0, 5, 40.0, 2000.0, None),
        Err(AnalysisError::BudgetExceeded(_))
    ));
    assert!(matches!(
        detect_onsets(&[1.0; 100], MAX_SAMPLE_RATE_HZ + 1.0, 512, 1024, 1.5, 0.08),
        Err(AnalysisError::BudgetExceeded(_))
    ));
    assert!(matches!(
        extract_harmonic_profile(&[1.0; 100], MAX_SAMPLE_RATE_HZ + 1.0, None, 16, "id", "label"),
        Err(AnalysisError::BudgetExceeded(_))
    ));
    assert!(matches!(
        transcribe_samples(&[1.0; 100], MAX_SAMPLE_RATE_HZ + 1.0, None),
        Err(AnalysisError::BudgetExceeded(_))
    ));
}

#[test]
fn test_hostile_cwe_20_nan_rejection() {
    let nan = f64::NAN;
    assert!(matches!(
        frequency_to_note_a440(nan),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        frequency_to_note(440.0, nan),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        spectral_centroid(&[nan, 1.0], 44100.0),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        spectral_flux(&[nan], &[1.0], true),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        spectral_rolloff(&[nan, 1.0], 44100.0, 0.85),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        zero_crossing_rate(&[1.0, nan, 2.0]),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        rms_energy(&[1.0, nan, 2.0]),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        yin_pitch(&[nan; 100], 44100.0, 40.0, 2000.0, 0.15),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        hps_pitch(&[nan; 100], 44100.0, 5, 40.0, 2000.0, None),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        detect_onsets(&[nan; 100], 44100.0, 512, 1024, 1.5, 0.08),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        extract_harmonic_profile(&[nan; 100], 44100.0, None, 16, "id", "label"),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        transcribe_samples(&[nan; 100], 44100.0, None),
        Err(AnalysisError::InvalidInput(_))
    ));
}

#[test]
fn test_hostile_cwe_20_infinity_rejection() {
    for &bad in &[f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            frequency_to_note_a440(bad),
            Err(AnalysisError::InvalidInput(_))
        ));
        assert!(matches!(
            spectral_centroid(&[1.0, bad], 44100.0),
            Err(AnalysisError::InvalidInput(_))
        ));
        assert!(matches!(
            zero_crossing_rate(&[1.0, bad]),
            Err(AnalysisError::InvalidInput(_))
        ));
        assert!(matches!(
            rms_energy(&[1.0, bad]),
            Err(AnalysisError::InvalidInput(_))
        ));
        assert!(matches!(
            yin_pitch(&[1.0, bad], 44100.0, 40.0, 2000.0, 0.15),
            Err(AnalysisError::InvalidInput(_))
        ));
    }
}

#[test]
fn test_hostile_domain_parameter_rejection() {
    // hop_length = 0 fail-closed
    assert!(matches!(
        zero_crossing_rate_framed(&[0.5; 100], 32, 0),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        rms_energy_framed(&[0.5; 100], 32, 0),
        Err(AnalysisError::InvalidInput(_))
    ));

    // spectral_flux empty frames
    let empty_frames: Vec<Vec<f64>> = Vec::new();
    assert!(matches!(
        spectral_flux_sequence(&empty_frames, true),
        Err(AnalysisError::InvalidInput(_))
    ));

    // hps_pitch num_harmonics == 0
    assert!(matches!(
        hps_pitch(&[0.5; 200], 44100.0, 0, 40.0, 2000.0, None),
        Err(AnalysisError::InvalidInput(_))
    ));

    // ppq out of range
    let bad_ppq = TranscriptionOptions {
        ppq: 0,
        ..Default::default()
    };
    assert!(matches!(
        transcribe_samples(&[0.5; 100], 44100.0, Some(&bad_ppq)),
        Err(AnalysisError::InvalidInput(_))
    ));
    let huge_ppq = TranscriptionOptions {
        ppq: 9601,
        ..Default::default()
    };
    assert!(matches!(
        transcribe_samples(&[0.5; 100], 44100.0, Some(&huge_ppq)),
        Err(AnalysisError::InvalidInput(_))
    ));

    // quantize_grid_beats <= 0 or > 16
    let bad_q = TranscriptionOptions {
        quantize_grid_beats: 0.0,
        ..Default::default()
    };
    assert!(matches!(
        transcribe_samples(&[0.5; 100], 44100.0, Some(&bad_q)),
        Err(AnalysisError::InvalidInput(_))
    ));
    let huge_q = TranscriptionOptions {
        quantize_grid_beats: 17.0,
        ..Default::default()
    };
    assert!(matches!(
        transcribe_samples(&[0.5; 100], 44100.0, Some(&huge_q)),
        Err(AnalysisError::BudgetExceeded(_))
    ));

    // bpm below MIN_BPM
    let low_bpm = TranscriptionOptions {
        bpm: MIN_BPM - 0.1,
        ..Default::default()
    };
    assert!(matches!(
        transcribe_samples(&[0.5; 100], 44100.0, Some(&low_bpm)),
        Err(AnalysisError::InvalidInput(_))
    ));

    // sample rate below MIN_SAMPLE_RATE_HZ
    assert!(matches!(
        transcribe_samples(&[0.5; 100], MIN_SAMPLE_RATE_HZ - 1.0, None),
        Err(AnalysisError::InvalidInput(_))
    ));

    // empty PcmAudio buffer
    let format = PcmFormat::mono_16bit(44100.0).expect("format");
    let empty_buf = PcmAudio::from_i16(vec![], format, 0.0).expect("pcm");
    assert!(matches!(
        transcribe_pcm_buffer(&empty_buf, None),
        Err(AnalysisError::InvalidInput(_))
    ));
}

#[test]
fn test_hostile_zero_crossing_rate_framed_len_one_no_nan() {
    // Hostile check: frame_length = 1 must produce zero without NaNs even under hostile/extreme values
    let hostile_vals = [0.0, -0.0, 1e-300, -1e-300, 1e300, -1e300, 0.5, -0.5];
    let res = zero_crossing_rate_framed(&hostile_vals, 1, 1).expect("frame_length=1 must succeed");
    assert_eq!(res.len(), hostile_vals.len());
    for (idx, &v) in res.iter().enumerate() {
        assert!(!v.is_nan(), "res[{idx}] must not be NaN");
        assert_eq!(v, 0.0, "res[{idx}] must be 0.0");
    }
}
