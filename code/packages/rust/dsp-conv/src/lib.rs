//! # `dsp-conv` — same-size convolution and image filters
//!
//! **DSP04 Phase 1 + 2 (this release).**  Same-size 1-D
//! convolution with four boundary modes
//! (`Zero` / `Replicate` / `Reflect` / `Wrap`).  Output length
//! equals input length; the kernel is centred (centre =
//! `K / 2` with integer division — matches
//! `scipy.ndimage.convolve`).
//!
//! ## Quick example
//!
//! ```rust
//! use dsp_conv::{conv1d, BoundaryMode};
//!
//! let signal = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
//! let kernel = vec![0.25_f32, 0.5, 0.25];
//! let out = conv1d(&signal, &kernel, BoundaryMode::Reflect).unwrap();
//! assert_eq!(out.len(), signal.len());
//! ```
//!
//! ## How this differs from `dsp-filters::fir`
//!
//! `dsp-filters::fir(signal, kernel)` returns the full
//! linear convolution of length `N + K - 1` with implicit
//! zero padding past the signal's support — the standard
//! signal-processing convention.
//!
//! `dsp-conv::conv1d` returns a length-`N` output where the
//! kernel is centred at each output sample, and lets you
//! choose how to extend the signal at the boundaries.  That's
//! what image-processing code (and `scipy.ndimage`) usually
//! wants.
//!
//! ## Phase scope
//!
//! - **Phase 0** — spec (`code/specs/DSP04-convolution.md`).
//! - **Phase 1+2** — crate skeleton + scalar `conv1d` with
//!   4 boundary modes.  Landed as 0.1.0.
//! - **Phase 3 (this release)** — scalar [`conv2d`] for
//!   `[H, W]` images.  Row-major, same-size output, boundary
//!   extension applied independently along each axis.
//! - **Phase 4** — `sep_conv2d` (separable 2-D conv — much
//!   faster for blurs / Gaussians where the kernel factors).
//! - **Phase 5** — image filter design helpers (Gaussian,
//!   Sobel, box, Laplacian, sharpen).
//! - **Phase 6** — matrix-ir-lowered `conv1d` / `conv2d`.

#![warn(rust_2018_idioms)]

pub mod two_d;
pub use two_d::conv2d;

use std::fmt;

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
    let n = signal.len();
    let k = kernel.len();
    let centre = (k / 2) as isize;

    // For each output index `i`, sum kernel[j] · signal_ext[i + centre - j].
    //
    // The convolution definition uses `signal[i - j]` for
    // standard linear convolution where the kernel slides
    // left-to-right.  We adopt the *centred* convention used
    // by image-processing tools: the kernel's centre tap
    // aligns with output index `i`, so the source index is
    // `i + centre - j`.
    //
    // (For symmetric kernels — which is most filter design
    // outputs — this matches the standard convolution result
    // for the inner samples and only differs in how
    // boundaries are extended.)
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut acc = 0.0f32;
        for j in 0..k {
            let src_index = (i as isize) + centre - (j as isize);
            let v = sample(signal, src_index, mode);
            acc += kernel[j] * v;
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

/// **Phase 3 helper.**  Map a possibly-out-of-bounds index `idx`
/// into a valid source index in `[0, n)` per the boundary `mode`,
/// or return `None` when the mode is `Zero` and the index falls
/// outside the support (i.e. the convolution should accumulate
/// `0` for that tap).
///
/// `n` must be `≥ 1`.  This is used by both the 1-D `sample`
/// here and the 2-D conv2d in the [`crate::two_d`] module, which
/// applies the extension along each axis independently.
pub(crate) fn extend_index(idx: isize, n: isize, mode: BoundaryMode) -> Option<usize> {
    if idx >= 0 && idx < n {
        return Some(idx as usize);
    }
    match mode {
        BoundaryMode::Zero => None,
        BoundaryMode::Replicate => Some(idx.clamp(0, n - 1) as usize),
        BoundaryMode::Reflect => {
            // Mirror about both boundaries.  Works for arbitrarily
            // far indices via the "fold into [-N+1, N) twice"
            // formulation:
            //
            //   period = 2 · (N - 1)  (one full reflection cycle)
            //   m = ((idx mod period) + period) mod period
            //   reflected = m if m < N else (2(N-1) - m)
            //
            // For N = 1 the period collapses; we special-case it.
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
        BoundaryMode::Wrap => {
            // Modular index — `rem_euclid` handles negatives
            // correctly.
            Some(idx.rem_euclid(n) as usize)
        }
    }
}

/// Errors produced by convolution primitives.
///
/// `ImageSizeMismatch` and `KernelTooLarge` are reserved for
/// the Phase 3 `conv2d` API; Phase 2 only ever returns
/// `EmptySignal` or `EmptyKernel`.
#[derive(Debug, Clone, PartialEq)]
pub enum ConvError {
    /// `signal` is empty.
    EmptySignal,
    /// `kernel` is empty.
    EmptyKernel,
    /// Reserved for `conv2d` (Phase 3): `image.len() != H · W`.
    ImageSizeMismatch(String),
    /// Reserved for `conv2d` (Phase 3): kernel bigger than image.
    KernelTooLarge(String),
}

impl fmt::Display for ConvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvError::EmptySignal => write!(f, "convolution signal must be non-empty"),
            ConvError::EmptyKernel => write!(f, "convolution kernel must be non-empty"),
            ConvError::ImageSizeMismatch(msg) => write!(f, "image size mismatch: {}", msg),
            ConvError::KernelTooLarge(msg) => write!(f, "kernel too large: {}", msg),
        }
    }
}

impl std::error::Error for ConvError {}

// ─────────────────────────── Tests ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    // ── error paths ────────────────────────────────────────────

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

    // ── closed-form ────────────────────────────────────────────

    #[test]
    fn conv1d_identity_kernel_returns_signal() {
        // K = 1, kernel = [1.0]: centre = 0, so each output is
        // just signal[i].  All four boundary modes give the same
        // answer since we never reach past the boundary.
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        for &mode in &[
            BoundaryMode::Zero,
            BoundaryMode::Replicate,
            BoundaryMode::Reflect,
            BoundaryMode::Wrap,
        ] {
            let out = conv1d(&signal, &[1.0], mode).unwrap();
            assert_close(&out, &signal, 1e-7);
        }
    }

    #[test]
    fn conv1d_centred_delta_preserves_signal() {
        // K = 3, kernel = [0, 1, 0]: centre = 1.  For each i,
        // out[i] = signal[i + 1 - 0]·0 + signal[i + 1 - 1]·1 +
        //         signal[i + 1 - 2]·0 = signal[i].
        // So a centred delta is the identity in conv terms,
        // regardless of boundary mode.
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let kernel = vec![0.0f32, 1.0, 0.0];
        for &mode in &[
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
    fn conv1d_output_length_equals_signal_length() {
        for n in [1usize, 5, 17, 100] {
            let signal: Vec<f32> = (0..n).map(|i| (i as f32) * 0.1).collect();
            for k in [1usize, 3, 5, 11] {
                let kernel: Vec<f32> = (0..k).map(|_| 1.0 / k as f32).collect();
                let out = conv1d(&signal, &kernel, BoundaryMode::Reflect).unwrap();
                assert_eq!(out.len(), n, "N={}, K={}", n, k);
            }
        }
    }

    // ── boundary modes ─────────────────────────────────────────
    //
    // We use signal = [1, 2, 3, 4, 5], kernel = [1, 1, 1].
    // Centre = 1.  For each output index i:
    //   out[i] = signal_ext[i+1] + signal_ext[i] + signal_ext[i-1]
    //
    // (The kernel is symmetric so the centring direction doesn't
    // matter; we read 3 consecutive samples around i.)

    fn handwritten_signal() -> Vec<f32> {
        vec![1.0, 2.0, 3.0, 4.0, 5.0]
    }

    fn handwritten_kernel() -> Vec<f32> {
        vec![1.0, 1.0, 1.0]
    }

    #[test]
    fn conv1d_zero_mode() {
        // signal_ext at -1 = 0, at 5 = 0.
        //   out[0] = 0 + 1 + 2 = 3       (signal[-1] + signal[0] + signal[1])
        //   out[1] = 1 + 2 + 3 = 6
        //   out[2] = 2 + 3 + 4 = 9
        //   out[3] = 3 + 4 + 5 = 12
        //   out[4] = 4 + 5 + 0 = 9       (signal[3] + signal[4] + signal[5])
        let signal = handwritten_signal();
        let kernel = handwritten_kernel();
        let out = conv1d(&signal, &kernel, BoundaryMode::Zero).unwrap();
        assert_close(&out, &[3.0, 6.0, 9.0, 12.0, 9.0], 1e-7);
    }

    #[test]
    fn conv1d_replicate_mode() {
        // signal_ext at -1 = signal[0] = 1, at 5 = signal[4] = 5.
        //   out[0] = 1 + 1 + 2 = 4
        //   out[1] = 1 + 2 + 3 = 6
        //   out[2] = 2 + 3 + 4 = 9
        //   out[3] = 3 + 4 + 5 = 12
        //   out[4] = 4 + 5 + 5 = 14
        let signal = handwritten_signal();
        let kernel = handwritten_kernel();
        let out = conv1d(&signal, &kernel, BoundaryMode::Replicate).unwrap();
        assert_close(&out, &[4.0, 6.0, 9.0, 12.0, 14.0], 1e-7);
    }

    #[test]
    fn conv1d_reflect_mode() {
        // signal_ext at -1 = signal[1] = 2, at 5 = signal[3] = 4.
        //   out[0] = 2 + 1 + 2 = 5
        //   out[1] = 1 + 2 + 3 = 6
        //   out[2] = 2 + 3 + 4 = 9
        //   out[3] = 3 + 4 + 5 = 12
        //   out[4] = 4 + 5 + 4 = 13
        let signal = handwritten_signal();
        let kernel = handwritten_kernel();
        let out = conv1d(&signal, &kernel, BoundaryMode::Reflect).unwrap();
        assert_close(&out, &[5.0, 6.0, 9.0, 12.0, 13.0], 1e-7);
    }

    #[test]
    fn conv1d_wrap_mode() {
        // signal_ext at -1 = signal[4] = 5, at 5 = signal[0] = 1.
        //   out[0] = 5 + 1 + 2 = 8
        //   out[1] = 1 + 2 + 3 = 6
        //   out[2] = 2 + 3 + 4 = 9
        //   out[3] = 3 + 4 + 5 = 12
        //   out[4] = 4 + 5 + 1 = 10
        let signal = handwritten_signal();
        let kernel = handwritten_kernel();
        let out = conv1d(&signal, &kernel, BoundaryMode::Wrap).unwrap();
        assert_close(&out, &[8.0, 6.0, 9.0, 12.0, 10.0], 1e-7);
    }

    // ── cross-checks ───────────────────────────────────────────

    #[test]
    fn conv1d_zero_matches_fir_centre_slice() {
        // For a 3-tap kernel, dsp_filters::fir returns length
        // N + 3 - 1 = N + 2.  The centre slice [(K-1)/2 ..
        // (K-1)/2 + N] = [1 .. N+1] should match conv1d under
        // Zero mode.
        let signal: Vec<f32> = (0..20).map(|i| ((i as f32) * 0.3).sin()).collect();
        let kernel = vec![0.25f32, 0.5, 0.25];
        let via_conv = conv1d(&signal, &kernel, BoundaryMode::Zero).unwrap();
        let via_fir = dsp_filters::fir(&signal, &kernel).unwrap();
        // Centre slice: skip the first (K-1)/2 = 1 sample.
        let centre = &via_fir[1..1 + signal.len()];
        assert_close(&via_conv, centre, 1e-6);
    }

    #[test]
    fn conv1d_wrap_preserves_periodicity() {
        // For a periodic signal whose period evenly divides N,
        // wrap-mode convolution is also periodic with the same
        // period.  Take signal = [1, 2, 1, 2, 1, 2] (period 2,
        // N = 6).  Output should also have period 2.
        let signal = vec![1.0f32, 2.0, 1.0, 2.0, 1.0, 2.0];
        let kernel = vec![0.5f32, 0.5];
        let out = conv1d(&signal, &kernel, BoundaryMode::Wrap).unwrap();
        // Verify period-2 structure.
        for i in 0..(out.len() - 2) {
            assert!(
                approx_eq(out[i], out[i + 2], 1e-6),
                "period-2 violation at {}: {} vs {}",
                i,
                out[i],
                out[i + 2]
            );
        }
    }

    #[test]
    fn conv1d_replicate_constant_signal_passes_through() {
        // A constant signal convolved with a normalised kernel
        // under Replicate mode should yield the same constant
        // (no edge artefacts from zero-padding).
        let signal = vec![3.5f32; 10];
        let kernel = vec![0.25f32, 0.5, 0.25];
        let out = conv1d(&signal, &kernel, BoundaryMode::Replicate).unwrap();
        for &v in &out {
            assert!(
                approx_eq(v, 3.5, 1e-5),
                "constant signal yielded {}, expected 3.5",
                v
            );
        }
    }

    // ── symmetry / integral checks ─────────────────────────────

    #[test]
    fn conv1d_symmetric_kernel_symmetric_input() {
        // Symmetric odd input + symmetric kernel under Reflect:
        // the output should also be symmetric about its centre.
        let signal = vec![1.0f32, 2.0, 4.0, 2.0, 1.0]; // symmetric
        let kernel = vec![0.25f32, 0.5, 0.25]; // symmetric
        let out = conv1d(&signal, &kernel, BoundaryMode::Reflect).unwrap();
        let n = out.len();
        for i in 0..(n / 2) {
            assert!(
                approx_eq(out[i], out[n - 1 - i], 1e-6),
                "asymmetry at i={}: {} vs {}",
                i,
                out[i],
                out[n - 1 - i]
            );
        }
    }

    #[test]
    fn conv1d_box_kernel_preserves_total_sum_wrap_mode() {
        // Wrap-mode convolution with a normalised box kernel
        // (sum = 1) preserves the total sum of the input (since
        // each input sample contributes exactly once to the sum
        // of the output).
        let signal: Vec<f32> = (0..16).map(|i| (i as f32) - 7.5).collect();
        let kernel = vec![1.0 / 3.0; 3];
        let out = conv1d(&signal, &kernel, BoundaryMode::Wrap).unwrap();
        let sig_sum: f32 = signal.iter().sum();
        let out_sum: f32 = out.iter().sum();
        assert!(
            approx_eq(sig_sum, out_sum, 1e-5),
            "sig_sum {} vs out_sum {}",
            sig_sum,
            out_sum
        );
    }

    #[test]
    fn conv1d_short_kernel_each_mode_compiles() {
        // Quick smoke test: K = 1 under each mode.  Should
        // produce the input unchanged.
        let signal = vec![1.0f32, 2.0, 3.0];
        for &mode in &[
            BoundaryMode::Zero,
            BoundaryMode::Replicate,
            BoundaryMode::Reflect,
            BoundaryMode::Wrap,
        ] {
            let out = conv1d(&signal, &[1.0], mode).unwrap();
            assert_close(&out, &signal, 1e-7);
        }
    }
}
