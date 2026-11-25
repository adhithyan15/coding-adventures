//! Hostile input testing for CWE-20, CWE-190, CWE-400, and CWE-674 bounds (VER15).

use dsp_complex::{ComplexError, ComplexTensor, MAX_SAMPLES};

#[test]
fn test_hostile_cwe_20_non_finite_rejection() {
    for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(matches!(
            ComplexTensor::try_from_real(&[1.0, bad, 3.0]),
            Err(ComplexError::InvalidInput(_))
        ));
        assert!(matches!(
            ComplexTensor::from_real_imag(&[1.0, bad], &[0.0, 0.0]),
            Err(ComplexError::InvalidInput(_))
        ));
        assert!(matches!(
            ComplexTensor::from_real_imag(&[1.0, 2.0], &[0.0, bad]),
            Err(ComplexError::InvalidInput(_))
        ));
        assert!(matches!(
            ComplexTensor::from_interleaved(vec![1.0, 2.0, bad, 4.0]),
            Err(ComplexError::InvalidInput(_))
        ));
        assert!(matches!(
            ComplexTensor::from_interleaved(vec![1.0, 2.0, 3.0, bad]),
            Err(ComplexError::InvalidInput(_))
        ));
    }
}

#[test]
fn test_hostile_cwe_20_subnormals_and_extreme_magnitude_no_spurious_overflow() {
    // Subnormals must be accepted and preserve finite magnitude/phase without NaN.
    let sub = f32::MIN_POSITIVE / 4.0;
    let sub_t = ComplexTensor::from_real_imag(&[sub, -sub, 0.0], &[sub, 0.0, -0.0]).unwrap();
    let sub_mag = sub_t.magnitude();
    assert!(sub_mag.iter().all(|&m| m.is_finite() && m >= 0.0));
    assert!(sub_mag[0] > 0.0, "hypot must not underflow subnormal magnitude to 0");
    assert!(sub_t.phase().iter().all(|&p| p.is_finite()));

    // Large finite pairs whose squares exceed f32::MAX must not spuriously overflow to Inf.
    let large = 1.0e20_f32;
    let large_t = ComplexTensor::from_real_imag(&[large], &[large]).unwrap();
    let large_mag = large_t.magnitude();
    assert_eq!(large_mag.len(), 1);
    assert!(
        large_mag[0].is_finite(),
        "magnitude of (1e20, 1e20) must not overflow to Inf; got {}",
        large_mag[0]
    );
    let expected = large * std::f32::consts::SQRT_2;
    assert!((large_mag[0] - expected).abs() / expected < 1e-6);
}

#[test]
fn test_hostile_cwe_20_length_and_parity_validation() {
    assert_eq!(
        ComplexTensor::from_real_imag(&[1.0, 2.0], &[3.0]).unwrap_err(),
        ComplexError::LengthMismatch {
            real_len: 2,
            imag_len: 1,
        }
    );
    assert_eq!(
        ComplexTensor::from_interleaved(vec![1.0]).unwrap_err(),
        ComplexError::OddInterleavedLength(1)
    );
    assert_eq!(
        ComplexTensor::from_interleaved(vec![1.0, 2.0, 3.0]).unwrap_err(),
        ComplexError::OddInterleavedLength(3)
    );

    // Empty inputs are valid empty tensors of length 0.
    let empty = ComplexTensor::try_from_real(&[]).unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
}

#[test]
fn test_hostile_cwe_400_max_samples_cap() {
    let oversized_real = vec![0.0_f32; MAX_SAMPLES + 1];
    assert!(matches!(
        ComplexTensor::try_from_real(&oversized_real),
        Err(ComplexError::InvalidInput(_))
    ));
    let dummy_imag = vec![0.0_f32; MAX_SAMPLES + 1];
    assert!(matches!(
        ComplexTensor::from_real_imag(&oversized_real, &dummy_imag),
        Err(ComplexError::InvalidInput(_))
    ));
    drop(oversized_real);
    drop(dummy_imag);

    let oversized_interleaved = vec![0.0_f32; (MAX_SAMPLES + 1) * 2];
    assert!(matches!(
        ComplexTensor::from_interleaved(oversized_interleaved),
        Err(ComplexError::InvalidInput(_))
    ));
}
