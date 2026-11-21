//! Hostile input testing for CWE-20, CWE-190, CWE-400, and CWE-674 bounds (VER15).

use dsp_complex::ComplexTensor;
use dsp_fft::{
    bluestein_scalar, bluestein_via_runtime, build_bluestein_graph_with_input, build_fft_graph,
    build_fft_graph_with_input, fft, fft_scalar, fft_via_runtime, ifft, ifft_scalar, irfft,
    irfft_scalar, rfft, rfft_scalar, wrap_real_as_complex, Direction, FftError, MAX_FFT_LEN,
};
use matrix_ir::{DType, GraphBuilder, Shape};

#[test]
fn test_hostile_cwe_20_non_finite_rejection() {
    for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(matches!(
            fft(&[1.0, bad, 3.0, 4.0], false),
            Err(FftError::InvalidInput(_))
        ));
        assert!(matches!(
            fft(&[1.0, 0.0, bad, 0.0], true),
            Err(FftError::InvalidInput(_))
        ));
        assert!(matches!(
            fft_scalar(&[1.0, 0.0, bad, 0.0]),
            Err(FftError::InvalidInput(_))
        ));
        assert!(matches!(
            ifft_scalar(&[1.0, 0.0, bad, 0.0]),
            Err(FftError::InvalidInput(_))
        ));
        assert!(matches!(
            bluestein_scalar(&[1.0, 0.0, bad, 0.0, 2.0, 0.0], Direction::Forward),
            Err(FftError::InvalidInput(_))
        ));
        assert!(matches!(
            build_fft_graph_with_input(&[1.0, bad], Direction::Forward),
            Err(FftError::InvalidInput(_))
        ));
        assert!(matches!(
            build_bluestein_graph_with_input(&[1.0, 0.0, bad, 0.0, 2.0, 0.0], Direction::Forward),
            Err(FftError::InvalidInput(_))
        ));
        assert!(matches!(
            rfft(&[1.0, bad, 3.0]),
            Err(FftError::InvalidInput(_))
        ));
        assert!(matches!(
            rfft_scalar(&[1.0, bad, 3.0]),
            Err(FftError::InvalidInput(_))
        ));
        assert!(matches!(
            irfft_scalar(&[1.0, bad, 2.0, 0.0], 2),
            Err(FftError::InvalidInput(_))
        ));
    }
}

#[test]
fn test_hostile_cwe_20_overflow_to_non_finite_rejected_without_panic() {
    // Finite extreme inputs whose butterfly/chirp sums exceed f32::MAX must return Err, never panic or leak Inf/NaN.
    let huge_real = [f32::MAX, f32::MAX, f32::MAX, f32::MAX];
    assert!(matches!(
        fft(&huge_real, false),
        Err(FftError::InvalidInput(_))
    ));
    assert!(matches!(
        fft_via_runtime(&huge_real, Direction::Forward),
        Err(FftError::InvalidInput(_))
    ));
    assert!(matches!(
        rfft(&huge_real),
        Err(FftError::InvalidInput(_))
    ));

    let huge_complex = [f32::MAX, 0.0, f32::MAX, 0.0];
    assert!(matches!(
        fft_scalar(&huge_complex),
        Err(FftError::InvalidInput(_))
    ));
    assert!(matches!(
        fft(&huge_complex, true),
        Err(FftError::InvalidInput(_))
    ));

    let huge_non_pow2 = [f32::MAX, 0.0, f32::MAX, 0.0, f32::MAX, 0.0];
    assert!(matches!(
        bluestein_scalar(&huge_non_pow2, Direction::Forward),
        Err(FftError::InvalidInput(_))
    ));
    assert!(matches!(
        bluestein_via_runtime(&huge_non_pow2, Direction::Forward),
        Err(FftError::InvalidInput(_))
    ));
}

#[test]
fn test_hostile_cwe_20_subnormals_and_empty_inputs() {
    // Empty inputs fail closed.
    assert!(matches!(fft(&[], false), Err(FftError::InvalidInput(_))));
    assert!(matches!(fft(&[], true), Err(FftError::InvalidInput(_))));
    assert!(matches!(fft_scalar(&[]), Err(FftError::InvalidInput(_))));
    assert!(matches!(ifft_scalar(&[]), Err(FftError::InvalidInput(_))));
    assert!(matches!(
        bluestein_scalar(&[], Direction::Forward),
        Err(FftError::InvalidInput(_))
    ));
    assert!(matches!(rfft(&[]), Err(FftError::InvalidInput(_))));
    assert!(matches!(irfft_scalar(&[], 0), Err(FftError::InvalidInput(_))));
    let empty_ct = ComplexTensor::try_from_real(&[]).unwrap();
    assert!(matches!(ifft(&empty_ct), Err(FftError::InvalidInput(_))));
    assert!(matches!(irfft(&empty_ct, 0), Err(FftError::InvalidInput(_))));

    // Odd interleaved buffer lengths fail closed.
    assert!(matches!(
        fft(&[1.0, 2.0, 3.0], true),
        Err(FftError::InvalidInput(_))
    ));
    assert!(matches!(
        fft_scalar(&[1.0, 2.0, 3.0]),
        Err(FftError::InvalidInput(_))
    ));

    // Subnormal inputs produce finite outputs without NaN.
    let sub = f32::MIN_POSITIVE / 4.0;
    let sub_sig = [sub, -sub, sub * 2.0, 0.0];
    let spec = fft(&sub_sig, false).unwrap();
    assert!(spec.as_interleaved().iter().all(|&x| x.is_finite()));
    let back = ifft(&spec).unwrap();
    assert!(back.as_interleaved().iter().all(|&x| x.is_finite()));
}

#[test]
fn test_hostile_cwe_400_and_cwe_190_bounds() {
    assert!(matches!(
        irfft_scalar(&[1.0, 0.0], (MAX_FFT_LEN + 1) as u32),
        Err(FftError::InvalidInput(_))
    ));
    assert!(matches!(
        build_fft_graph((MAX_FFT_LEN * 2) as u32, Direction::Forward),
        Err(FftError::InvalidInput(_))
    ));
    assert!(matches!(
        build_fft_graph(0, Direction::Forward),
        Err(FftError::NotPowerOfTwo(0))
    ));
    assert!(matches!(
        build_fft_graph(3, Direction::Forward),
        Err(FftError::NotPowerOfTwo(3))
    ));

    // Bluestein convolution length > MAX_FFT_LEN rejected before allocation.
    let n_over = (MAX_FFT_LEN / 2) + 1;
    let sig_real = vec![0.0_f32; n_over];
    assert!(matches!(
        fft(&sig_real, false),
        Err(FftError::InvalidInput(_))
    ));
    assert!(matches!(
        rfft(&sig_real),
        Err(FftError::InvalidInput(_))
    ));
    assert!(matches!(
        rfft_scalar(&sig_real),
        Err(FftError::InvalidInput(_))
    ));
    drop(sig_real);

    let half_over = vec![0.0_f32; (n_over / 2 + 1) * 2];
    assert!(matches!(
        irfft_scalar(&half_over, n_over as u32),
        Err(FftError::InvalidInput(_))
    ));
    drop(half_over);

    let sig_complex = vec![0.0_f32; 2 * n_over];
    assert!(matches!(
        bluestein_scalar(&sig_complex, Direction::Forward),
        Err(FftError::InvalidInput(_))
    ));
    assert!(matches!(
        build_bluestein_graph_with_input(&sig_complex, Direction::Forward),
        Err(FftError::InvalidInput(_))
    ));
}

#[test]
#[should_panic(expected = "exceeds MAX_FFT_LEN")]
fn test_hostile_wrap_real_as_complex_rejects_u32_max_without_oom() {
    let mut b = GraphBuilder::new();
    let t = b.input(DType::F32, Shape::from(&[4]));
    let _ = wrap_real_as_complex(&mut b, &t, u32::MAX);
}
