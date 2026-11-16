//! # `dsp-conv` — same-size convolution and image filters
//!
//! **DSP04 Phases 1-5.**  Same-size 1-D, 2-D, and separable convolution
//! with four boundary modes (`Zero` / `Replicate` / `Reflect` / `Wrap`),
//! plus image filter design helpers (Gaussian, Sobel, box, Laplacian, sharpen).

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod kernels;
pub mod sep;
pub mod two_d;
pub use kernels::{
    box_blur_kernel, gaussian_blur_kernel, laplacian_kernel, sharpen_kernel,
    sobel_x_kernel, sobel_y_kernel,
};
pub use sep::sep_conv2d;
pub use two_d::conv2d;

use std::fmt;

/// Maximum signal / image samples (16,777,216 samples = 64 MB of f32).
pub const MAX_SAMPLES: usize = 1 << 24;

/// Maximum convolution kernel length (1,048,576 taps).
pub const MAX_KERNEL_LEN: usize = 1 << 20;

/// How to extend the signal past its `[0, N)` support during
/// convolution.
///
/// | Mode        | scipy.ndimage equivalent |
/// | ----------- | ------------------------ |
/// | `Zero`      | `mode='constant'`        |
/// | `Replicate` | `mode='nearest'`         |
/// | `Reflect`   | `mode='reflect'`         |
/// | `Wrap`      | `mode='wrap'`            |
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BoundaryMode {
    /// Pad with `0.0` outside `[0, N)`.
    Zero,
    /// Clamp to `[0, N - 1]` — boundary samples replicate.
    Replicate,
    /// Mirror about each boundary: index `-1` → `1`,
    /// index `N` → `N - 2`, etc.
    Reflect,
    /// Periodic / modular: `signal[j mod N]`, handling
    /// negative `j` correctly.
    Wrap,
}

/// Same-size 1-D convolution.  Output length equals
/// `signal.len()`.  The kernel is centred at each output
/// sample (centre = `kernel.len() / 2`, integer division;
/// for even `K` the upper centre is picked).
///
/// Boundary handling is controlled by `mode`.  See
/// [`BoundaryMode`] for the four supported extensions.
pub fn conv1d(
    signal: &[f32],
    kernel: &[f32],
    mode: BoundaryMode,
) -> Result<Vec<f32>, ConvError> {
    if signal.is_empty() {
        return Err(ConvError::EmptySignal);
    }
    if kernel.is_empty() {
        return Err(ConvError::EmptyKernel);
    }
    if signal.len() > MAX_SAMPLES {
        return Err(ConvError::InvalidInput(format!(
            "signal length {} exceeds MAX_SAMPLES {}",
            signal.len(),
            MAX_SAMPLES
        )));
    }
    if kernel.len() > MAX_KERNEL_LEN {
        return Err(ConvError::InvalidInput(format!(
            "kernel length {} exceeds MAX_KERNEL_LEN {}",
            kernel.len(),
            MAX_KERNEL_LEN
        )));
    }

    for &x in signal {
        if !x.is_finite() {
            return Err(ConvError::InvalidInput("non-finite float in signal".into()));
        }
    }
    for &h in kernel {
        if !h.is_finite() {
            return Err(ConvError::InvalidInput("non-finite float in kernel".into()));
        }
    }

    let n = signal.len();
    let k = kernel.len();
    let centre = (k / 2) as isize;

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut acc = 0.0f32;
        for (j, &tap) in kernel.iter().enumerate() {
            let src_index = (i as isize) + centre - (j as isize);
            let v = sample(signal, src_index, mode);
            acc += tap * v;
        }
        out.push(acc);
    }
    Ok(out)
}

/// Helper: read `signal` at a possibly-out-of-bounds index,
/// extending via the chosen `mode`.
fn sample(signal: &[f32], idx: isize, mode: BoundaryMode) -> f32 {
    match extend_index(idx, signal.len() as isize, mode) {
        Some(i) => signal[i],
        None => 0.0,
    }
}

/// Map a possibly-out-of-bounds index `idx` into a valid source index
/// in `[0, n)` per the boundary `mode`, or return `None` when the mode is
/// `Zero` and the index falls outside the support.
pub(crate) fn extend_index(idx: isize, n: isize, mode: BoundaryMode) -> Option<usize> {
    if idx >= 0 && idx < n {
        return Some(idx as usize);
    }
    match mode {
        BoundaryMode::Zero => None,
        BoundaryMode::Replicate => Some(idx.clamp(0, n - 1) as usize),
        BoundaryMode::Reflect => {
            if n == 1 {
                return Some(0);
            }
            let period = 2 * (n - 1);
            let mut m = idx.rem_euclid(period);
            if m >= n {
                m = 2 * (n - 1) - m;
            }
            Some(m as usize)
        }
        BoundaryMode::Wrap => Some(idx.rem_euclid(n) as usize),
    }
}

/// Errors produced by convolution primitives.
#[derive(Debug, Clone, PartialEq)]
pub enum ConvError {
    /// `signal` is empty.
    EmptySignal,
    /// `kernel` is empty.
    EmptyKernel,
    /// `image.len() != H · W` or dimension overflow.
    ImageSizeMismatch(String),
    /// Kernel bigger than image.
    KernelTooLarge(String),
    /// Non-finite float, input exceeding defensive caps, or invalid parameter.
    InvalidInput(String),
}

impl fmt::Display for ConvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvError::EmptySignal => write!(f, "convolution signal must be non-empty"),
            ConvError::EmptyKernel => write!(f, "convolution kernel must be non-empty"),
            ConvError::ImageSizeMismatch(msg) => write!(f, "image size mismatch: {}", msg),
            ConvError::KernelTooLarge(msg) => write!(f, "kernel too large: {}", msg),
            ConvError::InvalidInput(msg) => write!(f, "invalid input: {}", msg),
        }
    }
}

impl std::error::Error for ConvError {}

// ─────────────────────────── Tests ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_test_bootstrap::prng::SplitMix64;

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        let scale = a.abs().max(b.abs()).max(1.0);
        (a - b).abs() <= scale * tol
    }

    fn assert_close(a: &[f32], b: &[f32], tol: f32) {
        assert_eq!(a.len(), b.len(), "length mismatch");
        for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                approx_eq(*x, *y, tol),
                "mismatch at {}: {} vs {} (tol {})",
                i,
                x,
                y,
                tol
            );
        }
    }

    fn sample_f32(rng: &mut SplitMix64) -> f32 {
        let u = rng.next_u64();
        let mantissa = (u & 0x00ff_ffff) as f32 / (0x0100_0000 as f32);
        (mantissa * 2.0) - 1.0
    }

    #[test]
    fn conv1d_rejects_empty_signal() {
        let err = conv1d(&[], &[1.0, 2.0], BoundaryMode::Zero).unwrap_err();
        assert_eq!(err, ConvError::EmptySignal);
    }

    #[test]
    fn conv1d_rejects_empty_kernel() {
        let err = conv1d(&[1.0, 2.0], &[], BoundaryMode::Zero).unwrap_err();
        assert_eq!(err, ConvError::EmptyKernel);
    }

    #[test]
    fn conv1d_output_length_equals_signal_length() {
        for n in 1..=8 {
            for k in 1..=5 {
                let signal = vec![1.0f32; n];
                let kernel = vec![1.0f32; k];
                let out = conv1d(&signal, &kernel, BoundaryMode::Zero).unwrap();
                assert_eq!(out.len(), n);
            }
        }
    }

    #[test]
    fn conv1d_identity_kernel_returns_signal() {
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let kernel = vec![1.0f32];
        for mode in [
            BoundaryMode::Zero,
            BoundaryMode::Replicate,
            BoundaryMode::Reflect,
            BoundaryMode::Wrap,
        ] {
            let out = conv1d(&signal, &kernel, mode).unwrap();
            assert_close(&out, &signal, 1e-7);
        }
    }

    #[test]
    fn conv1d_centred_delta_preserves_signal() {
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let kernel = vec![0.0f32, 1.0, 0.0];
        let out = conv1d(&signal, &kernel, BoundaryMode::Zero).unwrap();
        assert_close(&out, &signal, 1e-7);
    }

    #[test]
    fn conv1d_zero_mode() {
        let signal = vec![1.0f32, 1.0, 1.0];
        let kernel = vec![1.0f32, 1.0, 1.0];
        let out = conv1d(&signal, &kernel, BoundaryMode::Zero).unwrap();
        assert_close(&out, &[2.0, 3.0, 2.0], 1e-7);
    }

    #[test]
    fn conv1d_replicate_mode() {
        let signal = vec![1.0f32, 2.0, 3.0];
        let kernel = vec![1.0f32, 1.0, 1.0];
        let out = conv1d(&signal, &kernel, BoundaryMode::Replicate).unwrap();
        assert_close(&out, &[4.0, 6.0, 8.0], 1e-7);
    }

    #[test]
    fn conv1d_replicate_constant_signal_passes_through() {
        let signal = vec![3.0f32; 8];
        let kernel = vec![0.25f32, 0.5, 0.25];
        let out = conv1d(&signal, &kernel, BoundaryMode::Replicate).unwrap();
        assert_close(&out, &signal, 1e-6);
    }

    #[test]
    fn conv1d_reflect_mode() {
        let signal = vec![10.0f32, 20.0, 30.0];
        let kernel = vec![1.0f32, 1.0, 1.0];
        let out = conv1d(&signal, &kernel, BoundaryMode::Reflect).unwrap();
        assert_close(&out, &[50.0, 60.0, 70.0], 1e-7);
    }

    #[test]
    fn conv1d_wrap_mode() {
        let signal = vec![1.0f32, 2.0, 3.0];
        let kernel = vec![1.0f32, 1.0, 1.0];
        let out = conv1d(&signal, &kernel, BoundaryMode::Wrap).unwrap();
        assert_close(&out, &[6.0, 6.0, 6.0], 1e-7);
    }

    #[test]
    fn conv1d_wrap_preserves_periodicity() {
        let signal = vec![1.0f32, 2.0, 3.0, 1.0, 2.0, 3.0];
        let kernel = vec![0.25f32, 0.5, 0.25];
        let out = conv1d(&signal, &kernel, BoundaryMode::Wrap).unwrap();
        assert_close(&out[..3], &out[3..], 1e-6);
    }

    #[test]
    fn conv1d_box_kernel_preserves_total_sum_wrap_mode() {
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let kernel = vec![1.0 / 3.0; 3];
        let out = conv1d(&signal, &kernel, BoundaryMode::Wrap).unwrap();
        let sum_in: f32 = signal.iter().sum();
        let sum_out: f32 = out.iter().sum();
        assert!(approx_eq(sum_in, sum_out, 1e-6));
    }

    #[test]
    fn conv1d_symmetric_kernel_symmetric_input() {
        let signal = vec![1.0f32, 3.0, 5.0, 3.0, 1.0];
        let kernel = vec![0.2f32, 0.6, 0.2];
        let out = conv1d(&signal, &kernel, BoundaryMode::Reflect).unwrap();
        let n = out.len();
        for i in 0..(n / 2) {
            assert!(approx_eq(out[i], out[n - 1 - i], 1e-6));
        }
    }

    #[test]
    fn conv1d_zero_matches_fir_centre_slice() {
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let kernel = vec![0.25f32, 0.5, 0.25];
        let via_conv1d = conv1d(&signal, &kernel, BoundaryMode::Zero).unwrap();
        let via_fir = dsp_filters::fir(&signal, &kernel).unwrap();
        assert_close(&via_conv1d, &via_fir[1..6], 1e-7);
    }

    #[test]
    fn conv1d_short_kernel_each_mode_compiles() {
        let signal = vec![1.0f32, 2.0];
        let kernel = vec![1.0f32];
        for mode in [
            BoundaryMode::Zero,
            BoundaryMode::Replicate,
            BoundaryMode::Reflect,
            BoundaryMode::Wrap,
        ] {
            let out = conv1d(&signal, &kernel, mode).unwrap();
            assert_eq!(out.len(), 2);
        }
    }

    #[test]
    fn hostile_non_finite_inputs_rejected() {
        assert!(matches!(
            conv1d(&[1.0, f32::NAN], &[1.0], BoundaryMode::Zero),
            Err(ConvError::InvalidInput(_))
        ));
        assert!(matches!(
            conv1d(&[1.0], &[f32::INFINITY], BoundaryMode::Zero),
            Err(ConvError::InvalidInput(_))
        ));
        assert!(matches!(
            conv2d(&[1.0, f32::NAN], &[1.0], 1, 2, 1, 1, BoundaryMode::Zero),
            Err(ConvError::InvalidInput(_))
        ));
        assert!(matches!(
            sep_conv2d(&[1.0, 2.0], &[f32::NEG_INFINITY], &[1.0], 1, 2, BoundaryMode::Zero),
            Err(ConvError::InvalidInput(_))
        ));
    }

    #[test]
    fn pbt_splitmix64_conv1d_properties() {
        let mut rng = SplitMix64::new(0xcafe_babe_0123_4567);
        let sizes = [1usize, 2, 3, 5, 8, 16, 33, 64];
        for &n in &sizes {
            let mut sig = Vec::with_capacity(n);
            for _ in 0..n {
                sig.push(sample_f32(&mut rng));
            }
            // Identity kernel
            let id_k = [1.0f32];
            let out = conv1d(&sig, &id_k, BoundaryMode::Replicate).unwrap();
            assert_close(&out, &sig, 1e-6);

            // Centered 3-tap delta
            let delta_k = [0.0f32, 1.0, 0.0];
            let out_d = conv1d(&sig, &delta_k, BoundaryMode::Wrap).unwrap();
            assert_close(&out_d, &sig, 1e-6);
        }
    }
}
