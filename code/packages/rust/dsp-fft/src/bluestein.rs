//! # Bluestein's algorithm — FFT for arbitrary lengths
//!
//! **DSP01 Phase 4a (scalar reference).**  Computes the discrete
//! Fourier transform of a length-`N` sequence for *any* `N ≥ 1`,
//! including non-power-of-two lengths the radix-2 path can't
//! touch.  This phase ships the scalar reference only;
//! later phases (4b: rfft/irfft, 4c: matrix-ir lowered Bluestein)
//! build on top.
//!
//! ## Why Bluestein?
//!
//! The matrix-ir-lowered FFT in [`crate::radix2`] requires `N` to
//! be a power of two — the butterfly tree's depth is `log₂(N)`
//! and the bit-reversal permutation is a power-of-two identity.
//! Real-world signal lengths are usually *not* powers of two
//! (think 1000 samples, 882 samples for audio resampling, etc.).
//!
//! Bluestein's algorithm — also called the chirp z-transform —
//! recasts the length-`N` DFT as a length-`M` *linear convolution*,
//! where `M ≥ 2N - 1` and we pick the next power of two.  The
//! length-`M` convolution is then computed via three length-`M`
//! FFTs (which the radix-2 path handles natively).  The price is
//! one extra FFT-pair worth of memory traffic and three times the
//! arithmetic of a same-size radix-2 FFT; the win is that *every*
//! `N` works with one code path.
//!
//! ## Algorithm
//!
//! The chirp-z identity reorganizes `n · k` as a difference of
//! squares (a "chirp"):
//!
//! ```text
//!     n · k = ( n² + k² - (k - n)² ) / 2
//! ```
//!
//! Substituting into the forward DFT:
//!
//! ```text
//!     X[k] = Σ_n  x[n] · exp(-2πi · n · k / N)
//!          = exp(-iπ · k² / N) · Σ_n  ( x[n] · exp(-iπ · n² / N) )
//!                                      · exp(+iπ · (k - n)² / N)
//! ```
//!
//! Define:
//!
//! - `a[n] = x[n] · exp(-iπ · n² / N)`   ("pre-chirp"; length `N`)
//! - `b[n] = exp(+iπ · n² / N)`           ("anti-chirp"; length
//!   `2N - 1` if indexed from `-(N - 1)` to `N - 1`)
//!
//! Then `X[k] = exp(-iπ · k² / N) · (a ⋆ b)[k]` for `k = 0..N-1`,
//! where `⋆` is linear convolution.
//!
//! To compute the convolution we pick `M = next_pow2(2N - 1)` and:
//!
//! 1. Zero-pad `a` from length `N` to length `M`.
//! 2. Build `b'` of length `M` by wrapping the bilateral chirp:
//!    `b'[k] = exp(+iπ · k² / N)` for `k = 0..N-1`,
//!    `b'[k] = exp(+iπ · (k - M)² / N)` for `k = M - N + 1..M-1`,
//!    `b'[k] = 0` elsewhere.
//!    The "wrap" gives us the negative-index half of `b`.
//! 3. Compute `A = FFT(a)`, `B = FFT(b')`, `C[k] = A[k] · B[k]`
//!    (elementwise complex multiply).
//! 4. `c = IFFT(C)`.  The first `N` samples of `c` are the
//!    linear convolution `(a ⋆ b)[0..N]`.
//! 5. `X[k] = exp(-iπ · k² / N) · c[k]` for `k = 0..N-1`.
//!
//! Inverse FFT is the same algorithm with the chirp sign flipped
//! and a final `1/N` scaling (backward normalization, matching
//! the radix-2 path).
//!
//! ## Complexity
//!
//! - **Time**: three length-`M` FFTs at `M · log₂(M)` ops each,
//!   plus three length-`M` pointwise complex multiplies — i.e.
//!   `O(M log M) = O(N log N)` since `M < 4N`.
//! - **Memory**: `O(M) = O(N)` working storage.
//! - **Numerical accuracy**: the chirp computation involves
//!   floating-point modular arithmetic on `n²`, which loses
//!   precision for very large `N` (`n²` for `n = 65535` is
//!   `2³²`-scale and the residue mod `2N` loses bits).  This
//!   matters past `N ≈ 1M` in `f32`; below that, the round-trip
//!   stays within `1e-4` like the radix-2 path.
//!
//! ## What this module does NOT do (Phase 4a)
//!
//! - Matrix-ir lowering.  The convolution uses `fft_scalar` /
//!   `ifft_scalar` internally so it runs on CPU only.  Phase 4c
//!   will replace those calls with [`crate::radix2::fft_via_runtime`]
//!   (or a `bluestein_via_runtime` wrapper) so the whole thing
//!   lifts onto the matrix execution layer.
//! - `rfft` / `irfft` half-spectrum APIs.  Phase 4b.
//! - Real-input optimisation.  Bluestein on a real signal still
//!   does the full complex convolution; the half-spectrum win
//!   comes from `rfft`.

use crate::{fft_scalar, ifft_scalar, Direction, FftError};

/// Forward / inverse FFT via Bluestein's algorithm.  Accepts any
/// `N ≥ 1`, including non-power-of-two lengths.  Operates on
/// interleaved `[re, im, re, im, …]` `f32` buffers — same layout
/// as [`fft_scalar`].
///
/// `signal` must have even length (one `[re, im]` pair per
/// element).  Length is interpreted as `N = signal.len() / 2`.
///
/// The output has the same length and convention as the
/// corresponding direction of [`fft_scalar`] / [`ifft_scalar`].
/// In particular the inverse direction applies the same
/// "backward" `1/N` normalization (matches numpy / scipy /
/// MATLAB).
///
/// For power-of-two `N` the radix-2 path
/// ([`fft_scalar`] / [`ifft_scalar`]) is faster.  This routine
/// is the canonical fallback for everything else.
pub fn bluestein_scalar(
    signal: &[f32],
    direction: Direction,
) -> Result<Vec<f32>, FftError> {
    if signal.len() % 2 != 0 {
        return Err(FftError::InvalidInput(format!(
            "interleaved buffer must have even length; got {}",
            signal.len()
        )));
    }
    let n = signal.len() / 2;
    if n == 0 {
        return Err(FftError::InvalidInput(
            "FFT length must be at least 1".into(),
        ));
    }

    // ── Trivial N = 1 case: the DFT is the identity.
    //   Skip the chirp construction entirely.  We still apply the
    //   inverse direction's `1/N` factor, but with N = 1 that's a
    //   no-op too.
    if n == 1 {
        return Ok(signal.to_vec());
    }

    // ── Step 0: pick the convolution length M = next_pow2(2N - 1).
    //   This is the smallest power of two that fits a linear
    //   convolution of two length-N sequences (length 2N - 1).
    let conv_len = 2 * n - 1;
    let m = conv_len.next_power_of_two();

    // ── Step 1: build the chirp.  The pre-chirp factor for index
    //   `k` is `exp(sign · iπ · k² / N)` where `sign = -1` for
    //   forward, `+1` for inverse.  We compute `k² mod 2N` to
    //   stay precise — the chirp is periodic with period `2N`
    //   (since `exp(±iπ · 2N / N) = exp(±2πi) = 1`).
    //
    //   We store the chirp interleaved `[re, im]` as a small
    //   helper Vec.  `chirp[k] = exp(sign · iπ · k² / N)`.
    let sign: f32 = match direction {
        Direction::Forward => -1.0,
        Direction::Inverse => 1.0,
    };
    let chirp = build_chirp(n, sign);

    // ── Step 2: a[n] = x[n] · chirp[n], padded to length M with
    //   zeros.  Output is interleaved length 2M.
    let mut a = vec![0.0f32; 2 * m];
    for k in 0..n {
        let xr = signal[2 * k];
        let xi = signal[2 * k + 1];
        let cr = chirp[2 * k];
        let ci = chirp[2 * k + 1];
        // Complex multiply: (xr + i·xi) · (cr + i·ci)
        //   = (xr·cr - xi·ci) + i(xr·ci + xi·cr)
        a[2 * k]     = xr * cr - xi * ci;
        a[2 * k + 1] = xr * ci + xi * cr;
    }

    // ── Step 3: build b' of length M.  This is the bilateral
    //   chirp `exp(-sign · iπ · k² / N)` (note opposite sign from
    //   the pre-chirp), indexed from -(N-1) to N-1 and wrapped
    //   onto [0, M).
    //
    //   - b'[k] for k = 0..N         = chirp_conj[k]
    //   - b'[k] for k = M-N+1..M     = chirp_conj[M - k]
    //   - b'[k] for k = N..M-N       = 0
    //
    //   `chirp_conj` is `exp(-sign · iπ · k² / N)`, which is
    //   the elementwise conjugate of `chirp` (sign-flipped imag).
    let mut b = vec![0.0f32; 2 * m];
    for k in 0..n {
        b[2 * k]     =  chirp[2 * k];      // re unchanged
        b[2 * k + 1] = -chirp[2 * k + 1];  // im negated (conjugate)
    }
    for k in 1..n {
        // Index M - k wraps to the negative half of the bilateral
        // chirp.  Since `(-k)² == k²`, `chirp_conj[M-k]` is just
        // the same chirp_conj[k] value.
        let idx = m - k;
        b[2 * idx]     =  chirp[2 * k];
        b[2 * idx + 1] = -chirp[2 * k + 1];
    }

    // ── Step 4: convolve via FFT.  A = FFT(a), B = FFT(b),
    //   C[k] = A[k] · B[k], c = IFFT(C).
    //
    //   `m` is a power of two by construction so `fft_scalar`
    //   accepts both calls.
    let a_spec = fft_scalar(&a)?;
    let b_spec = fft_scalar(&b)?;
    let mut c_spec = vec![0.0f32; 2 * m];
    for k in 0..m {
        let ar = a_spec[2 * k];
        let ai = a_spec[2 * k + 1];
        let br = b_spec[2 * k];
        let bi = b_spec[2 * k + 1];
        c_spec[2 * k]     = ar * br - ai * bi;
        c_spec[2 * k + 1] = ar * bi + ai * br;
    }
    let conv = ifft_scalar(&c_spec)?;

    // ── Step 5: X[k] = chirp[k] · conv[k] for k = 0..N.
    //
    //   For inverse direction we additionally divide by N to
    //   match the "backward" normalization convention.  The
    //   `ifft_scalar` call above already divided by M (the
    //   convolution length), but the outer DFT is length N, so
    //   we need an extra `M / N` correction — except wait, no:
    //   the chirp identity gives us *linear convolution*, and
    //   `ifft_scalar(FFT(a) · FFT(b))` is *circular convolution*
    //   of length M.  Linear ⊆ circular when M ≥ 2N - 1, which
    //   we guaranteed in Step 0, so the first N samples are the
    //   linear convolution we want.
    //
    //   Normalization: `ifft_scalar` divides by M.  But the
    //   linear convolution we want has *no* such division — it's
    //   `Σ_j a[j] · b[k-j]`.  The chirp identity for the
    //   *forward* DFT comes out clean: `X[k] = chirp[k] · conv[k]`
    //   with no extra factor.  For the *inverse* DFT we also need
    //   a `1/N` factor to match the backward convention.
    let mut out = vec![0.0f32; 2 * n];
    let inv_n: f32 = if direction == Direction::Inverse {
        1.0 / (n as f32)
    } else {
        1.0
    };
    for k in 0..n {
        let cr = conv[2 * k];
        let ci = conv[2 * k + 1];
        let wr = chirp[2 * k];
        let wi = chirp[2 * k + 1];
        // (wr + i·wi) · (cr + i·ci)
        let re = wr * cr - wi * ci;
        let im = wr * ci + wi * cr;
        out[2 * k]     = re * inv_n;
        out[2 * k + 1] = im * inv_n;
    }
    Ok(out)
}

/// Build the chirp sequence `chirp[k] = exp(sign · iπ · k² / N)`
/// for `k = 0..N`, returned as an interleaved `[re, im]` buffer.
///
/// The exponent argument is `sign · π · (k² mod 2N) / N` —
/// reducing `k²` modulo `2N` before the floating-point divide
/// keeps the value bounded to `(-π, π)` regardless of how large
/// `k` gets, which is what saves us precision for large `N`.
///
/// Why `2N` and not `N`?  Because the chirp has period `2N`:
/// `exp(iπ · (k + 2N)² / N)` differs from `exp(iπ · k² / N)`
/// by `exp(iπ · (4kN + 4N²) / N) = exp(4πi · k + 4πi · N) = 1`.
fn build_chirp(n: usize, sign: f32) -> Vec<f32> {
    use std::f32::consts::PI;
    let two_n = (2 * n) as u64;
    let mut out = Vec::with_capacity(2 * n);
    for k in 0..n {
        // k² mod 2N — compute in u64 to handle large N without
        // overflow.  N ≤ usize::MAX, k < N, k² could be up to
        // N² which on 64-bit usize already fits, but using u64
        // explicit is clearer.
        let k_sq = (k as u64).wrapping_mul(k as u64);
        let residue = (k_sq % two_n) as f32;
        let theta = sign * PI * residue / (n as f32);
        let (im, re) = theta.sin_cos();
        out.push(re);
        out.push(im);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Loose float equality with a "scale-aware" tolerance — we
    /// compare magnitudes, so `tol` is interpreted as a relative
    /// epsilon for values above 1.0 and an absolute epsilon below.
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

    /// Naive O(N²) DFT for use as a small-N test oracle.  We use
    /// it to validate Bluestein against the textbook definition
    /// at the non-power-of-two sizes the radix-2 path can't run.
    fn naive_dft(signal: &[f32], direction: Direction) -> Vec<f32> {
        let n = signal.len() / 2;
        let sign: f32 = match direction {
            Direction::Forward => -1.0,
            Direction::Inverse => 1.0,
        };
        let scale: f32 = match direction {
            Direction::Forward => 1.0,
            Direction::Inverse => 1.0 / (n as f32),
        };
        let mut out = vec![0.0f32; 2 * n];
        for k in 0..n {
            let mut sr = 0.0f32;
            let mut si = 0.0f32;
            for nn in 0..n {
                let theta = sign * 2.0 * PI * (k as f32) * (nn as f32) / (n as f32);
                let (wi, wr) = theta.sin_cos();
                let xr = signal[2 * nn];
                let xi = signal[2 * nn + 1];
                sr += wr * xr - wi * xi;
                si += wr * xi + wi * xr;
            }
            out[2 * k]     = sr * scale;
            out[2 * k + 1] = si * scale;
        }
        out
    }

    // ── error paths ────────────────────────────────────────────

    #[test]
    fn rejects_odd_length_buffer() {
        let err = bluestein_scalar(&[1.0, 2.0, 3.0], Direction::Forward).unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    #[test]
    fn rejects_empty_buffer() {
        let err = bluestein_scalar(&[], Direction::Forward).unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    // ── degenerate / power-of-two N ─────────────────────────────

    #[test]
    fn n1_is_identity_forward() {
        let signal = vec![3.5f32, -1.25];
        let spectrum = bluestein_scalar(&signal, Direction::Forward).unwrap();
        assert_close(&spectrum, &signal, 1e-7);
    }

    #[test]
    fn n1_is_identity_inverse() {
        let signal = vec![3.5f32, -1.25];
        let recovered = bluestein_scalar(&signal, Direction::Inverse).unwrap();
        assert_close(&recovered, &signal, 1e-7);
    }

    #[test]
    fn bluestein_matches_radix2_for_power_of_two_n8() {
        // Sanity check: at power-of-two N, Bluestein and the
        // radix-2 path should agree to within numerical
        // tolerance.  This is a cross-check that the chirp
        // construction is correct.
        let signal: Vec<f32> = (0..8)
            .flat_map(|i| [(i as f32) * 0.3 - 0.7, ((i as f32) * 0.11).sin()])
            .collect();
        let via_radix2 = fft_scalar(&signal).unwrap();
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        assert_close(&via_bluestein, &via_radix2, 1e-4);
    }

    // ── non-power-of-two N (the whole point of Bluestein) ───────

    #[test]
    fn forward_n3_matches_naive_dft() {
        // N = 3: smallest non-trivial non-pow2.  Convolution length
        // M = next_pow2(2·3 - 1) = next_pow2(5) = 8.
        let signal = vec![1.0f32, 0.0, 2.0, 0.0, 3.0, 0.0];
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        let via_naive = naive_dft(&signal, Direction::Forward);
        assert_close(&via_bluestein, &via_naive, 1e-4);
    }

    #[test]
    fn forward_n5_matches_naive_dft() {
        // N = 5: M = next_pow2(9) = 16.
        let signal: Vec<f32> = (0..5)
            .flat_map(|i| [(i as f32) - 2.0, (i as f32) * 0.5])
            .collect();
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        let via_naive = naive_dft(&signal, Direction::Forward);
        assert_close(&via_bluestein, &via_naive, 1e-4);
    }

    #[test]
    fn forward_n6_matches_naive_dft() {
        // N = 6: M = next_pow2(11) = 16.
        let signal: Vec<f32> = (0..6)
            .flat_map(|i| {
                let x = (2.0 * PI * (i as f32) / 6.0).cos();
                [x, 0.0f32]
            })
            .collect();
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        let via_naive = naive_dft(&signal, Direction::Forward);
        assert_close(&via_bluestein, &via_naive, 1e-4);
    }

    #[test]
    fn forward_n7_matches_naive_dft() {
        // N = 7 is a prime — the worst-case scenario for any
        // mixed-radix FFT.  Bluestein still handles it at the
        // same cost (M = next_pow2(13) = 16).
        let signal: Vec<f32> = (0..7)
            .flat_map(|i| [(i as f32).sin(), ((i as f32) * 0.3).cos()])
            .collect();
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        let via_naive = naive_dft(&signal, Direction::Forward);
        assert_close(&via_bluestein, &via_naive, 1e-4);
    }

    #[test]
    fn forward_n12_matches_naive_dft() {
        // N = 12 is composite but not a power of two.
        // M = next_pow2(23) = 32.
        let signal: Vec<f32> = (0..12)
            .flat_map(|i| [((i as f32) * 0.1).sin(), ((i as f32) * 0.07).cos()])
            .collect();
        let via_bluestein = bluestein_scalar(&signal, Direction::Forward).unwrap();
        let via_naive = naive_dft(&signal, Direction::Forward);
        assert_close(&via_bluestein, &via_naive, 1e-4);
    }

    // ── round-trip ─────────────────────────────────────────────

    #[test]
    fn round_trip_n3() {
        let original = vec![1.0f32, 0.0, 2.0, 0.0, 3.0, 0.0];
        let spectrum = bluestein_scalar(&original, Direction::Forward).unwrap();
        let recovered = bluestein_scalar(&spectrum, Direction::Inverse).unwrap();
        assert_close(&original, &recovered, 1e-4);
    }

    #[test]
    fn round_trip_n7_real_complex_mix() {
        let n: usize = 7;
        let original: Vec<f32> = (0..n)
            .flat_map(|i| [(i as f32) * 0.3, ((i as f32) * 0.5).sin()])
            .collect();
        let spectrum = bluestein_scalar(&original, Direction::Forward).unwrap();
        let recovered = bluestein_scalar(&spectrum, Direction::Inverse).unwrap();
        assert_close(&original, &recovered, 1e-4);
    }

    #[test]
    fn round_trip_works_for_many_sizes() {
        // Stress test: round-trip every N from 1 to 32.
        // Includes power-of-two N (where Bluestein and radix-2
        // both work) and arbitrary N (Bluestein's home turf).
        for n in 1..=32usize {
            let original: Vec<f32> = (0..n)
                .flat_map(|i| [((i as f32) * 0.15).sin(), 0.0f32])
                .collect();
            let spectrum =
                bluestein_scalar(&original, Direction::Forward).unwrap();
            let recovered =
                bluestein_scalar(&spectrum, Direction::Inverse).unwrap();
            // Tolerance scales loosely with N; 1e-3 covers N=32
            // comfortably.
            for (i, (a, b)) in original.iter().zip(recovered.iter()).enumerate() {
                let scale = a.abs().max(b.abs()).max(1.0);
                assert!(
                    (a - b).abs() <= scale * 1e-3,
                    "round-trip failed for N={}, index {}: {} vs {}",
                    n,
                    i,
                    a,
                    b
                );
            }
        }
    }

    // ── closed-form known vectors ──────────────────────────────

    #[test]
    fn forward_impulse_n5_is_all_ones() {
        // fft(impulse) = [1, 1, …, 1] regardless of N.
        let n = 5;
        let mut signal = vec![0.0f32; 2 * n];
        signal[0] = 1.0;
        let spectrum = bluestein_scalar(&signal, Direction::Forward).unwrap();
        for k in 0..n {
            assert!(
                approx_eq(spectrum[2 * k], 1.0, 1e-4),
                "bin {} real = {}, expected 1.0",
                k,
                spectrum[2 * k]
            );
            assert!(
                approx_eq(spectrum[2 * k + 1], 0.0, 1e-4),
                "bin {} imag = {}, expected 0.0",
                k,
                spectrum[2 * k + 1]
            );
        }
    }

    #[test]
    fn forward_dc_n7_is_single_bin() {
        // fft(constant) = [N, 0, 0, …, 0].
        let n = 7;
        let signal: Vec<f32> = (0..n).flat_map(|_| [1.0f32, 0.0]).collect();
        let spectrum = bluestein_scalar(&signal, Direction::Forward).unwrap();
        assert!(approx_eq(spectrum[0], n as f32, 1e-4));
        assert!(approx_eq(spectrum[1], 0.0, 1e-4));
        for k in 1..n {
            assert!(
                approx_eq(spectrum[2 * k], 0.0, 1e-3),
                "bin {} real = {}, expected 0",
                k,
                spectrum[2 * k]
            );
            assert!(
                approx_eq(spectrum[2 * k + 1], 0.0, 1e-3),
                "bin {} imag = {}, expected 0",
                k,
                spectrum[2 * k + 1]
            );
        }
    }
}
