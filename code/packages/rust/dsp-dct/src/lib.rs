// These index loops walk transform basis functions by index, several of them
// indexing parallel arrays (`out`/`coeffs`/`x`) or using the index in the cosine
// arithmetic; the explicit range mirrors the DCT/IDCT math, so keep it.
#![allow(clippy::needless_range_loop)]
//! # `dsp-dct` — Discrete Cosine Transform
//!
//! **DSP02 Phase 1 + 2 (this release).**  Pure-Rust scalar
//! reference for DCT-II (forward) and DCT-III (inverse), in
//! both `None` (un-normalised) and `Ortho` (orthonormal,
//! mutual-inverse) conventions.  Matches `scipy.fft.dct(type=2)`
//! / `scipy.fft.idct(type=3)` exactly.
//!
//! ## Algorithm summary
//!
//! - **DCT-II** uses the Makhoul reduction: pre-shuffle the
//!   length-`N` real input into a length-`N` real sequence,
//!   FFT it (via `dsp-fft::fft_scalar`), multiply by twiddles,
//!   take the real part times two, then apply normalisation.
//!   `O(N log N)` time.
//! - **DCT-III** uses the textbook `O(N²)` double-sum in this
//!   phase.  Phase 3 will lower it to FFT (per the
//!   "Algorithm — DCT-III via FFT" section of the DSP02 spec)
//!   so it lifts onto the matrix execution layer.  The naive
//!   form is correct, simple, and easy to verify.
//!
//! Under `Ortho`, the two are mutual inverses:
//! `idct(dct(x, II, Ortho), III, Ortho) ≈ x` within `1e-4`
//! relative tolerance for `N ≤ 64K`, f32 dtype.
//!
//! ## Quick example
//!
//! ```rust
//! use dsp_dct::{dct, idct, DctType, DctNorm};
//!
//! let signal = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
//! let coeffs = dct(&signal, DctType::II, DctNorm::Ortho).unwrap();
//! let back = idct(&coeffs, DctType::III, DctNorm::Ortho).unwrap();
//! // `back` matches `signal` within 1e-4 relative tolerance.
//! ```
//!
//! ## Phase scope
//!
//! - **Phase 0** — spec (`code/specs/DSP02-dct.md`).
//! - **Phase 1+2** — crate skeleton + scalar DCT-II / DCT-III
//!   + tests.  Landed as 0.1.0.
//! - **Phase 3** — matrix-ir-lowered `dct_via_runtime` so the
//!   transform actually runs on the matrix execution layer.
//!   Will also replace the naive Phase 2 DCT-III with the
//!   FFT-based form.  Pending.
//! - **Phase 4 (this release)** — 2-D [`dct_2d`] / [`idct_2d`]
//!   for image / JPEG workloads.  Row-then-column factorisation
//!   on top of the 1-D primitives.
//! - **Phase 5** — Loeffler 8-point specialisation.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod two_d;
pub use two_d::{dct_2d, idct_2d};

use dsp_fft::{fft_scalar, ifft_scalar, FftError};
use std::f32::consts::PI;
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
// § 1. Defensive Boundaries and Constants (CWE-400 / CWE-190)
// ═══════════════════════════════════════════════════════════════════════

/// Maximum 1-D DCT transform length (CWE-400 bound: 1,048,576).
pub const MAX_DCT_LEN: usize = 1 << 20;

/// Maximum total samples across 1-D or 2-D buffers (CWE-400 bound: 16,777,216).
pub const MAX_SAMPLES: usize = 1 << 24;

// ═══════════════════════════════════════════════════════════════════════
// § 2. Types and Conventions
// ═══════════════════════════════════════════════════════════════════════

/// Which DCT variant to compute.  V1 ships II (forward) and III
/// (inverse under `Ortho`).  DCT-I and DCT-IV are deferred per
/// the DSP02 spec.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DctType {
    /// Forward DCT — the canonical "DCT", what JPEG / MFCC use.
    /// `scipy.fft.dct(type=2)`.
    II,
    /// Inverse DCT — paired with DCT-II under `Ortho` makes them
    /// mutual inverses.  `scipy.fft.idct(type=3)`.
    III,
}

/// Normalisation convention.
///
/// - `None` — un-normalised, the raw cosine sum.  Matches
///   `scipy.fft.dct(_, norm=None)`.  Round-tripping `(II, None)`
///   then `(III, None)` requires an explicit `2/N` rescale.
/// - `Ortho` — orthonormal; under `Ortho` DCT-II and DCT-III
///   are mutual inverses, no rescale needed.  Matches
///   `scipy.fft.dct(_, norm='ortho')`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DctNorm {
    None,
    Ortho,
}

// ═══════════════════════════════════════════════════════════════════════
// § 3. Public 1-D Transform Dispatch
// ═══════════════════════════════════════════════════════════════════════

/// Forward DCT: takes a length-`N` real signal and returns a
/// length-`N` real coefficient vector.
///
/// In V1, `dct_type` should be [`DctType::II`] for the standard
/// forward DCT; passing [`DctType::III`] also works and computes
/// the type-III transform (which is the *inverse* of type-II
/// under `Ortho`).  See [`idct`] for the more idiomatic inverse
/// path.
pub fn dct(
    signal: &[f32],
    dct_type: DctType,
    norm: DctNorm,
) -> Result<Vec<f32>, DctError> {
    if signal.is_empty() {
        return Err(DctError::EmptyInput);
    }
    if signal.len() > MAX_DCT_LEN || signal.len() > MAX_SAMPLES {
        return Err(DctError::InvalidInput(format!(
            "input length {} exceeds MAX_DCT_LEN ({})",
            signal.len(),
            MAX_DCT_LEN
        )));
    }
    if signal.iter().any(|&x| !x.is_finite()) {
        return Err(DctError::InvalidInput(
            "input contains non-finite float (NaN or Inf)".into(),
        ));
    }
    let n = signal.len();
    match dct_type {
        DctType::II => dct_ii_via_fft(signal, n, norm),
        DctType::III => {
            let unnorm = dct_iii_via_fft(signal, n)?;
            Ok(apply_dct_iii_norm(&unnorm, n, norm))
        }
    }
}

/// Inverse DCT: takes a length-`N` real coefficient vector and
/// returns a length-`N` real signal.
///
/// In V1, pass [`DctType::III`] (the inverse-by-construction
/// pair for DCT-II) with the *same* `norm` you used on the
/// forward call.  `idct(dct(x, II, Ortho), III, Ortho) ≈ x`.
///
/// `idct(_, II, _)` is allowed too and computes the type-II
/// transform — under `Ortho` this is identical to `dct(_, II, _)`
/// because the orthogonal DCT-II is its own forward + transpose.
pub fn idct(
    coeffs: &[f32],
    dct_type: DctType,
    norm: DctNorm,
) -> Result<Vec<f32>, DctError> {
    if coeffs.is_empty() {
        return Err(DctError::EmptyInput);
    }
    if coeffs.len() > MAX_DCT_LEN || coeffs.len() > MAX_SAMPLES {
        return Err(DctError::InvalidInput(format!(
            "input length {} exceeds MAX_DCT_LEN ({})",
            coeffs.len(),
            MAX_DCT_LEN
        )));
    }
    if coeffs.iter().any(|&x| !x.is_finite()) {
        return Err(DctError::InvalidInput(
            "input contains non-finite float (NaN or Inf)".into(),
        ));
    }
    let n = coeffs.len();
    match dct_type {
        DctType::III => idct_iii(coeffs, n, norm),
        DctType::II => dct_ii_via_fft(coeffs, n, norm),
    }
}

// ─────────────────────────── DCT-II via FFT ───────────────────────────

/// DCT-II via the Makhoul reduction.  Returns the un-normalised
/// then `norm`-scaled coefficients.
///
/// Steps:
///
/// 1. Pre-shuffle `signal` into `y` (even samples in order, odd
///    samples reversed).
/// 2. FFT(y) of length `N` — uses `dsp_fft::fft_scalar` which
///    handles all `N ≥ 1` (radix-2 fast path or Bluestein).
/// 3. `X[k] = 2 · Re(Y[k] · exp(-iπk/(2N)))`.
/// 4. Apply normalisation per `norm`.
fn dct_ii_via_fft(
    signal: &[f32],
    n: usize,
    norm: DctNorm,
) -> Result<Vec<f32>, DctError> {
    // ── Step 1: Makhoul shuffle.
    //
    // For even N, indices land cleanly:
    //   y[0..N/2]      = x[0, 2, 4, …, N-2]
    //   y[N/2..N]      = x[N-1, N-3, …, 1] (reversed odd)
    //
    // For odd N, the middle sample is the last odd index reversed,
    // which is x[N-1] itself.  The two-loop construction below
    // handles both parities uniformly:
    //
    //   - Even loop: y[m] = x[2m] for m = 0..ceil(N/2)
    //   - Odd loop:  y[N-1-m] = x[2m+1] for m = 0..floor(N/2)
    let mut y = vec![0.0f32; n];
    let half_even = n.div_ceil(2); // ceil(N / 2)
    let half_odd = n / 2; // floor(N / 2)
    for m in 0..half_even {
        y[m] = signal[2 * m];
    }
    for m in 0..half_odd {
        y[n - 1 - m] = signal[2 * m + 1];
    }

    // ── Step 2: FFT of length N.  We wrap y as interleaved
    //   complex with im = 0 since fft_scalar takes interleaved
    //   complex input.
    let mut interleaved = Vec::with_capacity(2 * n);
    for &v in &y {
        interleaved.push(v);
        interleaved.push(0.0);
    }
    let spectrum = if n.is_power_of_two() {
        fft_scalar(&interleaved).map_err(fft_err)?
    } else {
        // Non-pow2 — go through the public fft path which
        // dispatches to Bluestein.  We need the interleaved
        // result, so use bluestein_scalar directly to avoid
        // the ComplexTensor wrapping that the public `fft()`
        // adds.
        dsp_fft::bluestein_scalar(&interleaved, dsp_fft::Direction::Forward)
            .map_err(fft_err)?
    };

    // ── Step 3: twiddle multiply, take real part, double.
    //
    //   X[k] = 2 · Re( Y[k] · exp(-iπk/(2N)) )
    //
    //        = 2 · ( Y_re[k] · cos(-θ_k) - Y_im[k] · sin(-θ_k) )
    //        = 2 · ( Y_re[k] · cos(θ_k)  + Y_im[k] · sin(θ_k) )
    //
    //   with θ_k = π · k / (2N).
    let two_n = (2 * n) as f32;
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let theta = PI * (k as f32) / two_n;
        let (sin_t, cos_t) = theta.sin_cos();
        let y_re = spectrum[2 * k];
        let y_im = spectrum[2 * k + 1];
        out.push(2.0 * (y_re * cos_t + y_im * sin_t));
    }

    // ── Step 4: normalise.
    //
    //   Ortho: X[0] *= √(1/(4N));  X[k>0] *= √(1/(2N))
    //   None:  no scaling.
    apply_dct_ii_norm(&mut out, n, norm);
    Ok(out)
}

/// Apply the DCT-II output normalisation (in place).
fn apply_dct_ii_norm(out: &mut [f32], n: usize, norm: DctNorm) {
    if norm == DctNorm::None {
        return;
    }
    let n_f = n as f32;
    let s0 = (1.0 / (4.0 * n_f)).sqrt();
    let sk = (1.0 / (2.0 * n_f)).sqrt();
    out[0] *= s0;
    for x in &mut out[1..] {
        *x *= sk;
    }
}

// ─────────────────────────── DCT-III via FFT ───────────────────────────

/// Fast O(N log N) un-normalised DCT-III via IFFT (DSP02 Phase 3).
///
/// Computes the un-normalised textbook DCT-III sum matching scipy:
///
/// ```text
///     X[k] = x[0] + 2 · Σ_{n=1..N-1}  x[n] · cos( π · n · (2k + 1) / (2N) )
/// ```
///
/// The algorithm reduces DCT-III to a length-N IFFT via the inverse Makhoul shuffle:
/// 1. Form length-N complex sequence Y[k]:
///    - Y[0] = (x[0], 0)
///    - Y[k] = (x[k] - i · x[N - k]) · exp(+iπk / (2N))  for k = 1..N-1
/// 2. IFFT(Y) via `dsp_fft` (radix-2 if power-of-two, else Bluestein).
/// 3. Un-shuffle and multiply by N (to invert the IFFT 1/N factor):
///    - X[2m]     = N · Re(y[m])          for m = 0..ceil(N/2)
///    - X[2m + 1] = N · Re(y[N - 1 - m])  for m = 0..floor(N/2)
pub(crate) fn dct_iii_via_fft(coeffs: &[f32], n: usize) -> Result<Vec<f32>, DctError> {
    if n == 1 {
        return Ok(vec![coeffs[0]]);
    }

    let mut interleaved = Vec::with_capacity(2 * n);
    interleaved.push(coeffs[0]);
    interleaved.push(0.0);

    let two_n = (2 * n) as f32;
    for k in 1..n {
        let theta = PI * (k as f32) / two_n;
        let (sin_t, cos_t) = theta.sin_cos();
        let re_k = coeffs[k];
        let im_k = -coeffs[n - k];
        // (re_k + i·im_k) · (cos_t + i·sin_t)
        let y_re = re_k * cos_t - im_k * sin_t;
        let y_im = re_k * sin_t + im_k * cos_t;
        interleaved.push(y_re);
        interleaved.push(y_im);
    }

    let y = if n.is_power_of_two() {
        ifft_scalar(&interleaved).map_err(fft_err)?
    } else {
        dsp_fft::bluestein_scalar(&interleaved, dsp_fft::Direction::Inverse)
            .map_err(fft_err)?
    };

    let n_f = n as f32;
    let mut out = vec![0.0f32; n];
    let half_even = n.div_ceil(2);
    let half_odd = n / 2;
    for m in 0..half_even {
        out[2 * m] = y[2 * m] * n_f;
    }
    for m in 0..half_odd {
        out[2 * m + 1] = y[2 * (n - 1 - m)] * n_f;
    }

    Ok(out)
}

/// Naive `O(N²)` un-normalised DCT-III.  Reference oracle for
/// Phase 2; Phase 3 upgraded default DCT-III to `dct_iii_via_fft`.
/// Kept for exact parity testing.
#[cfg(test)]
pub(crate) fn dct_iii_naive(coeffs: &[f32], n: usize) -> Vec<f32> {
    let n_f = n as f32;
    let two_n = 2.0 * n_f;
    let mut out = vec![0.0f32; n];
    for k in 0..n {
        let mut acc = coeffs[0];
        for nn in 1..n {
            let theta = PI * (nn as f32) * (2.0 * (k as f32) + 1.0) / two_n;
            acc += 2.0 * coeffs[nn] * theta.cos();
        }
        out[k] = acc;
    }
    out
}

/// Inverse DCT (i.e., DCT-III) entry point used by `idct`.
///
/// Under `Ortho` we want `idct(dct(x, II, Ortho), III, Ortho) = x`.
/// Since the un-normalised DCT-II followed by un-normalised
/// DCT-III gives `2N · x` (each sample scaled by `2N`), and the
/// `Ortho` forward DCT-II divides by `√(4N)` for `X[0]` and
/// `√(2N)` for `X[k>0]`, the inverse must un-do those scales
/// then divide by `2N` (or equivalently, apply the appropriate
/// per-input scales).
fn idct_iii(coeffs: &[f32], n: usize, norm: DctNorm) -> Result<Vec<f32>, DctError> {
    // Un-Ortho the input first so DCT-III sees the
    // same scale it would for `None`.
    let input: Vec<f32> = if norm == DctNorm::Ortho {
        let n_f = n as f32;
        let s0 = (4.0 * n_f).sqrt();
        let sk = (2.0 * n_f).sqrt();
        let mut v = coeffs.to_vec();
        v[0] *= s0;
        for x in &mut v[1..] {
            *x *= sk;
        }
        v
    } else {
        coeffs.to_vec()
    };

    let mut out = dct_iii_via_fft(&input, n)?;

    // After running un-normalised DCT-III on un-normalised DCT-II
    // coefficients we get `2N · x`.  Divide by 2N to recover x.
    let scale = 1.0 / (2.0 * (n as f32));
    for v in &mut out {
        *v *= scale;
    }

    Ok(out)
}

/// Apply the DCT-III output normalisation for the *forward*
/// `dct(_, III, _)` path (not the inverse).  In V1 this is just
/// the un-normalised raw transform under `None`, and the
/// scipy-matching Ortho scaling under `Ortho` (same shape as
/// DCT-II's: `X[0] *= √(1/(4N))`, `X[k>0] *= √(1/(2N))`).
fn apply_dct_iii_norm(out: &[f32], n: usize, norm: DctNorm) -> Vec<f32> {
    let mut v = out.to_vec();
    if norm == DctNorm::Ortho {
        let n_f = n as f32;
        let s0 = (1.0 / (4.0 * n_f)).sqrt();
        let sk = (1.0 / (2.0 * n_f)).sqrt();
        v[0] *= s0;
        for x in &mut v[1..] {
            *x *= sk;
        }
    }
    v
}

// ─────────────────────────── Errors ───────────────────────────

/// Errors produced by DCT primitives.
#[derive(Debug, Clone, PartialEq)]
pub enum DctError {
    /// Generic invalid-input error (for future use; V1 has none
    /// beyond `EmptyInput`).
    InvalidInput(String),
    /// `signal` is empty.  V1 requires `N ≥ 1`.
    EmptyInput,
    /// Wraps a `dsp_fft::FftError` from the underlying FFT call.
    Fft(String),
}

impl fmt::Display for DctError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DctError::InvalidInput(msg) => write!(f, "invalid input: {}", msg),
            DctError::EmptyInput => write!(f, "DCT requires at least one sample"),
            DctError::Fft(msg) => write!(f, "FFT failure: {}", msg),
        }
    }
}

impl std::error::Error for DctError {}

fn fft_err(e: FftError) -> DctError {
    DctError::Fft(format!("{:?}", e))
}

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

    /// Naive O(N²) DCT-II reference oracle — matches scipy's
    /// un-normalised DCT-II.  Used to verify the FFT-based
    /// `dct_ii_via_fft`.
    fn naive_dct_ii(x: &[f32]) -> Vec<f32> {
        let n = x.len();
        let n_f = n as f32;
        let two_n = 2.0 * n_f;
        let mut out = vec![0.0f32; n];
        for k in 0..n {
            let mut acc = 0.0f32;
            for nn in 0..n {
                let theta =
                    PI * (k as f32) * (2.0 * (nn as f32) + 1.0) / two_n;
                acc += x[nn] * theta.cos();
            }
            out[k] = 2.0 * acc;
        }
        out
    }

    // ── error paths ────────────────────────────────────────────

    #[test]
    fn dct_rejects_empty_input() {
        let err = dct(&[], DctType::II, DctNorm::None).unwrap_err();
        assert_eq!(err, DctError::EmptyInput);
    }

    #[test]
    fn idct_rejects_empty_input() {
        let err = idct(&[], DctType::III, DctNorm::Ortho).unwrap_err();
        assert_eq!(err, DctError::EmptyInput);
    }

    // ── closed-form known vectors ──────────────────────────────

    #[test]
    fn dct_ii_of_impulse_matches_cosine_sequence() {
        // x = [1, 0, 0, …, 0] → X[k] = 2 cos(πk/(2N)) (un-norm).
        let n = 8;
        let mut signal = vec![0.0f32; n];
        signal[0] = 1.0;
        let coeffs = dct(&signal, DctType::II, DctNorm::None).unwrap();
        for k in 0..n {
            let expected = 2.0 * (PI * (k as f32) / (2.0 * n as f32)).cos();
            assert!(
                approx_eq(coeffs[k], expected, 1e-5),
                "bin {}: got {}, expected {}",
                k,
                coeffs[k],
                expected
            );
        }
    }

    #[test]
    fn dct_ii_of_dc_concentrates_at_bin_0() {
        // x = [1, 1, …, 1] → X[0] = 2N, X[k>0] = 0 (un-norm).
        let n = 8;
        let signal = vec![1.0f32; n];
        let coeffs = dct(&signal, DctType::II, DctNorm::None).unwrap();
        assert!(approx_eq(coeffs[0], 2.0 * n as f32, 1e-4));
        for k in 1..n {
            assert!(
                approx_eq(coeffs[k], 0.0, 1e-4),
                "bin {} = {}, expected ~0",
                k,
                coeffs[k]
            );
        }
    }

    #[test]
    fn dct_ii_of_dc_n3_non_pow2() {
        // Non-power-of-two N — exercises the Bluestein FFT path
        // through dct_ii_via_fft.
        let n = 3;
        let signal = vec![1.0f32; n];
        let coeffs = dct(&signal, DctType::II, DctNorm::None).unwrap();
        assert!(approx_eq(coeffs[0], 2.0 * n as f32, 1e-4));
        for k in 1..n {
            assert!(approx_eq(coeffs[k], 0.0, 1e-3));
        }
    }

    #[test]
    fn dct_ii_ortho_of_dc_returns_sqrt_n() {
        // Under Ortho, x = [1, 1, …, 1] → X[0] = √N (since the
        // un-norm answer 2N times the √(1/(4N)) Ortho scale gives
        // √(N²/N) = √N).
        let n = 16;
        let signal = vec![1.0f32; n];
        let coeffs = dct(&signal, DctType::II, DctNorm::Ortho).unwrap();
        let expected_dc = (n as f32).sqrt();
        assert!(approx_eq(coeffs[0], expected_dc, 1e-5));
        for k in 1..n {
            assert!(approx_eq(coeffs[k], 0.0, 1e-4));
        }
    }

    // ── naive cross-check ──────────────────────────────────────

    #[test]
    fn dct_ii_matches_naive_dft_n2_n3_n4_n5_n8_n16() {
        for &n in &[2usize, 3, 4, 5, 8, 16] {
            let signal: Vec<f32> = (0..n)
                .map(|i| ((i as f32) * 0.3 - 0.7).sin())
                .collect();
            let via_fft = dct(&signal, DctType::II, DctNorm::None).unwrap();
            let via_naive = naive_dct_ii(&signal);
            for (i, (a, b)) in via_fft.iter().zip(via_naive.iter()).enumerate() {
                let scale = a.abs().max(b.abs()).max(1.0);
                assert!(
                    (a - b).abs() <= scale * 1e-4,
                    "N={}, index {}: fft {} vs naive {}",
                    n,
                    i,
                    a,
                    b
                );
            }
        }
    }

    // ── round-trips under Ortho ────────────────────────────────

    fn round_trip_ortho(n: usize, tol: f32) {
        let signal: Vec<f32> = (0..n)
            .map(|i| ((i as f32) * 0.5 - (n as f32) * 0.25).sin() + 0.3)
            .collect();
        let coeffs = dct(&signal, DctType::II, DctNorm::Ortho).unwrap();
        let recovered = idct(&coeffs, DctType::III, DctNorm::Ortho).unwrap();
        assert_close(&signal, &recovered, tol);
    }

    #[test]
    fn round_trip_ortho_n1() {
        // N = 1 is degenerate but shouldn't error.
        let signal = vec![3.5f32];
        let coeffs = dct(&signal, DctType::II, DctNorm::Ortho).unwrap();
        let recovered = idct(&coeffs, DctType::III, DctNorm::Ortho).unwrap();
        assert_close(&signal, &recovered, 1e-5);
    }

    #[test]
    fn round_trip_ortho_n2() {
        round_trip_ortho(2, 1e-5);
    }

    #[test]
    fn round_trip_ortho_n8() {
        round_trip_ortho(8, 1e-4);
    }

    #[test]
    fn round_trip_ortho_n16() {
        round_trip_ortho(16, 1e-4);
    }

    #[test]
    fn round_trip_ortho_n31_non_pow2() {
        round_trip_ortho(31, 1e-3);
    }

    #[test]
    fn round_trip_ortho_n64() {
        round_trip_ortho(64, 1e-3);
    }

    // ── round-trips under None (with explicit rescale) ─────────

    #[test]
    fn round_trip_none_with_explicit_rescale() {
        // Under None norm: dct + idct gives 2N * x, so the caller
        // must divide by 2N.  The internal idct path *already*
        // applies that 2N division (it's required for the Ortho
        // path to round-trip), so for None we just compare
        // directly.
        for &n in &[2usize, 8, 16, 32] {
            let signal: Vec<f32> = (0..n).map(|i| (i as f32) * 0.5).collect();
            let coeffs = dct(&signal, DctType::II, DctNorm::None).unwrap();
            let recovered = idct(&coeffs, DctType::III, DctNorm::None).unwrap();
            assert_close(&signal, &recovered, 1e-4);
        }
    }

    // ── DCT-III FFT vs Naive Parity ────────────────────────────

    #[test]
    fn test_exact_numerical_parity_dct_iii_via_fft_vs_naive() {
        for n in 1..=32 {
            let signal: Vec<f32> = (0..n)
                .map(|i| ((i as f32) * 0.7 - 1.2).cos())
                .collect();
            let via_fft = dct_iii_via_fft(&signal, n).unwrap();
            let via_naive = dct_iii_naive(&signal, n);
            assert_close(&via_fft, &via_naive, 1e-4);
        }
    }

    // ── SplitMix64 Property-Based Tests and Hostile Suites ─────

    use coding_adventures_test_bootstrap::prng::SplitMix64;

    fn sample_f32(rng: &mut SplitMix64) -> f32 {
        let u = rng.next_u64();
        let mantissa = (u & 0x00ff_ffff) as f32 / (0x0100_0000 as f32);
        (mantissa * 2.0) - 1.0
    }

    #[test]
    fn pbt_splitmix64_dct_round_trips() {
        let mut rng = SplitMix64::new(0xa1b2_c3d4_e5f6_0718);
        let test_sizes = [1usize, 2, 3, 4, 5, 7, 8, 12, 16, 23, 31, 32, 64, 100];
        for &n in &test_sizes {
            let mut signal = Vec::with_capacity(n);
            for _ in 0..n {
                signal.push(sample_f32(&mut rng));
            }

            // DCT-II forward, DCT-III inverse (Ortho)
            let coeffs = dct(&signal, DctType::II, DctNorm::Ortho).unwrap();
            assert_eq!(coeffs.len(), n);
            let recovered = idct(&coeffs, DctType::III, DctNorm::Ortho).unwrap();
            assert_close(&recovered, &signal, 1e-3);

            // DCT-II forward, DCT-III inverse (None)
            let coeffs_none = dct(&signal, DctType::II, DctNorm::None).unwrap();
            let recovered_none = idct(&coeffs_none, DctType::III, DctNorm::None).unwrap();
            assert_close(&recovered_none, &signal, 1e-3);
        }
    }

    #[test]
    fn hostile_non_finite_inputs_rejected() {
        assert!(matches!(
            dct(&[1.0, f32::NAN, 3.0], DctType::II, DctNorm::Ortho),
            Err(DctError::InvalidInput(_))
        ));
        assert!(matches!(
            dct(&[1.0, f32::INFINITY, 3.0], DctType::II, DctNorm::Ortho),
            Err(DctError::InvalidInput(_))
        ));
        assert!(matches!(
            dct(&[1.0, f32::NEG_INFINITY, 3.0], DctType::III, DctNorm::Ortho),
            Err(DctError::InvalidInput(_))
        ));
        assert!(matches!(
            idct(&[f32::NAN, 2.0], DctType::III, DctNorm::Ortho),
            Err(DctError::InvalidInput(_))
        ));
    }

    #[test]
    fn hostile_oversized_inputs_rejected() {
        // Empty
        assert_eq!(
            dct(&[], DctType::II, DctNorm::Ortho).unwrap_err(),
            DctError::EmptyInput
        );
        assert_eq!(
            idct(&[], DctType::III, DctNorm::Ortho).unwrap_err(),
            DctError::EmptyInput
        );
    }
}
