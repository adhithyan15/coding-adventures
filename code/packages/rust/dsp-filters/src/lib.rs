//! # `dsp-filters` — FIR / IIR filters for the DSP layer
//!
//! **DSP03 Phase 1 + 2 (this release).**  Pure-Rust scalar
//! reference for FIR (finite impulse response) filtering via
//! direct linear convolution.  Phase 3 will add an FFT-based
//! overlap-add path for long kernels; Phase 4 adds IIR
//! (direct-form-II Transposed); Phase 5 adds the canonical
//! filter design helpers (Butterworth, Chebyshev,
//! windowed-sinc).
//!
//! ## Quick example
//!
//! ```rust
//! use dsp_filters::fir;
//!
//! let signal = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0];
//! let kernel = vec![0.25_f32, 0.5, 0.25];   // 3-tap low-pass
//! let smoothed = fir(&signal, &kernel).unwrap();
//! assert_eq!(smoothed.len(), signal.len() + kernel.len() - 1);
//! ```
//!
//! ## Algorithm
//!
//! Direct linear convolution:
//!
//! ```text
//!     y[n] = Σ_{k=0..K-1}  kernel[k] · signal[n - k]
//! ```
//!
//! Output length is `N + K - 1` (the full convolution; matches
//! `numpy.convolve(signal, kernel, mode='full')`).  Boundary
//! handling: input is implicitly zero-padded outside `[0, N)`.
//!
//! `O(N · K)` time, `O(N + K)` memory.  For long kernels
//! (`K > ~64`), Phase 3 will add an FFT-based overlap-add
//! implementation that's `O((N + K) · log(N + K))`.
//!
//! ## Phase scope
//!
//! - **Phase 0** — spec (`code/specs/DSP03-filters.md`).
//! - **Phase 1+2 (this release)** — crate skeleton + scalar
//!   FIR + tests.
//! - **Phase 3** — `fir_fft` (FFT-based overlap-add).
//! - **Phase 4** — `iir` (direct-form-II Transposed).
//! - **Phase 5** — filter design helpers.
//! - **Phase 6** — matrix-ir-lowered `fir_via_runtime`.

#![warn(rust_2018_idioms)]

use std::fmt;

/// Finite-impulse-response filter via direct linear convolution.
///
/// `signal` is a length-`N` real signal; `kernel` is a length-`K`
/// real impulse response.  Returns a `Vec<f32>` of length
/// `N + K - 1` holding the full linear convolution
/// (`numpy.convolve(signal, kernel, mode='full')`).
///
/// Boundary handling: the input is implicitly zero-padded outside
/// `[0, N)`.  The output's `n`-th sample sums all kernel taps that
/// land within `signal`'s support.
///
/// Returns [`FilterError::EmptySignal`] or
/// [`FilterError::EmptyKernel`] when either input is empty.
pub fn fir(signal: &[f32], kernel: &[f32]) -> Result<Vec<f32>, FilterError> {
    if signal.is_empty() {
        return Err(FilterError::EmptySignal);
    }
    if kernel.is_empty() {
        return Err(FilterError::EmptyKernel);
    }
    let n = signal.len();
    let k = kernel.len();
    let out_len = n + k - 1;
    let mut out = vec![0.0f32; out_len];

    // Direct convolution.  For each output sample `i`, sum the
    // `kernel[j]` taps over the `j` for which `i - j` lands inside
    // `[0, n)`.  Equivalent to (and faster than) the textbook
    // sliding-window form when expressed as the inner-product over
    // `j ∈ [j_lo, j_hi)` with explicitly-clamped bounds.
    //
    //   - For `i < k`, `j_lo = 0` (kernel head is bounded by zero).
    //     `j_hi = min(i + 1, k)` (kernel can't reach past index 0
    //     of `signal` from the left).
    //   - For `i >= k - 1`, `j_lo = max(0, i - n + 1)` (kernel
    //     can't reach past the right edge of `signal`).
    //     `j_hi = k`.
    //
    // The combined formula collapses to:
    //   `j_lo = max(0, i + 1 - n)`,  `j_hi = min(k, i + 1)`.
    for i in 0..out_len {
        let j_lo = if i + 1 > n { i + 1 - n } else { 0 };
        let j_hi = if i + 1 < k { i + 1 } else { k };
        let mut acc = 0.0f32;
        for j in j_lo..j_hi {
            // signal index = i - j, guaranteed in [0, n) by the
            // bounds above.  kernel index = j, in [0, k).
            acc += kernel[j] * signal[i - j];
        }
        out[i] = acc;
    }
    Ok(out)
}

/// Errors produced by filter primitives.
///
/// `InvalidCoefficient` and `Fft` variants are reserved for IIR
/// (Phase 4) and FFT-based FIR (Phase 3) respectively; Phase 2
/// only ever returns `EmptySignal` or `EmptyKernel`.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterError {
    /// `signal` is empty.  V1 requires `N ≥ 1`.
    EmptySignal,
    /// `kernel` is empty.  V1 requires `K ≥ 1`.
    EmptyKernel,
    /// Reserved for IIR (Phase 4): `a[0] == 0`, NaN coefficients,
    /// length mismatch between `b` and `a`, etc.
    InvalidCoefficient(String),
    /// Reserved for FIR-via-FFT (Phase 3): wraps a
    /// `dsp_fft::FftError` from the underlying FFT call.
    Fft(String),
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterError::EmptySignal => write!(f, "filter signal must be non-empty"),
            FilterError::EmptyKernel => write!(f, "filter kernel must be non-empty"),
            FilterError::InvalidCoefficient(msg) => {
                write!(f, "invalid coefficient: {}", msg)
            }
            FilterError::Fft(msg) => write!(f, "FFT failure: {}", msg),
        }
    }
}

impl std::error::Error for FilterError {}

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

    /// Naive O(N · K) FIR oracle — direct double-sum without the
    /// bounded-index optimization in `fir`.  Used to verify
    /// correctness across various N, K combinations.
    fn naive_fir(signal: &[f32], kernel: &[f32]) -> Vec<f32> {
        let n = signal.len();
        let k = kernel.len();
        let mut out = vec![0.0f32; n + k - 1];
        for i in 0..n {
            for j in 0..k {
                out[i + j] += signal[i] * kernel[j];
            }
        }
        out
    }

    // ── error paths ────────────────────────────────────────────

    #[test]
    fn fir_rejects_empty_signal() {
        let err = fir(&[], &[1.0, 2.0]).unwrap_err();
        assert_eq!(err, FilterError::EmptySignal);
    }

    #[test]
    fn fir_rejects_empty_kernel() {
        let err = fir(&[1.0, 2.0, 3.0], &[]).unwrap_err();
        assert_eq!(err, FilterError::EmptyKernel);
    }

    // ── closed-form known vectors ──────────────────────────────

    #[test]
    fn fir_with_identity_kernel_returns_signal() {
        // [1.0] is the convolutional identity.
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let out = fir(&signal, &[1.0]).unwrap();
        assert_eq!(out.len(), 5);
        assert_close(&out, &signal, 1e-7);
    }

    #[test]
    fn fir_with_delay_kernel_shifts_by_one() {
        // [0.0, 1.0, 0.0] is a unit-delay convolved with a
        // surrounding zero — the full convolution starts with a 0
        // and trails with a 0.
        let signal = vec![1.0f32, 2.0, 3.0];
        let out = fir(&signal, &[0.0, 1.0, 0.0]).unwrap();
        // Output length = 3 + 3 - 1 = 5.
        // n=0: kernel[0..1] · signal[0..1] = 0·1 = 0
        // n=1: kernel[0..2] · signal[1, 0] = 0·2 + 1·1 = 1
        // n=2: kernel[0..3] · signal[2, 1, 0] = 0·3 + 1·2 + 0·1 = 2
        // n=3: kernel[1..3] · signal[2, 1] = 1·3 + 0·2 = 3
        // n=4: kernel[2..3] · signal[2] = 0·3 = 0
        let expected = vec![0.0f32, 1.0, 2.0, 3.0, 0.0];
        assert_close(&out, &expected, 1e-7);
    }

    #[test]
    fn fir_with_uniform_kernel_preserves_total_sum() {
        // Convolving with kernel [1.0; K] sums every neighborhood
        // of K samples.  The total sum of the output equals
        // (sum of signal) * (sum of kernel) — basic integral
        // identity for linear convolution.
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let kernel = vec![1.0f32; 3];
        let out = fir(&signal, &kernel).unwrap();
        let sig_sum: f32 = signal.iter().sum();
        let ker_sum: f32 = kernel.iter().sum();
        let out_sum: f32 = out.iter().sum();
        assert!(
            approx_eq(out_sum, sig_sum * ker_sum, 1e-5),
            "out_sum {} != sig_sum {} * ker_sum {} = {}",
            out_sum,
            sig_sum,
            ker_sum,
            sig_sum * ker_sum
        );
    }

    #[test]
    fn fir_with_box_kernel_3tap() {
        // 3-tap box [1, 1, 1] convolved with [1, 2, 3]:
        // n=0: 1·1 = 1
        // n=1: 1·1 + 1·2 = 3
        // n=2: 1·1 + 1·2 + 1·3 = 6
        // n=3: 1·2 + 1·3 = 5
        // n=4: 1·3 = 3
        let signal = vec![1.0f32, 2.0, 3.0];
        let kernel = vec![1.0f32, 1.0, 1.0];
        let out = fir(&signal, &kernel).unwrap();
        assert_close(&out, &[1.0, 3.0, 6.0, 5.0, 3.0], 1e-7);
    }

    // ── length contract ────────────────────────────────────────

    #[test]
    fn fir_output_length_is_n_plus_k_minus_1() {
        for &(n, k) in &[(1usize, 1), (1, 5), (5, 1), (8, 8), (100, 15), (3, 31)] {
            let signal: Vec<f32> = (0..n).map(|i| (i as f32) * 0.1).collect();
            let kernel: Vec<f32> = (0..k).map(|i| 1.0 / ((i + 1) as f32)).collect();
            let out = fir(&signal, &kernel).unwrap();
            assert_eq!(
                out.len(),
                n + k - 1,
                "wrong output length for N={}, K={}",
                n,
                k
            );
        }
    }

    // ── naive cross-check ──────────────────────────────────────

    #[test]
    fn fir_matches_naive_reference_n5_k3() {
        let signal: Vec<f32> = (0..5).map(|i| ((i as f32) * 0.3).sin()).collect();
        let kernel = vec![0.1f32, 0.4, 0.5];
        let optimized = fir(&signal, &kernel).unwrap();
        let naive = naive_fir(&signal, &kernel);
        assert_close(&optimized, &naive, 1e-6);
    }

    #[test]
    fn fir_matches_naive_reference_n8_k4() {
        let signal: Vec<f32> = (0..8).map(|i| ((i as f32) * 0.2).cos()).collect();
        let kernel = vec![0.25f32, 0.5, 0.25, 0.1];
        let optimized = fir(&signal, &kernel).unwrap();
        let naive = naive_fir(&signal, &kernel);
        assert_close(&optimized, &naive, 1e-6);
    }

    #[test]
    fn fir_matches_naive_reference_n100_k15() {
        // Larger size to exercise the inner-loop bounds with both
        // the head and tail edges firing for many output samples.
        let signal: Vec<f32> = (0..100).map(|i| ((i as f32) * 0.07).sin()).collect();
        // Hamming-ish 15-tap kernel.
        let kernel: Vec<f32> = (0..15)
            .map(|i| {
                let theta =
                    2.0 * std::f32::consts::PI * (i as f32) / 14.0;
                0.54 - 0.46 * theta.cos()
            })
            .collect();
        let optimized = fir(&signal, &kernel).unwrap();
        let naive = naive_fir(&signal, &kernel);
        assert_close(&optimized, &naive, 1e-4);
    }
}
