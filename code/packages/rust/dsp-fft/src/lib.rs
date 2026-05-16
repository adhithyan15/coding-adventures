//! # `dsp-fft` — FFT / IFFT for the DSP layer
//!
//! Pure-Rust radix-2 Cooley-Tukey FFT / inverse FFT.  Operates on
//! interleaved `[re, im, re, im, …]` `f32` buffers — same layout
//! as `dsp-complex::ComplexTensor`.
//!
//! ## Phase scope
//!
//! - **Phase 2** — scalar reference (`fft_scalar` / `ifft_scalar`).
//!   Stays as the test oracle for every later phase.
//! - **Phase 3a / 3b.i** — Slice + Concat ops landed in
//!   `matrix-ir`.
//! - **Phase 3b.ii** — `build_fft_graph` emits a `matrix_ir::Graph`
//!   that computes the radix-2 FFT entirely in generic tensor ops.
//! - **Phase 3b.iii** — `fft_via_runtime` plans + dispatches that
//!   graph through `matrix-runtime` + `matrix-cpu`, downloading the
//!   spectrum.
//! - **Phase 3b.iv** — the public [`fft`] entry point routes
//!   real-valued power-of-two inputs through `fft_via_runtime`
//!   instead of the scalar reference.
//! - **Phase 4a (this release)** — scalar Bluestein
//!   ([`bluestein_scalar`]) handles arbitrary (non-power-of-two)
//!   lengths.  The public [`fft`] / [`ifft`] now accept any
//!   `N ≥ 1`; non-pow2 inputs fall through to Bluestein.
//!   Complex-input transforms also delegate to Bluestein for
//!   non-pow2 `N`.
//! - **Phase 4b** — `rfft` / `irfft` (real-input half-spectrum
//!   API exploiting conjugate symmetry).
//! - **Phase 4c** — Matrix-ir-lowered Bluestein so non-pow2
//!   FFTs also lift onto the matrix execution layer.
//! - **Phase 5** — MX05 specialised emitters (folded twiddles for
//!   canonical sizes 8…1024).
//!
//! ## Algorithm
//!
//! Standard decimation-in-time radix-2 FFT:
//!
//! 1. Bit-reverse the input in place.
//! 2. For each stage `s = 1..=log2(N)` with `half = 2^(s-1)` and
//!    `full = 2^s`:
//!    - For each `block_start` in `(0..N).step_by(full)`:
//!      - For each `j` in `0..half`:
//!        - `t = twiddle[j * (N / full)] * x[block_start + j + half]`
//!        - `x[block_start + j + half] = x[block_start + j] - t`
//!        - `x[block_start + j]        = x[block_start + j] + t`
//!
//! Inverse FFT uses positive-sign twiddles and divides every output
//! element by `N` at the end ("backward" normalization — matches
//! numpy / scipy / MATLAB defaults).
//!
//! ## Numerical accuracy
//!
//! V1 contract: `ifft(fft(x))` round-trips within `1e-5` relative
//! tolerance for `N ≤ 65536`, f32 dtype.  Snapshot vectors (impulse,
//! DC, single-bin sinusoid) hit closed-form known values.

#![warn(rust_2018_idioms)]

pub mod bluestein;
pub mod radix2;
pub use bluestein::bluestein_scalar;
pub use radix2::{build_fft_graph, build_fft_graph_with_input, fft_via_runtime};

use dsp_complex::ComplexTensor;
use std::fmt;

/// Forward FFT on interleaved `[re, im]` data.  Length of `signal`
/// must be `2 * N` where `N` is a power of two; returns a buffer of
/// the same length holding the natural-order spectrum.
///
/// Use [`fft`] (which takes a real or complex slice) for the
/// higher-level API.
pub fn fft_scalar(signal: &[f32]) -> Result<Vec<f32>, FftError> {
    if signal.len() % 2 != 0 {
        return Err(FftError::InvalidInput(format!(
            "interleaved buffer must have even length; got {}",
            signal.len()
        )));
    }
    let n = signal.len() / 2;
    require_pow2_length(n)?;
    let mut buf = signal.to_vec();
    bit_reversal_permute(&mut buf, n);
    butterflies(&mut buf, n, Direction::Forward);
    Ok(buf)
}

/// Inverse FFT on interleaved `[re, im]` data.  Output is divided
/// by `N` to satisfy the "backward" normalization convention.
///
/// Length of `spectrum` must be `2 * N` where `N` is a power of two.
pub fn ifft_scalar(spectrum: &[f32]) -> Result<Vec<f32>, FftError> {
    if spectrum.len() % 2 != 0 {
        return Err(FftError::InvalidInput(format!(
            "interleaved buffer must have even length; got {}",
            spectrum.len()
        )));
    }
    let n = spectrum.len() / 2;
    require_pow2_length(n)?;
    let mut buf = spectrum.to_vec();
    bit_reversal_permute(&mut buf, n);
    butterflies(&mut buf, n, Direction::Inverse);
    // Backward normalization: divide by N.
    let inv_n = 1.0 / (n as f32);
    for x in buf.iter_mut() {
        *x *= inv_n;
    }
    Ok(buf)
}

/// Compute the forward 1-D FFT of a real-valued or already-complex
/// `signal`.
///
/// - If `signal` has length `N` (real input):
///   - **Power-of-two `N ≥ 2`** routes through the matrix-ir-lowered
///     FFT graph via
///     [`fft_via_runtime`](crate::radix2::fft_via_runtime) — the
///     FFT actually runs on the matrix execution layer.  When
///     `matrix-metal` / `matrix-cuda` claim `Slice` + `Concat` in
///     their `supported_ops` bitsets, the call lifts to GPU
///     automatically with no `dsp-fft` change required.
///   - **Non-power-of-two `N`** (new in Phase 4a) falls back to the
///     scalar Bluestein algorithm
///     ([`bluestein_scalar`](crate::bluestein::bluestein_scalar)).
///     Still CPU-only; Phase 4c will lower it to matrix-ir.
///   - **`N = 1`** is the identity, handled by the scalar path.
/// - If `signal` has length `2N` (interleaved complex), pass
///   `complex: true`.  Power-of-two `N` uses [`fft_scalar`];
///   non-pow2 `N` (also new in Phase 4a) uses Bluestein.
///
/// Returns a `ComplexTensor` of `N` complex elements.
///
/// **Phase 4a**: the public API now accepts arbitrary `N ≥ 1`.
/// Previously non-power-of-two lengths returned
/// `FftError::NotPowerOfTwo`.
pub fn fft(signal: &[f32], complex: bool) -> Result<ComplexTensor, FftError> {
    let interleaved = if complex {
        // Complex input: route by N parity to either the radix-2
        // scalar oracle (power-of-two) or Bluestein (anything else).
        // Both share the interleaved `[re, im]` buffer convention so
        // we can dispatch with no buffer rewrap.
        if signal.len() % 2 != 0 {
            return Err(FftError::InvalidInput(format!(
                "interleaved complex buffer must have even length; got {}",
                signal.len()
            )));
        }
        let n = signal.len() / 2;
        if n.is_power_of_two() && n >= 1 {
            fft_scalar(signal)?
        } else {
            bluestein_scalar(signal, Direction::Forward)?
        }
    } else if signal.len() < 2 {
        // Degenerate N < 2 case (empty or single real sample).
        // The matrix-ir graph builder requires N ≥ 2; fall back to
        // the scalar oracle so the public API behavior is unchanged
        // for these edge cases — empty still returns `InvalidInput`,
        // single-element passes through as the identity.
        let mut interleaved_real = Vec::with_capacity(signal.len() * 2);
        for &x in signal {
            interleaved_real.push(x);
            interleaved_real.push(0.0);
        }
        fft_scalar(&interleaved_real)?
    } else if signal.len().is_power_of_two() {
        // Real input, N ≥ 2 power-of-two: matrix-ir-lowered path.
        fft_via_runtime(signal, Direction::Forward)?
    } else {
        // Real input, non-power-of-two N: wrap as interleaved
        // complex with im = 0 then call Bluestein.  No matrix-ir
        // path yet (Phase 4c).
        let mut interleaved_real = Vec::with_capacity(signal.len() * 2);
        for &x in signal {
            interleaved_real.push(x);
            interleaved_real.push(0.0);
        }
        bluestein_scalar(&interleaved_real, Direction::Forward)?
    };
    Ok(ComplexTensor::from_interleaved(interleaved).expect("fft output has even length"))
}

/// Compute the inverse 1-D FFT.  Input is the interleaved spectrum
/// from [`fft`] (or an external source matching the convention);
/// output is a complex `ComplexTensor` whose `real()` part is the
/// recovered signal when the input was real-valued.
///
/// Power-of-two `N` runs through [`ifft_scalar`] (still the scalar
/// oracle — the matrix-ir graph builder for inverse-from-complex
/// is a future phase); non-power-of-two `N` (new in Phase 4a) runs
/// through the scalar Bluestein algorithm with
/// `Direction::Inverse`, applying the same `1/N` backward
/// normalization.
pub fn ifft(spectrum: &ComplexTensor) -> Result<ComplexTensor, FftError> {
    let interleaved = spectrum.as_interleaved();
    let n = spectrum.len();
    let result = if n.is_power_of_two() && n >= 1 {
        ifft_scalar(interleaved)?
    } else {
        bluestein_scalar(interleaved, Direction::Inverse)?
    };
    Ok(ComplexTensor::from_interleaved(result).expect("ifft output has even length"))
}

// ─────────────────────────── Internals ───────────────────────────

/// FFT direction.  `Forward` is the standard discrete Fourier
/// transform with `exp(-2πi · k · n / N)` twiddles; `Inverse` uses
/// the conjugate (`exp(+2πi · ...)`) and divides every output
/// element by `N` (backward normalization).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Inverse,
}

fn require_pow2_length(n: usize) -> Result<(), FftError> {
    if n == 0 {
        return Err(FftError::InvalidInput(
            "FFT length must be at least 1".into(),
        ));
    }
    if !n.is_power_of_two() {
        return Err(FftError::NotPowerOfTwo(n));
    }
    Ok(())
}

/// Reorder the interleaved buffer in-place so element at index `i`
/// moves to index `bit_reverse(i, log2(n))`.
///
/// Each complex element is two consecutive f32s, so we swap pairs.
fn bit_reversal_permute(buf: &mut [f32], n: usize) {
    let log_n = (n as u64).trailing_zeros() as usize;
    for i in 0..n {
        let j = bit_reverse(i, log_n);
        if j > i {
            buf.swap(2 * i, 2 * j);
            buf.swap(2 * i + 1, 2 * j + 1);
        }
    }
}

fn bit_reverse(mut x: usize, bits: usize) -> usize {
    let mut r = 0usize;
    for _ in 0..bits {
        r = (r << 1) | (x & 1);
        x >>= 1;
    }
    r
}

/// Run the radix-2 butterfly stages on a bit-reversed buffer in
/// place.  Direction picks the twiddle sign and (for Inverse) leaves
/// the `/N` scaling to the caller.
fn butterflies(buf: &mut [f32], n: usize, dir: Direction) {
    use std::f32::consts::PI;
    let sign: f32 = match dir {
        Direction::Forward => -1.0,
        Direction::Inverse => 1.0,
    };
    let mut full = 2usize;
    while full <= n {
        let half = full / 2;
        // Twiddle base: e^(sign * 2π i / full) — recurrence to avoid
        // calling sin/cos in the inner loop would help perf but
        // hurt accuracy.  Phase 2 is the reference; clarity beats
        // speed here.
        for block_start in (0..n).step_by(full) {
            for j in 0..half {
                let theta = sign * 2.0 * PI * (j as f32) / (full as f32);
                let (w_im, w_re) = theta.sin_cos();

                // x = buf[2 * (block_start + j) .. +2]
                // y = buf[2 * (block_start + j + half) .. +2]
                let x_idx = 2 * (block_start + j);
                let y_idx = 2 * (block_start + j + half);
                let x_re = buf[x_idx];
                let x_im = buf[x_idx + 1];
                let y_re = buf[y_idx];
                let y_im = buf[y_idx + 1];

                // t = w * y  (complex multiply)
                let t_re = w_re * y_re - w_im * y_im;
                let t_im = w_re * y_im + w_im * y_re;

                // buf[x_idx] = x + t, buf[y_idx] = x - t
                buf[x_idx] = x_re + t_re;
                buf[x_idx + 1] = x_im + t_im;
                buf[y_idx] = x_re - t_re;
                buf[y_idx + 1] = x_im - t_im;
            }
        }
        full *= 2;
    }
}

/// Errors produced by FFT primitives.
#[derive(Debug, Clone, PartialEq)]
pub enum FftError {
    /// `signal` has odd length, wrong dtype, etc.
    InvalidInput(String),
    /// V1 (Phase 2/3) requires power-of-two N.  Phase 4 will add
    /// Bluestein for arbitrary lengths.
    NotPowerOfTwo(usize),
}

impl fmt::Display for FftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FftError::InvalidInput(msg) => write!(f, "invalid input: {}", msg),
            FftError::NotPowerOfTwo(n) => write!(
                f,
                "FFT length {} is not a power of two; Phase 4 will add Bluestein for arbitrary N",
                n
            ),
        }
    }
}

impl std::error::Error for FftError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Loose float equality.  FFT round-trips accumulate ~ULP per
    /// stage; for N ≤ 64K, a relative tolerance of 1e-4 is achievable
    /// with the naive algorithm (Kahan summation would tighten it).
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

    // ── error path ─────────────────────────────────────────────

    #[test]
    fn fft_rejects_odd_interleaved_length() {
        let err = fft_scalar(&[1.0, 2.0, 3.0]).unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    #[test]
    fn fft_rejects_non_power_of_two_length() {
        // 3 complex elements → 6 floats; 3 isn't a power of two.
        let err = fft_scalar(&[1.0, 0.0, 2.0, 0.0, 3.0, 0.0]).unwrap_err();
        assert_eq!(err, FftError::NotPowerOfTwo(3));
    }

    #[test]
    fn fft_rejects_empty_signal() {
        let err = fft_scalar(&[]).unwrap_err();
        assert!(matches!(err, FftError::InvalidInput(_)));
    }

    // ── known vectors ──────────────────────────────────────────

    #[test]
    fn fft_of_impulse_is_all_ones() {
        // Impulse at index 0: [1, 0, 0, 0, …] (interleaved [1, 0, 0, 0, 0, 0, …])
        let n = 8;
        let mut signal = vec![0.0f32; 2 * n];
        signal[0] = 1.0;
        let spectrum = fft_scalar(&signal).unwrap();
        // Spectrum should be [1, 0, 1, 0, 1, 0, …] (1.0 in every real bin).
        for k in 0..n {
            assert!(
                approx_eq(spectrum[2 * k], 1.0, 1e-6),
                "real bin {} = {}, expected 1.0",
                k,
                spectrum[2 * k]
            );
            assert!(
                approx_eq(spectrum[2 * k + 1], 0.0, 1e-6),
                "imag bin {} = {}, expected 0.0",
                k,
                spectrum[2 * k + 1]
            );
        }
    }

    #[test]
    fn fft_of_dc_is_single_bin() {
        // DC: [1, 1, 1, 1, …]
        let n = 8;
        let signal: Vec<f32> = (0..n).flat_map(|_| [1.0f32, 0.0]).collect();
        let spectrum = fft_scalar(&signal).unwrap();
        // Spectrum: [N, 0, 0, 0, …]
        assert!(approx_eq(spectrum[0], n as f32, 1e-5));
        assert!(approx_eq(spectrum[1], 0.0, 1e-5));
        for k in 1..n {
            assert!(approx_eq(spectrum[2 * k], 0.0, 1e-4));
            assert!(approx_eq(spectrum[2 * k + 1], 0.0, 1e-4));
        }
    }

    #[test]
    fn fft_of_pure_cosine_concentrates_in_two_bins() {
        // x[n] = cos(2π · k0 · n / N) → bins k0 and N-k0 each get N/2.
        let n: usize = 16;
        let k0: usize = 3;
        let signal: Vec<f32> = (0..n)
            .flat_map(|n_idx| {
                let x = (2.0 * PI * (k0 as f32) * (n_idx as f32) / (n as f32)).cos();
                [x, 0.0f32]
            })
            .collect();
        let spectrum = fft_scalar(&signal).unwrap();
        let half = (n as f32) / 2.0;
        // Bin k0
        let mag_k0 =
            (spectrum[2 * k0].powi(2) + spectrum[2 * k0 + 1].powi(2)).sqrt();
        assert!(
            (mag_k0 - half).abs() < 1e-3,
            "bin {} magnitude = {}, expected {}",
            k0,
            mag_k0,
            half
        );
        // Bin N-k0
        let kn = n - k0;
        let mag_kn = (spectrum[2 * kn].powi(2) + spectrum[2 * kn + 1].powi(2)).sqrt();
        assert!((mag_kn - half).abs() < 1e-3);
        // Other bins should be near zero.
        for k in 0..n {
            if k == k0 || k == kn {
                continue;
            }
            let mag = (spectrum[2 * k].powi(2) + spectrum[2 * k + 1].powi(2)).sqrt();
            assert!(mag < 1e-3, "bin {} = {} should be ~0", k, mag);
        }
    }

    // ── round-trip ─────────────────────────────────────────────

    #[test]
    fn round_trip_recovers_real_signal_n8() {
        let original: Vec<f32> =
            (0..8).flat_map(|n| [(n as f32) * 0.5 - 1.5, 0.0]).collect();
        let spectrum = fft_scalar(&original).unwrap();
        let recovered = ifft_scalar(&spectrum).unwrap();
        assert_close(&original, &recovered, 1e-5);
    }

    #[test]
    fn round_trip_recovers_random_complex_n64() {
        // Pseudorandom (deterministic) complex signal.
        let n: usize = 64;
        let mut original = Vec::with_capacity(2 * n);
        let mut state: u32 = 0xDEAD_BEEF;
        for _ in 0..(2 * n) {
            // xorshift32
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            // Map to [-1, 1].
            let f = ((state as f32) / (u32::MAX as f32)) * 2.0 - 1.0;
            original.push(f);
        }
        let spectrum = fft_scalar(&original).unwrap();
        let recovered = ifft_scalar(&spectrum).unwrap();
        // N=64 needs slightly looser tolerance due to log2(64)=6 stages.
        assert_close(&original, &recovered, 1e-4);
    }

    #[test]
    fn round_trip_works_for_n_up_to_1024() {
        for &log_n in &[1u32, 2, 4, 6, 8, 10] {
            let n = 1usize << log_n;
            let original: Vec<f32> = (0..n)
                .flat_map(|i| [((i as f32) * 0.1).sin(), ((i as f32) * 0.07).cos()])
                .collect();
            let spectrum = fft_scalar(&original).unwrap();
            let recovered = ifft_scalar(&spectrum).unwrap();
            // Larger N → more accumulated error.  1e-3 covers N up to 1024.
            assert_close(&original, &recovered, 1e-3);
        }
    }

    // ── public API (forwarding) ─────────────────────────────────

    #[test]
    fn public_fft_wraps_real_signal() {
        let real = vec![1.0f32, 0.0, 0.0, 0.0];
        let spectrum = fft(&real, false).unwrap();
        assert_eq!(spectrum.len(), 4);
        // Same as the impulse known-vector test:
        for k in 0..4 {
            let re = spectrum.as_interleaved()[2 * k];
            assert!(approx_eq(re, 1.0, 1e-6), "bin {} real = {}", k, re);
        }
    }

    #[test]
    fn public_ifft_round_trips_via_complex_tensor() {
        let real = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let spectrum = fft(&real, false).unwrap();
        let recovered = ifft(&spectrum).unwrap();
        // The recovered tensor's real() part should match `real`.
        let rec_real = recovered.real();
        assert_close(&real, &rec_real, 1e-4);
    }

    #[test]
    fn public_fft_accepts_already_complex_input() {
        let interleaved = vec![1.0f32, 2.0, 3.0, 4.0]; // 2 complex elements
        let spectrum = fft(&interleaved, true).unwrap();
        assert_eq!(spectrum.len(), 2);
        // DC bin should be (1+3, 2+4) = (4, 6).
        assert!(approx_eq(spectrum.as_interleaved()[0], 4.0, 1e-5));
        assert!(approx_eq(spectrum.as_interleaved()[1], 6.0, 1e-5));
    }

    // ── Phase 3b.iv: public real-input fft goes through the
    //    matrix-ir graph end-to-end via `fft_via_runtime`.  These
    //    tests verify the routing produces the same answers as the
    //    scalar oracle (which is what the previous Phase 2 wrapper
    //    was forwarding to).  Tolerance is 1e-4 to match the
    //    Phase 3b.iii contract for the matrix-runtime path.

    #[test]
    fn public_fft_real_matches_scalar_via_runtime_n8() {
        let real: Vec<f32> = (0..8).map(|i| (i as f32) * 0.5 - 1.5).collect();
        // New routing.
        let via_runtime = fft(&real, false).unwrap();
        // Scalar oracle: wrap as interleaved and call fft_scalar.
        let mut interleaved = Vec::with_capacity(real.len() * 2);
        for &x in &real {
            interleaved.push(x);
            interleaved.push(0.0);
        }
        let scalar = fft_scalar(&interleaved).unwrap();
        assert_close(via_runtime.as_interleaved(), &scalar, 1e-4);
    }

    #[test]
    fn public_fft_real_matches_scalar_via_runtime_n16_sinusoid() {
        // Pure sinusoid — exercises every butterfly stage's twiddle
        // arithmetic, not just the trivial diagonal of the impulse.
        let n: usize = 16;
        let real: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * 3.0 * (i as f32) / (n as f32)).cos())
            .collect();
        let via_runtime = fft(&real, false).unwrap();
        let mut interleaved = Vec::with_capacity(real.len() * 2);
        for &x in &real {
            interleaved.push(x);
            interleaved.push(0.0);
        }
        let scalar = fft_scalar(&interleaved).unwrap();
        assert_close(via_runtime.as_interleaved(), &scalar, 1e-4);
    }

    #[test]
    fn public_fft_real_impulse_via_runtime() {
        // Closed-form: fft(impulse) = all ones.  This is the
        // canonical "the runtime path actually computes the right
        // thing" smoke test.
        let real = vec![1.0f32, 0.0, 0.0, 0.0];
        let spectrum = fft(&real, false).unwrap();
        assert_eq!(spectrum.len(), 4);
        for k in 0..4 {
            let re = spectrum.as_interleaved()[2 * k];
            let im = spectrum.as_interleaved()[2 * k + 1];
            assert!(approx_eq(re, 1.0, 1e-5), "bin {} real = {}", k, re);
            assert!(approx_eq(im, 0.0, 1e-5), "bin {} imag = {}", k, im);
        }
    }

    #[test]
    fn public_round_trip_real_through_runtime_and_scalar_ifft() {
        // Forward FFT through the matrix-ir runtime path, inverse
        // through the scalar reference.  This is the only round
        // trip we can express today since `ifft` is still scalar
        // (the matrix-ir graph is real-input only).
        let real = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let spectrum = fft(&real, false).unwrap();
        let recovered = ifft(&spectrum).unwrap();
        let rec_real = recovered.real();
        assert_close(&real, &rec_real, 1e-4);
    }

    #[test]
    fn public_fft_complex_input_still_goes_through_scalar() {
        // The complex-input path is not yet supported by the
        // matrix-ir graph builder, so it must continue to match
        // the Phase 2 scalar oracle exactly (not 1e-4, exactly).
        let interleaved = vec![1.0f32, 2.0, 3.0, 4.0];
        let via_public = fft(&interleaved, true).unwrap();
        let via_scalar = fft_scalar(&interleaved).unwrap();
        assert_eq!(via_public.as_interleaved(), via_scalar.as_slice());
    }

    // ── Phase 4a: public API now accepts arbitrary N ≥ 1 via
    //    Bluestein on the non-power-of-two path.  These tests
    //    pin that behavior — what used to return
    //    `FftError::NotPowerOfTwo` now returns a correct spectrum.

    #[test]
    fn public_fft_real_non_pow2_routes_through_bluestein() {
        // N = 3: the smallest non-trivial non-pow2.  Previously
        // this returned NotPowerOfTwo(3); now Bluestein computes
        // the actual DFT.  Compare against the naive O(N²) DFT.
        let real = vec![1.0f32, 2.0, 3.0];
        let spectrum = fft(&real, false).unwrap();
        assert_eq!(spectrum.len(), 3);
        // Compare against naive_dft via the bluestein test
        // module's reference — we duplicate it here inline.
        let n = 3;
        let mut interleaved = vec![0.0f32; 2 * n];
        for (i, &x) in real.iter().enumerate() {
            interleaved[2 * i] = x;
        }
        for k in 0..n {
            let mut sr = 0.0f32;
            let mut si = 0.0f32;
            for nn in 0..n {
                let theta =
                    -2.0 * PI * (k as f32) * (nn as f32) / (n as f32);
                let (wi, wr) = theta.sin_cos();
                sr += wr * interleaved[2 * nn] - wi * interleaved[2 * nn + 1];
                si += wr * interleaved[2 * nn + 1] + wi * interleaved[2 * nn];
            }
            let got_re = spectrum.as_interleaved()[2 * k];
            let got_im = spectrum.as_interleaved()[2 * k + 1];
            assert!(
                approx_eq(got_re, sr, 1e-4),
                "bin {} re: got {}, expected {}",
                k,
                got_re,
                sr
            );
            assert!(
                approx_eq(got_im, si, 1e-4),
                "bin {} im: got {}, expected {}",
                k,
                got_im,
                si
            );
        }
    }

    #[test]
    fn public_ifft_round_trip_non_pow2_n5() {
        // Round-trip through the public API for N = 5 (non-pow2).
        // Forward goes through Bluestein (since 5 isn't a power
        // of two), inverse also goes through Bluestein.
        let real = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let spectrum = fft(&real, false).unwrap();
        assert_eq!(spectrum.len(), 5);
        let recovered = ifft(&spectrum).unwrap();
        let rec_real = recovered.real();
        assert_close(&real, &rec_real, 1e-4);
    }

    #[test]
    fn public_fft_complex_non_pow2_routes_through_bluestein() {
        // Complex-input, N = 3 (non-pow2).  Previously returned
        // NotPowerOfTwo(3) via fft_scalar; now goes through
        // Bluestein.  We verify by round-trip — the cleanest
        // check that doesn't require importing the naive DFT.
        let interleaved = vec![1.0f32, 0.5, 2.0, -0.5, 3.0, 0.25];
        let spectrum = fft(&interleaved, true).unwrap();
        assert_eq!(spectrum.len(), 3);
        let recovered = ifft(&spectrum).unwrap();
        assert_close(recovered.as_interleaved(), &interleaved, 1e-4);
    }
}
