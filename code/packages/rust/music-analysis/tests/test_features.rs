//! Unit tests for audio feature extraction (MUS08, VER15).

use coding_adventures_music_analysis::{
    rms_energy, rms_energy_framed, spectral_centroid, spectral_flux, spectral_flux_sequence,
    spectral_rolloff, zero_crossing_rate, zero_crossing_rate_framed, AnalysisError,
};

#[test]
fn test_spectral_centroid_single_tone() {
    // 5 bins: [0, 2500, 5000, 7500, 10000] Hz for sr=20000
    // Peak at bin 2 (5000 Hz)
    let spectrum = [0.0, 0.0, 1.0, 0.0, 0.0];
    let c = spectral_centroid(&spectrum, 20000.0).expect("centroid should compute");
    assert!((c - 5000.0).abs() < 1e-3);
}

#[test]
fn test_spectral_centroid_flat_spectrum() {
    // Flat spectrum centroid should be exactly at Nyquist / 2
    let spectrum = [1.0; 9];
    let c = spectral_centroid(&spectrum, 16000.0).expect("flat centroid should compute");
    assert!((c - 4000.0).abs() < 1e-3);
}

#[test]
fn test_spectral_centroid_zero_energy() {
    let spectrum = [0.0; 5];
    assert_eq!(spectral_centroid(&spectrum, 44100.0).unwrap(), 0.0);
    assert_eq!(spectral_centroid(&[1.0], 44100.0).unwrap(), 0.0);
}

#[test]
fn test_spectral_flux_pair() {
    let spec1 = [1.0, 2.0, 3.0, 4.0];
    let spec2 = [1.0, 2.0, 3.0, 4.0];
    // No change
    assert_eq!(spectral_flux(&spec1, &spec2, true).unwrap(), 0.0);

    // Increase in upper bins
    let spec3 = [1.0, 2.0, 5.0, 7.0];
    let flux = spectral_flux(&spec1, &spec3, true).unwrap();
    assert!((flux - 5.0).abs() < 1e-5);

    // Decrease with rectified=true should be 0
    let spec4 = [0.5, 1.0, 1.5, 2.0];
    assert_eq!(spectral_flux(&spec1, &spec4, true).unwrap(), 0.0);

    // Decrease with rectified=false
    let unrectified = spectral_flux(&spec1, &spec4, false).unwrap();
    assert!(unrectified > 0.0);
}

#[test]
fn test_spectral_flux_sequence() {
    let frames = [vec![1.0, 1.0], vec![2.0, 2.0], vec![1.0, 1.0]];
    let fluxes = spectral_flux_sequence(&frames, true).unwrap();
    assert_eq!(fluxes.len(), 2);
    assert!((fluxes[0] - 2.0).abs() < 1e-5);
    assert!((fluxes[1] - 0.0).abs() < 1e-5); // rectified decrease

    let single_frame = [vec![1.0, 2.0]];
    assert!(spectral_flux_sequence(&single_frame, true).unwrap().is_empty());
}

#[test]
fn test_spectral_rolloff() {
    // 5 bins: [0, 2500, 5000, 7500, 10000] Hz for sr=20000
    let spectrum = [0.0, 0.0, 1.0, 0.0, 0.0];
    // All energy is at 5000 Hz
    let ro = spectral_rolloff(&spectrum, 20000.0, 0.85).unwrap();
    assert!((ro - 5000.0).abs() < 1e-3);

    // Flat spectrum
    let spectrum_flat = [1.0; 10];
    let ro_flat = spectral_rolloff(&spectrum_flat, 18000.0, 0.50).unwrap();
    assert!(ro_flat > 0.0 && ro_flat < 9000.0);

    // Zero energy
    assert_eq!(spectral_rolloff(&[0.0; 5], 20000.0, 0.85).unwrap(), 0.0);
    assert_eq!(spectral_rolloff(&[1.0], 20000.0, 0.85).unwrap(), 0.0);

    // 1.0 rolloff percentage reaches end of loop without accumulator exceeding
    let res = spectral_rolloff(&[1.0, 2.0, 3.0], 44100.0, 1.0).unwrap();
    assert!((res - 22050.0).abs() < 1e-3);
}

#[test]
fn test_zero_crossing_rate() {
    // Constant positive -> 0
    assert_eq!(zero_crossing_rate(&[1.0, 1.0, 1.0, 1.0]).unwrap(), 0.0);

    // Alternating -> 1.0
    assert_eq!(zero_crossing_rate(&[1.0, -1.0, 1.0, -1.0]).unwrap(), 1.0);

    // Short signal
    assert_eq!(zero_crossing_rate(&[1.0]).unwrap(), 0.0);

    // Sliding window framing
    let sig = [1.0, 1.0, -1.0, -1.0, 1.0, 1.0];
    let zcr_framed = zero_crossing_rate_framed(&sig, 3, 1).unwrap();
    assert_eq!(zcr_framed.len(), 4);

    // Short signal with framing
    let short_framed = zero_crossing_rate_framed(&[1.0, -1.0], 10, 5).unwrap();
    assert_eq!(short_framed.len(), 1);
}

#[test]
fn test_zero_crossing_rate_framed_length_one_regression() {
    // Regression test for Finding 1: frame_length == 1 must not divide by 0 / produce NaN
    let sig = [1.0, -1.0, 1.0, -1.0, 1.0];
    let zcr = zero_crossing_rate_framed(&sig, 1, 1).expect("frame_length=1 must succeed");
    assert_eq!(zcr.len(), 5);
    for &val in &zcr {
        assert!(!val.is_nan(), "zcr value must not be NaN");
        assert_eq!(
            val, 0.0,
            "frame of length 1 has 0 crossings, rate must be 0.0"
        );
    }

    // Also with hop_length > 1
    let zcr_hop2 =
        zero_crossing_rate_framed(&sig, 1, 2).expect("frame_length=1 hop=2 must succeed");
    assert_eq!(zcr_hop2.len(), 3);
    for &val in &zcr_hop2 {
        assert!(!val.is_nan());
        assert_eq!(val, 0.0);
    }
}

#[test]
fn test_rms_energy() {
    // DC signal
    assert!((rms_energy(&[2.0, 2.0, 2.0]).unwrap() - 2.0).abs() < 1e-5);

    // Empty raises InvalidInput
    assert!(matches!(rms_energy(&[]), Err(AnalysisError::InvalidInput(_))));

    // Zero signal
    assert_eq!(rms_energy(&[0.0, 0.0]).unwrap(), 0.0);

    // Sine wave RMS should be A / sqrt(2)
    let n = 1000usize;
    let sine: Vec<f64> = (0..n)
        .map(|i| (2.0 * std::f64::consts::PI * 5.0 * (i as f64) / (n as f64)).sin())
        .collect();
    let rms = rms_energy(&sine).unwrap();
    assert!((rms - 1.0 / 2.0f64.sqrt()).abs() < 1e-2);

    // Windowed framing
    let sig = [1.0; 10];
    let rms_framed = rms_energy_framed(&sig, 4, 2).unwrap();
    assert_eq!(rms_framed.len(), 4);
    for val in rms_framed {
        assert!((val - 1.0).abs() < 1e-5);
    }

    // Short signal with framing
    let short_framed = rms_energy_framed(&[2.0, 2.0], 10, 5).unwrap();
    assert_eq!(short_framed.len(), 1);
    assert!((short_framed[0] - 2.0).abs() < 1e-5);
}

#[test]
fn test_features_invalid_inputs() {
    assert!(matches!(
        spectral_centroid(&[], 44100.0),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        spectral_centroid(&[1.0, 2.0], -100.0),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        spectral_rolloff(&[1.0, 2.0], 44100.0, 0.0),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        spectral_rolloff(&[1.0, 2.0], 44100.0, 1.5),
        Err(AnalysisError::BudgetExceeded(_))
    ));
}
