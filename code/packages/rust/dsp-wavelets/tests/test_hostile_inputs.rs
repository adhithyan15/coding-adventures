//! Hostile input testing for CWE-20, CWE-190, CWE-400, and CWE-674 in `dsp-wavelets` (VER15).

use dsp_wavelets::{
    cwt, dwt_1d, dwt_2d, idwt_1d, idwt_2d, slice_level, split_levels, Band, WaveletBoundary,
    WaveletError, WaveletType, MAX_SAMPLES, MAX_SCALES, MAX_WAVELET_LEVELS,
};

#[test]
fn test_hostile_cwe_20_non_finite_inputs_rejected() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(matches!(
            dwt_1d(&[1.0, bad, 2.0, 3.0], WaveletType::Haar, 1, WaveletBoundary::Periodic),
            Err(WaveletError::InvalidParam(_))
        ));
        assert!(matches!(
            idwt_1d(
                &[1.0, bad, 2.0, 3.0],
                WaveletType::Haar,
                1,
                WaveletBoundary::Periodic,
                4
            ),
            Err(WaveletError::InvalidCoefficients(_))
        ));
        assert!(matches!(
            dwt_2d(&[1.0, bad, 2.0, 3.0], 2, 2, WaveletType::Haar, 1, WaveletBoundary::Periodic),
            Err(WaveletError::InvalidParam(_))
        ));
        assert!(matches!(
            idwt_2d(
                &[1.0, bad, 2.0, 3.0],
                2,
                2,
                WaveletType::Haar,
                1,
                WaveletBoundary::Periodic
            ),
            Err(WaveletError::InvalidCoefficients(_))
        ));
        assert!(matches!(
            cwt(&[1.0, bad, 2.0, 3.0], &[1.0], WaveletType::Morlet),
            Err(WaveletError::InvalidParam(_))
        ));
        assert!(matches!(
            cwt(&[1.0, 2.0, 3.0, 4.0], &[bad], WaveletType::Morlet),
            Err(WaveletError::InvalidParam(_))
        ));
    }
}

#[test]
fn test_hostile_cwe_20_cwt_subnormal_scales_and_overflow() {
    // Subnormal / tiny scales where 1.0 / s overflows to Inf -> 0.0 * Inf = NaN if unguarded.
    for tiny_scale in [0.0_f32, -0.0, -1.0, f32::from_bits(1), 1e-40_f32, 1e-39_f32] {
        for wt in [WaveletType::Morlet, WaveletType::MexicanHat] {
            assert!(
                matches!(
                    cwt(&[1.0, 2.0, 3.0, 4.0], &[tiny_scale], wt),
                    Err(WaveletError::InvalidParam(_))
                ),
                "tiny/non-positive scale {tiny_scale:e} must be rejected for {:?}",
                wt
            );
        }
    }

    // Finite inputs whose transform output overflows f32 to Inf/NaN.
    assert!(matches!(
        dwt_1d(&[f32::MAX; 4], WaveletType::Haar, 1, WaveletBoundary::Periodic),
        Err(WaveletError::InvalidParam(_))
    ));
    assert!(matches!(
        idwt_1d(
            &[f32::MAX; 4],
            WaveletType::Haar,
            1,
            WaveletBoundary::Periodic,
            4
        ),
        Err(WaveletError::InvalidCoefficients(_))
    ));
    assert!(matches!(
        cwt(&[f32::MAX; 4], &[1e-38_f32], WaveletType::Morlet),
        Err(WaveletError::InvalidParam(_))
    ));
    assert!(matches!(
        cwt(&[f32::MAX; 4], &[1e-38_f32], WaveletType::MexicanHat),
        Err(WaveletError::InvalidParam(_))
    ));

    // Large-amplitude finite Morlet CWT must not spuriously overflow via re^2 + im^2 (uses hypot).
    let large_amp = cwt(&[1.0e25_f32; 4], &[1.0e14_f32], WaveletType::Morlet)
        .expect("large finite Morlet CWT must use hypot without spurious overflow");
    assert_eq!(large_amp.len(), 4);
    assert!(large_amp.iter().all(|v| v.is_finite() && *v >= 0.0));
}

#[test]
fn test_hostile_cwe_190_overflow_and_invalid_params() {
    // Dimension overflow in 2-D DWT/IDWT
    assert!(matches!(
        dwt_2d(&[1.0], u32::MAX, u32::MAX, WaveletType::Haar, 1, WaveletBoundary::Periodic),
        Err(WaveletError::InvalidParam(_))
    ));
    assert!(matches!(
        idwt_2d(&[1.0], u32::MAX, u32::MAX, WaveletType::Haar, 1, WaveletBoundary::Periodic),
        Err(WaveletError::InvalidParam(_))
    ));

    // Zero / out-of-range levels and lengths
    for bad_levels in [0u32, MAX_WAVELET_LEVELS + 1, u32::MAX] {
        assert!(matches!(
            dwt_1d(&[1.0; 8], WaveletType::Haar, bad_levels, WaveletBoundary::Periodic),
            Err(WaveletError::InvalidParam(_))
        ));
        assert!(matches!(
            idwt_1d(&[1.0; 8], WaveletType::Haar, bad_levels, WaveletBoundary::Periodic, 8),
            Err(WaveletError::InvalidParam(_))
        ));
        assert!(matches!(
            split_levels(8, 8, bad_levels),
            Err(WaveletError::InvalidParam(_))
        ));
    }

    assert!(matches!(
        idwt_1d(&[1.0; 4], WaveletType::Haar, 1, WaveletBoundary::Periodic, 0),
        Err(WaveletError::InvalidParam(_))
    ));
    assert!(matches!(
        slice_level(&[1.0; 8], 8, 2, 0, Band::Detail),
        Err(WaveletError::InvalidParam(_))
    ));
    assert!(matches!(
        slice_level(&[1.0; 8], 8, 2, 3, Band::Detail),
        Err(WaveletError::InvalidParam(_))
    ));
    assert!(matches!(
        slice_level(&[1.0; 8], 8, 2, 1, Band::Approximation),
        Err(WaveletError::InvalidParam(_))
    ));

    // 2-D DWT/IDWT symmetry: SignalTooShort and unsupported wavelet checks up front.
    assert!(matches!(
        dwt_2d(&[1.0; 4], 2, 2, WaveletType::Haar, 2, WaveletBoundary::Periodic),
        Err(WaveletError::SignalTooShort(_))
    ));
    assert!(matches!(
        idwt_2d(&[1.0; 7], 2, 2, WaveletType::Haar, 2, WaveletBoundary::Periodic),
        Err(WaveletError::SignalTooShort(_))
    ));
    assert!(matches!(
        dwt_2d(&[1.0; 64], 8, 8, WaveletType::Daubechies(2), 3, WaveletBoundary::Periodic),
        Err(WaveletError::SignalTooShort(_))
    ));
    assert!(matches!(
        idwt_2d(&[1.0; 64], 8, 8, WaveletType::Daubechies(2), 3, WaveletBoundary::Periodic),
        Err(WaveletError::SignalTooShort(_))
    ));
    assert!(matches!(
        dwt_2d(&[1.0; 64], 8, 8, WaveletType::Daubechies(3), 1, WaveletBoundary::Periodic),
        Err(WaveletError::InvalidParam(_))
    ));
}

#[test]
fn test_hostile_cwe_400_caps_before_allocation() {
    assert_eq!(
        dwt_1d(&[], WaveletType::Haar, 1, WaveletBoundary::Periodic),
        Err(WaveletError::EmptySignal)
    );
    assert_eq!(
        idwt_1d(&[], WaveletType::Haar, 1, WaveletBoundary::Periodic, 4),
        Err(WaveletError::EmptySignal)
    );
    assert_eq!(
        dwt_2d(&[], 2, 2, WaveletType::Haar, 1, WaveletBoundary::Periodic),
        Err(WaveletError::EmptySignal)
    );
    assert_eq!(
        idwt_2d(&[], 2, 2, WaveletType::Haar, 1, WaveletBoundary::Periodic),
        Err(WaveletError::EmptySignal)
    );
    assert_eq!(
        cwt(&[], &[1.0], WaveletType::Morlet),
        Err(WaveletError::EmptySignal)
    );

    assert!(matches!(
        idwt_1d(
            &[1.0; 4],
            WaveletType::Haar,
            1,
            WaveletBoundary::Periodic,
            (MAX_SAMPLES as u32) + 1
        ),
        Err(WaveletError::InvalidParam(_))
    ));
    assert!(matches!(
        cwt(&[1.0; 4], &[], WaveletType::Morlet),
        Err(WaveletError::InvalidParam(_))
    ));
    let too_many_scales = vec![1.0_f32; MAX_SCALES + 1];
    assert!(matches!(
        cwt(&[1.0; 4], &too_many_scales, WaveletType::Morlet),
        Err(WaveletError::InvalidParam(_))
    ));
    // Product scales.len() * signal.len() > MAX_SAMPLES rejected before Vec::with_capacity
    let sig_20k = vec![1.0_f32; 20_000];
    let scales_1000 = vec![1.0_f32; 1000];
    assert!(matches!(
        cwt(&sig_20k, &scales_1000, WaveletType::Morlet),
        Err(WaveletError::InvalidParam(_))
    ));

    // Oversized input slices rejected with InvalidParam.
    let oversized = vec![0.0_f32; MAX_SAMPLES + 1];
    assert!(matches!(
        dwt_1d(&oversized, WaveletType::Haar, 1, WaveletBoundary::Periodic),
        Err(WaveletError::InvalidParam(_))
    ));
    assert!(matches!(
        idwt_1d(&oversized, WaveletType::Haar, 1, WaveletBoundary::Periodic, 8),
        Err(WaveletError::InvalidParam(_))
    ));
    assert!(matches!(
        dwt_2d(&oversized, 1, 1, WaveletType::Haar, 1, WaveletBoundary::Periodic),
        Err(WaveletError::InvalidParam(_))
    ));
    assert!(matches!(
        idwt_2d(&oversized, 1, 1, WaveletType::Haar, 1, WaveletBoundary::Periodic),
        Err(WaveletError::InvalidParam(_))
    ));
    assert!(matches!(
        cwt(&oversized, &[1.0], WaveletType::Morlet),
        Err(WaveletError::InvalidParam(_))
    ));
}

#[test]
fn test_hostile_cwe_674_iterative_deep_pyramid() {
    let signal: Vec<f32> = (0..16384).map(|i| ((i as f32) * 0.03).sin()).collect();
    let coeffs = dwt_1d(&signal, WaveletType::Haar, 14, WaveletBoundary::Periodic)
        .expect("deep iterative dwt");
    let recon = idwt_1d(
        &coeffs,
        WaveletType::Haar,
        14,
        WaveletBoundary::Periodic,
        signal.len() as u32,
    )
    .expect("deep iterative idwt");
    assert_eq!(recon.len(), signal.len());
    for (a, b) in signal.iter().zip(&recon) {
        assert!((a - b).abs() < 1e-3);
    }
}
