//! # Real-input FFT (rfft) and its inverse (irfft)
//!
//! **DSP01 Phase 4b (scalar reference).**  Adds the `rfft` /
//! `irfft` pair to the public API.  These exploit conjugate
//! symmetry of real-input spectra:
//!
//! ```text
//!     For real x[n],  X[N - k] = conj(X[k])
//! ```
//!
//! Half the spectrum is redundant, so `rfft` returns only the
//! first `⌊N / 2⌋ + 1` bins.  That's `5` bins for `N = 8`, `4`
//! bins for `N = 7`, etc.  `irfft` takes the half-spectrum back
//! to a real signal of length `N` (the caller has to pass `N`
//! explicitly because `⌊N / 2⌋ + 1` doesn't disambiguate even
//! from odd lengths).
//!
//! ## Phase 4b scope
//!
//! This phase ships the correctness contract — every `N ≥ 1`
//! works, every closed-form known vector matches, every
//! `irfft(rfft(x)) ≈ x` round-trip succeeds.  We do *not*
//! implement the half-length "packing trick" optimization here:
//!
//! - The classic optimisation packs even/odd real samples into
//!   `N / 2` complex elements, FFTs that, then unpacks via twiddle
//!   multiplication to recover the length-`N` real spectrum.
//!   Saves ~2× wall-clock.
//! - Our V1 implementation just calls the existing complex
//!   [`crate::fft`] / [`crate::ifft`] (which already handle any
//!   `N` via radix-2 or Bluestein), then slices / mirrors the
//!   result.
//! - Asymptotic complexity is identical (`O(N log N)`); only the
//!   constant factor differs.  Phase 5 will add the packing
//!   optimisation when perf matters.
//!
//! ## Algorithm — `rfft`
//!
//! 1. Wrap the real signal as interleaved complex with `im = 0`:
//!    `[x[0], 0, x[1], 0, …, x[N-1], 0]`.
//! 2. Call [`crate::fft_scalar`] (or [`crate::bluestein_scalar`]
//!    for non-pow2 `N`) — same path the public `fft` uses.
//! 3. Slice off the first `⌊N / 2⌋ + 1` complex bins.
//!
//! The output is interleaved `[re, im, …, re, im]` of length
//! `2 · (⌊N / 2⌋ + 1)`.
//!
//! ## Algorithm — `irfft`
//!
//! 1. Reconstruct the full length-`N` spectrum from the
//!    half-spectrum using `X[N - k] = conj(X[k])`:
//!    - `full[0..⌊N/2⌋+1] = half_spectrum`
//!    - For `k in 1..(N + 1) / 2`: `full[N - k] = conj(half[k])`
//!
//!    For even `N`, this leaves bin `N/2` (Nyquist) untouched —
//!    the half-spectrum already supplies it, and it has no
//!    reflection partner.  For odd `N`, the loop hits every
//!    non-DC bin and there's no Nyquist.
//! 2. Call [`crate::ifft_scalar`] (or [`crate::bluestein_scalar`]
//!    with `Direction::Inverse`).
//! 3. Return only the real part (drop the imaginary lane).
//!
//! The imaginary lane of the inverse FFT output should be zero
//! up to FP noise when the input half-spectrum is a genuine
//! conjugate-symmetric spectrum; we don't enforce that — callers
//! who pass bogus half-spectra get a complex inverse with a
//! non-zero imaginary part that we silently discard.  Adding a
//! "imag must be near-zero" check is conceivable but matches
//! `numpy.fft.irfft` semantics by not bothering.

use crate::{bluestein_scalar, fft_scalar, ifft_scalar, Direction, FftError};
use dsp_complex::ComplexTensor;

/// Scalar real-input FFT.  Takes a real signal `[x[0], …, x[N-1]]`
/// of length `N ≥ 1` and returns the first `⌊N / 2⌋ + 1` complex
/// spectrum bins, interleaved `[re, im, …, re, im]`.
///
/// The output buffer has length `2 · (⌊N / 2⌋ + 1)`.
///
/// This is the canonical scalar oracle; the public [`crate::rfft`]
/// wraps it as a [`ComplexTensor`].
pub fn rfft_scalar(signal: &[f32]) -> Result<Vec<f32>, FftError> {
    let n = signal.len();
    if n == 0 {
        return Err(FftError::InvalidInput(
            "rfft input must be at least 1 sample".into(),
        ));
    }
    // ── Wrap as interleaved complex with im = 0.
    let mut interleaved = Vec::with_capacity(n * 2);
    for &x in signal {
        interleaved.push(x);
        interleaved.push(0.0);
    }
    // ── Full FFT (power-of-two via radix-2, otherwise Bluestein).
    let full = if n.is_power_of_two() {
        fft_scalar(&interleaved)?
    } else {
        bluestein_scalar(&interleaved, Direction::Forward)?
    };
    // ── Take the first ⌊N / 2⌋ + 1 complex bins.
    let half_bins = n / 2 + 1;
    let mut out = Vec::with_capacity(half_bins * 2);
    out.extend_from_slice(&full[..half_bins * 2]);
    Ok(out)
}

/// Scalar inverse real-input FFT.  Takes a half-spectrum of
/// length `⌊N / 2⌋ + 1` complex bins (interleaved
/// `[re, im, …, re, im]`) and an explicit `output_length` equal
/// to `N`, returns the real signal of length `N` as a `Vec<f32>`.
///
/// `output_length` is required because `⌊N / 2⌋ + 1` doesn't
/// uniquely determine `N` (e.g. `N = 7` and `N = 8` both yield
/// 4 and 5 bins respectively, but other adjacent pairs collide
/// — `(N, half_bins)` is one-to-one only with `N` given).  This
/// matches the `numpy.fft.irfft(a, n)` convention.
pub fn irfft_scalar(
    half_spectrum: &[f32],
    output_length: u32,
) -> Result<Vec<f32>, FftError> {
    let n = output_length as usize;
    if n == 0 {
        return Err(FftError::InvalidInput(
            "irfft output length must be at least 1".into(),
        ));
    }
    if half_spectrum.len() % 2 != 0 {
        return Err(FftError::InvalidInput(format!(
            "irfft half-spectrum must have even length; got {}",
            half_spectrum.len()
        )));
    }
    let half_bins = half_spectrum.len() / 2;
    let expected_half_bins = n / 2 + 1;
    if half_bins != expected_half_bins {
        return Err(FftError::InvalidInput(format!(
            "irfft: half-spectrum has {} bins, expected {} for output_length = {}",
            half_bins, expected_half_bins, n
        )));
    }

    // ── Step 1: reconstruct the full length-N spectrum using
    //   X[N - k] = conj(X[k]).
    //
    //   Memory layout: full[2k] = X[k].re, full[2k+1] = X[k].im.
    let mut full = vec![0.0f32; 2 * n];
    // Copy the half-spectrum into bins 0..half_bins.
    full[..2 * half_bins].copy_from_slice(half_spectrum);
    // Reflect bins 1..(N + 1) / 2 onto N - k with conjugation.
    //
    // For even N: covers k = 1..N/2 (exclusive), leaving bin N/2
    //   (Nyquist) as-is — it was already in the half-spectrum.
    // For odd N: covers k = 1..(N - 1)/2 + 1 = 1..(N+1)/2.
    let reflect_end = (n + 1) / 2;
    for k in 1..reflect_end {
        let src_re = full[2 * k];
        let src_im = full[2 * k + 1];
        let dst = n - k;
        full[2 * dst]     =  src_re;
        full[2 * dst + 1] = -src_im;
    }

    // ── Step 2: inverse FFT.
    let inverted = if n.is_power_of_two() {
        ifft_scalar(&full)?
    } else {
        bluestein_scalar(&full, Direction::Inverse)?
    };

    // ── Step 3: drop the imaginary lane.  Real-input spectra
    //   produce purely real inverse transforms (modulo FP noise);
    //   we keep only the real component the way numpy does.
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        out.push(inverted[2 * k]);
    }
    Ok(out)
}

/// Public real-input FFT.  Returns a [`ComplexTensor`] of length
/// `⌊N / 2⌋ + 1` holding the half-spectrum.  Wraps
/// [`rfft_scalar`].
pub fn rfft(signal: &[f32]) -> Result<ComplexTensor, FftError> {
    let interleaved = rfft_scalar(signal)?;
    Ok(ComplexTensor::from_interleaved(interleaved)
        .expect("rfft_scalar output has even length"))
}

/// Public inverse real-input FFT.  Takes a [`ComplexTensor`] of
/// `⌊N / 2⌋ + 1` complex bins and an explicit `output_length`
/// equal to `N`, returns the real signal of length `N` as a
/// `Vec<f32>`.  Wraps [`irfft_scalar`].
pub fn irfft(
    half_spectrum: &ComplexTensor,
    output_length: u32,
) -> Result<Vec<f32>, FftError> {
    irfft_scalar(half_spectrum.as_interleaved(), output_length)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

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
    fn rfft_rejects_empty_signal() {
        let err = rfft_scalar(&[]).unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    #[test]
    fn irfft_rejects_zero_output_length() {
        let err = irfft_scalar(&[1.0, 0.0], 0).unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    #[test]
    fn irfft_rejects_odd_buffer_length() {
        // Half-spectrum is interleaved; length must be even.
        let err = irfft_scalar(&[1.0, 0.0, 2.0], 2).unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    #[test]
    fn irfft_rejects_mismatched_bin_count() {
        // N = 8 needs 5 bins (10 f32); we pass only 4 bins (8 f32).
        let bad = vec![0.0f32; 8];
        let err = irfft_scalar(&bad, 8).unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    // ── closed-form known vectors ──────────────────────────────

    #[test]
    fn rfft_of_impulse_is_all_ones_n8() {
        // x = [1, 0, 0, …, 0] → X[k] = 1 for all k.  Half-spectrum
        // has 5 bins, each = 1 + 0i.
        let n = 8;
        let mut signal = vec![0.0f32; n];
        signal[0] = 1.0;
        let half = rfft_scalar(&signal).unwrap();
        assert_eq!(half.len(), 2 * (n / 2 + 1));
        for k in 0..(n / 2 + 1) {
            assert!(approx_eq(half[2 * k], 1.0, 1e-5));
            assert!(approx_eq(half[2 * k + 1], 0.0, 1e-5));
        }
    }

    #[test]
    fn rfft_of_dc_is_single_bin_n8() {
        // x = [1, 1, 1, …, 1] → X = [N, 0, 0, …].  Half-spectrum
        // starts at [N, 0] then all zeros.
        let n = 8;
        let signal = vec![1.0f32; n];
        let half = rfft_scalar(&signal).unwrap();
        assert!(approx_eq(half[0], n as f32, 1e-5));
        assert!(approx_eq(half[1], 0.0, 1e-5));
        for k in 1..(n / 2 + 1) {
            assert!(approx_eq(half[2 * k], 0.0, 1e-4));
            assert!(approx_eq(half[2 * k + 1], 0.0, 1e-4));
        }
    }

    #[test]
    fn rfft_of_pure_cosine_concentrates_one_bin_n16() {
        // x[n] = cos(2π · k0 · n / N) with k0 ≤ N/2.  Half-spectrum
        // gets magnitude N/2 at bin k0, near-zero elsewhere.  The
        // "second half" of the full spectrum (which would also have
        // magnitude N/2 at N - k0) is folded away — that's the
        // whole point of rfft.
        let n: usize = 16;
        let k0: usize = 3;
        let signal: Vec<f32> = (0..n)
            .map(|nn| (2.0 * PI * (k0 as f32) * (nn as f32) / (n as f32)).cos())
            .collect();
        let half = rfft_scalar(&signal).unwrap();
        let half_bins = n / 2 + 1;
        let target_mag = (n as f32) / 2.0;
        for k in 0..half_bins {
            let mag = (half[2 * k].powi(2) + half[2 * k + 1].powi(2)).sqrt();
            if k == k0 {
                assert!(
                    (mag - target_mag).abs() < 1e-3,
                    "bin {} mag = {}, expected {}",
                    k,
                    mag,
                    target_mag
                );
            } else {
                assert!(mag < 1e-3, "bin {} mag = {}", k, mag);
            }
        }
    }

    // ── round-trip ─────────────────────────────────────────────

    #[test]
    fn round_trip_n1() {
        let signal = vec![3.5f32];
        let half = rfft_scalar(&signal).unwrap();
        let recovered = irfft_scalar(&half, 1).unwrap();
        assert_close(&recovered, &signal, 1e-5);
    }

    #[test]
    fn round_trip_pow2_n8() {
        let signal: Vec<f32> = (0..8).map(|i| (i as f32) * 0.5 - 1.5).collect();
        let half = rfft_scalar(&signal).unwrap();
        let recovered = irfft_scalar(&half, 8).unwrap();
        assert_close(&recovered, &signal, 1e-4);
    }

    #[test]
    fn round_trip_pow2_n16() {
        let signal: Vec<f32> = (0..16)
            .map(|i| ((i as f32) * 0.3).sin() + ((i as f32) * 0.07).cos())
            .collect();
        let half = rfft_scalar(&signal).unwrap();
        let recovered = irfft_scalar(&half, 16).unwrap();
        assert_close(&recovered, &signal, 1e-4);
    }

    #[test]
    fn round_trip_pow2_n64() {
        // Deterministic pseudorandom-ish signal at the largest
        // pow2 the Phase 2 round-trip tests covered.
        let n = 64usize;
        let signal: Vec<f32> = (0..n)
            .map(|i| {
                let phase = 2.0 * PI * (i as f32) / (n as f32);
                phase.sin() * 0.7 + (3.0 * phase).cos() * 0.3
            })
            .collect();
        let half = rfft_scalar(&signal).unwrap();
        let recovered = irfft_scalar(&half, n as u32).unwrap();
        assert_close(&recovered, &signal, 1e-4);
    }

    #[test]
    fn round_trip_non_pow2_n3() {
        // Smallest non-power-of-two.  Internally goes through
        // Bluestein for both directions.
        let signal = vec![1.0f32, 2.0, 3.0];
        let half = rfft_scalar(&signal).unwrap();
        assert_eq!(half.len(), 2 * (3 / 2 + 1)); // 2 bins
        let recovered = irfft_scalar(&half, 3).unwrap();
        assert_close(&recovered, &signal, 1e-4);
    }

    #[test]
    fn round_trip_non_pow2_n7() {
        // N = 7: prime.  Half-spectrum has 4 bins.  The
        // reflection loop runs for k = 1, 2, 3 (every non-DC bin).
        let signal: Vec<f32> = (0..7)
            .map(|i| ((i as f32) * 0.4).cos() * 1.5)
            .collect();
        let half = rfft_scalar(&signal).unwrap();
        assert_eq!(half.len(), 2 * (7 / 2 + 1)); // 4 bins
        let recovered = irfft_scalar(&half, 7).unwrap();
        assert_close(&recovered, &signal, 1e-4);
    }

    #[test]
    fn round_trip_non_pow2_n12() {
        // N = 12 composite non-pow2.  Half-spectrum has 7 bins.
        let signal: Vec<f32> = (0..12)
            .map(|i| ((i as f32) * 0.25).sin() + ((i as f32) * 0.1).cos())
            .collect();
        let half = rfft_scalar(&signal).unwrap();
        assert_eq!(half.len(), 2 * (12 / 2 + 1)); // 7 bins
        let recovered = irfft_scalar(&half, 12).unwrap();
        assert_close(&recovered, &signal, 1e-4);
    }

    #[test]
    fn round_trip_works_for_every_n_in_range() {
        // Stress: round-trip for every N ∈ 1..=20.  Catches
        // off-by-one in the Nyquist / reflection loop boundary
        // for both even and odd N.
        for n in 1..=20usize {
            let signal: Vec<f32> = (0..n)
                .map(|i| ((i as f32) * 0.15).sin())
                .collect();
            let half = rfft_scalar(&signal).unwrap();
            let recovered = irfft_scalar(&half, n as u32).unwrap();
            for (i, (a, b)) in signal.iter().zip(recovered.iter()).enumerate() {
                let scale = a.abs().max(b.abs()).max(1.0);
                assert!(
                    (a - b).abs() <= scale * 1e-3,
                    "round-trip failed for N = {}, index {}: {} vs {}",
                    n,
                    i,
                    a,
                    b
                );
            }
        }
    }

    // ── public API ─────────────────────────────────────────────

    #[test]
    fn public_rfft_returns_complex_tensor() {
        let signal = vec![1.0f32, 0.0, 0.0, 0.0]; // impulse, N = 4
        let half = rfft(&signal).unwrap();
        assert_eq!(half.len(), 3); // ⌊4/2⌋ + 1 = 3 bins
        for k in 0..3 {
            assert!(approx_eq(half.as_interleaved()[2 * k], 1.0, 1e-5));
            assert!(approx_eq(half.as_interleaved()[2 * k + 1], 0.0, 1e-5));
        }
    }

    #[test]
    fn public_irfft_round_trips_via_complex_tensor() {
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let half = rfft(&signal).unwrap();
        let recovered = irfft(&half, 8).unwrap();
        assert_close(&recovered, &signal, 1e-4);
    }

    #[test]
    fn public_rfft_irfft_round_trip_non_pow2() {
        let signal: Vec<f32> = (0..7).map(|i| (i as f32) * 0.5).collect();
        let half = rfft(&signal).unwrap();
        let recovered = irfft(&half, 7).unwrap();
        assert_close(&recovered, &signal, 1e-4);
    }
}
