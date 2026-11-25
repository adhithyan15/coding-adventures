//! Hostile input testing for CWE-20, CWE-190, CWE-400, and CWE-674 in `dsp-conv` (VER15).

use dsp_conv::{
    conv1d, conv2d, sep_conv2d, try_box_blur_kernel, try_gaussian_blur_kernel, try_sharpen_kernel,
    BoundaryMode, ConvError, MAX_KERNEL_LEN, MAX_SAMPLES,
};

#[test]
fn test_hostile_cwe_20_non_finite_inputs_rejected() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(matches!(
            conv1d(&[1.0, bad, 2.0], &[1.0], BoundaryMode::Zero),
            Err(ConvError::InvalidInput(_))
        ));
        assert!(matches!(
            conv1d(&[1.0, 2.0], &[bad], BoundaryMode::Zero),
            Err(ConvError::InvalidInput(_))
        ));
        assert!(matches!(
            conv2d(&[1.0, bad, 2.0, 3.0], &[1.0], 2, 2, 1, 1, BoundaryMode::Zero),
            Err(ConvError::InvalidInput(_))
        ));
        assert!(matches!(
            conv2d(&[1.0, 2.0, 3.0, 4.0], &[bad], 2, 2, 1, 1, BoundaryMode::Zero),
            Err(ConvError::InvalidInput(_))
        ));
        assert!(matches!(
            sep_conv2d(&[1.0, bad, 2.0, 3.0], &[1.0], &[1.0], 2, 2, BoundaryMode::Zero),
            Err(ConvError::InvalidInput(_))
        ));
        assert!(matches!(
            sep_conv2d(&[1.0, 2.0, 3.0, 4.0], &[bad], &[1.0], 2, 2, BoundaryMode::Zero),
            Err(ConvError::InvalidInput(_))
        ));
        assert!(matches!(
            sep_conv2d(&[1.0, 2.0, 3.0, 4.0], &[1.0], &[bad], 2, 2, BoundaryMode::Zero),
            Err(ConvError::InvalidInput(_))
        ));
        assert!(matches!(
            try_gaussian_blur_kernel(bad, 3),
            Err(ConvError::InvalidInput(_))
        ));
        assert!(matches!(
            try_sharpen_kernel(bad),
            Err(ConvError::InvalidInput(_))
        ));
    }
}

#[test]
fn test_hostile_cwe_20_subnormal_and_overflow_kernels() {
    // Subnormal / tiny sigma underflows 2*sigma^2 to 0 or overflows 1/(sigma*sqrt(2*pi)) -> NaN if unchecked.
    for tiny_sigma in [0.0_f32, -0.0, -1.0, 1e-45_f32, 1e-40_f32, f32::MIN_POSITIVE, 1e-25_f32] {
        assert!(
            matches!(
                try_gaussian_blur_kernel(tiny_sigma, 5),
                Err(ConvError::InvalidInput(_))
            ),
            "tiny/non-positive sigma {tiny_sigma:e} must be rejected"
        );
    }
    // Huge sigma overflows 2*sigma^2 to Inf.
    assert!(matches!(
        try_gaussian_blur_kernel(f32::MAX, 5),
        Err(ConvError::InvalidInput(_))
    ));
    // Even or zero kernel sizes rejected.
    for bad_size in [0u32, 2, 4, 100] {
        assert!(matches!(
            try_gaussian_blur_kernel(1.0, bad_size),
            Err(ConvError::InvalidInput(_))
        ));
    }
    assert!(matches!(
        try_box_blur_kernel(0),
        Err(ConvError::InvalidInput(_))
    ));

    // Sharpen kernel overflow: 1.0 + 4.0 * amount overflows f32 for large finite amount.
    for big_amount in [1e38_f32, -1e38_f32, f32::MAX, f32::MIN] {
        assert!(
            matches!(
                try_sharpen_kernel(big_amount),
                Err(ConvError::InvalidInput(_))
            ),
            "overflowing sharpen amount {big_amount:e} must be rejected"
        );
    }
}

#[test]
fn test_hostile_cwe_20_finite_output_overflow_rejected() {
    // Finite inputs whose convolution sum overflows f32 to Inf or +Inf + -Inf = NaN.
    assert!(matches!(
        conv1d(&[f32::MAX, f32::MAX], &[2.0, 2.0], BoundaryMode::Replicate),
        Err(ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        conv1d(&[f32::MAX, -f32::MAX], &[2.0, 2.0], BoundaryMode::Wrap),
        Err(ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        conv2d(
            &[f32::MAX, f32::MAX, f32::MAX, f32::MAX],
            &[2.0],
            2,
            2,
            1,
            1,
            BoundaryMode::Zero
        ),
        Err(ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        sep_conv2d(
            &[f32::MAX, f32::MAX, f32::MAX, f32::MAX],
            &[2.0],
            &[2.0],
            2,
            2,
            BoundaryMode::Zero
        ),
        Err(ConvError::InvalidInput(_))
    ));
}

#[test]
fn test_hostile_cwe_190_dimension_overflow_and_mismatches() {
    // Dimension product overflow (H * W > usize::MAX or > MAX_SAMPLES) rejected before any allocation.
    assert!(matches!(
        conv2d(&[1.0], &[1.0], u32::MAX, u32::MAX, 1, 1, BoundaryMode::Zero),
        Err(ConvError::ImageSizeMismatch(_) | ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        conv2d(&[1.0], &[1.0], 1, 1, u32::MAX, u32::MAX, BoundaryMode::Zero),
        Err(ConvError::ImageSizeMismatch(_) | ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        sep_conv2d(&[1.0], &[1.0], &[1.0], u32::MAX, u32::MAX, BoundaryMode::Zero),
        Err(ConvError::ImageSizeMismatch(_) | ConvError::InvalidInput(_))
    ));

    // Zero dimensions, empty kernel/image warrant parity, and length mismatches.
    assert!(matches!(
        conv2d(&[], &[1.0], 0, 4, 1, 1, BoundaryMode::Zero),
        Err(ConvError::ImageSizeMismatch(_))
    ));
    assert!(matches!(
        conv2d(&[], &[1.0], 5000, 5000, 1, 1, BoundaryMode::Zero),
        Err(ConvError::ImageSizeMismatch(_))
    ));
    assert!(matches!(
        sep_conv2d(&[], &[1.0], &[1.0], 5000, 5000, BoundaryMode::Zero),
        Err(ConvError::ImageSizeMismatch(_))
    ));
    assert_eq!(
        conv2d(&[1.0; 9], &[], 3, 3, 3, 3, BoundaryMode::Zero),
        Err(ConvError::EmptyKernel)
    );
    assert!(matches!(
        conv2d(&[1.0; 5], &[1.0], 2, 3, 1, 1, BoundaryMode::Zero),
        Err(ConvError::ImageSizeMismatch(_))
    ));
    assert!(matches!(
        conv2d(&[1.0; 4], &[1.0; 3], 2, 2, 2, 2, BoundaryMode::Zero),
        Err(ConvError::ImageSizeMismatch(_))
    ));
    assert!(matches!(
        sep_conv2d(&[1.0; 5], &[1.0], &[1.0], 2, 3, BoundaryMode::Zero),
        Err(ConvError::ImageSizeMismatch(_))
    ));
    assert!(matches!(
        conv2d(&[1.0; 4], &[1.0; 9], 2, 2, 3, 3, BoundaryMode::Zero),
        Err(ConvError::KernelTooLarge(_))
    ));
}

#[test]
fn test_hostile_cwe_400_caps_enforced_before_allocation() {
    assert!(matches!(
        try_gaussian_blur_kernel(1.0, (MAX_KERNEL_LEN as u32) + 1),
        Err(ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        try_box_blur_kernel((MAX_KERNEL_LEN as u32) + 1),
        Err(ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        conv2d(&[1.0; 4], &[1.0], 10_000, 10_000, 1, 1, BoundaryMode::Zero),
        Err(ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        sep_conv2d(&[1.0; 4], &[1.0], &[1.0], 10_000, 10_000, BoundaryMode::Zero),
        Err(ConvError::InvalidInput(_))
    ));

    // Oversized kernel slice rejected with InvalidInput.
    let big_kernel = vec![0.0_f32; MAX_KERNEL_LEN + 1];
    assert!(matches!(
        conv1d(&[1.0], &big_kernel, BoundaryMode::Zero),
        Err(ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        conv2d(&[1.0], &big_kernel, 1, 1, 1, 1, BoundaryMode::Zero),
        Err(ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        sep_conv2d(&[1.0], &big_kernel, &[1.0], 1, 1, BoundaryMode::Zero),
        Err(ConvError::InvalidInput(_))
    ));

    // Oversized signal slice rejected with InvalidInput.
    let big_signal = vec![0.0_f32; MAX_SAMPLES + 1];
    assert!(matches!(
        conv1d(&big_signal, &[1.0], BoundaryMode::Zero),
        Err(ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        conv2d(&big_signal, &[1.0], 1, 1, 1, 1, BoundaryMode::Zero),
        Err(ConvError::InvalidInput(_))
    ));
    assert!(matches!(
        sep_conv2d(&big_signal, &[1.0], &[1.0], 1, 1, BoundaryMode::Zero),
        Err(ConvError::InvalidInput(_))
    ));
}

#[test]
fn test_hostile_cwe_674_iterative_large_kernel_and_signal() {
    let signal = vec![0.25_f32; 8192];
    let kernel = vec![1.0 / 257.0; 257];
    let out = conv1d(&signal, &kernel, BoundaryMode::Replicate).expect("iterative conv1d");
    assert_eq!(out.len(), 8192);
    assert!(out.iter().all(|x| x.is_finite()));
}
