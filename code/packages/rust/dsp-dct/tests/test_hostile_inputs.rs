//! Hostile input testing for CWE-20, CWE-190, CWE-400, and CWE-674 bounds (VER15).

use dsp_dct::{dct, dct_2d, idct, idct_2d, DctError, DctNorm, DctType, MAX_DCT_LEN};

#[test]
fn test_hostile_cwe_20_non_finite_rejection() {
    for &bad in &[f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(matches!(
            dct(&[1.0, bad, 3.0], DctType::II, DctNorm::Ortho),
            Err(DctError::InvalidInput(_))
        ));
        assert!(matches!(
            dct(&[1.0, bad, 3.0], DctType::III, DctNorm::None),
            Err(DctError::InvalidInput(_))
        ));
        assert!(matches!(
            idct(&[bad, 2.0], DctType::III, DctNorm::Ortho),
            Err(DctError::InvalidInput(_))
        ));
        assert!(matches!(
            idct(&[1.0, bad], DctType::II, DctNorm::None),
            Err(DctError::InvalidInput(_))
        ));
        assert!(matches!(
            dct_2d(&[1.0, bad, 3.0, 4.0], 2, 2, DctType::II, DctNorm::Ortho),
            Err(DctError::InvalidInput(_))
        ));
        assert!(matches!(
            idct_2d(&[1.0, 2.0, bad, 4.0], 2, 2, DctType::III, DctNorm::Ortho),
            Err(DctError::InvalidInput(_))
        ));
    }
}

#[test]
fn test_hostile_cwe_20_overflow_to_non_finite_rejected_and_ortho_n1_no_spurious_overflow() {
    // Contractive N=1 Ortho DCT-II / IDCT-III must NOT spuriously overflow on 2.0e38_f32.
    let big = [2.0e38_f32];
    let c = dct(&big, DctType::II, DctNorm::Ortho).unwrap();
    assert_eq!(c, big);
    let r = idct(&c, DctType::III, DctNorm::Ortho).unwrap();
    assert_eq!(r, big);

    // Genuine overflows fail closed with Err rather than leaking Inf/NaN.
    assert!(matches!(
        dct(&[f32::MAX], DctType::II, DctNorm::None),
        Err(DctError::InvalidInput(_))
    ));
    assert!(matches!(
        dct(&[f32::MAX; 4], DctType::II, DctNorm::Ortho),
        Err(DctError::InvalidInput(_) | DctError::Fft(_))
    ));
    assert!(matches!(
        dct_2d(&[f32::MAX; 4], 2, 2, DctType::II, DctNorm::None),
        Err(DctError::InvalidInput(_) | DctError::Fft(_))
    ));
}

#[test]
fn test_hostile_cwe_20_empty_and_subnormal_inputs() {
    assert_eq!(
        dct(&[], DctType::II, DctNorm::Ortho).unwrap_err(),
        DctError::EmptyInput
    );
    assert_eq!(
        idct(&[], DctType::III, DctNorm::Ortho).unwrap_err(),
        DctError::EmptyInput
    );
    // Warrant requires empty 2-D image with valid non-zero dims to return EmptyInput,
    // while zero dims return InvalidInput.
    assert_eq!(
        dct_2d(&[], 8, 8, DctType::II, DctNorm::Ortho).unwrap_err(),
        DctError::EmptyInput
    );
    assert_eq!(
        idct_2d(&[], 8, 8, DctType::III, DctNorm::Ortho).unwrap_err(),
        DctError::EmptyInput
    );
    assert!(matches!(
        dct_2d(&[], 0, 8, DctType::II, DctNorm::Ortho),
        Err(DctError::InvalidInput(_))
    ));
    assert!(matches!(
        dct_2d(&[], 8, 0, DctType::II, DctNorm::Ortho),
        Err(DctError::InvalidInput(_))
    ));
    assert!(matches!(
        idct_2d(&[1.0, 2.0], 2, 2, DctType::III, DctNorm::Ortho),
        Err(DctError::InvalidInput(_))
    ));

    // Subnormal inputs must stay finite without NaN.
    let sub = f32::MIN_POSITIVE / 4.0;
    let c = dct(&[sub, -sub, sub * 2.0, 0.0], DctType::II, DctNorm::Ortho).unwrap();
    assert!(c.iter().all(|&x| x.is_finite()));
    let r = idct(&c, DctType::III, DctNorm::Ortho).unwrap();
    assert!(r.iter().all(|&x| x.is_finite()));
}

#[test]
fn test_hostile_cwe_400_and_cwe_190_bounds() {
    let oversized_1d = vec![0.0_f32; MAX_DCT_LEN + 1];
    assert!(matches!(
        dct(&oversized_1d, DctType::II, DctNorm::Ortho),
        Err(DctError::InvalidInput(_))
    ));
    assert!(matches!(
        idct(&oversized_1d, DctType::III, DctNorm::Ortho),
        Err(DctError::InvalidInput(_))
    ));
    drop(oversized_1d);

    let img = vec![1.0, 2.0];
    assert!(matches!(
        dct_2d(&img, (MAX_DCT_LEN + 1) as u32, 2, DctType::II, DctNorm::Ortho),
        Err(DctError::InvalidInput(_))
    ));
    assert!(matches!(
        dct_2d(&img, 2, (MAX_DCT_LEN + 1) as u32, DctType::II, DctNorm::Ortho),
        Err(DctError::InvalidInput(_))
    ));
    assert!(matches!(
        dct_2d(&img, u32::MAX, u32::MAX, DctType::II, DctNorm::Ortho),
        Err(DctError::InvalidInput(_))
    ));
    assert!(matches!(
        idct_2d(&img, u32::MAX, u32::MAX, DctType::III, DctNorm::Ortho),
        Err(DctError::InvalidInput(_))
    ));
}
