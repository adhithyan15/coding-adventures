//! # Filter design helpers — windowed-sinc + Butterworth + Chebyshev
//!
//! **DSP03 Phase 5.**  Produces FIR kernels (for [`crate::fir`])
//! and IIR `(b, a)` coefficient pairs (for [`crate::iir`]) given
//! a target frequency response.  Phase 5 ships:
//!
//! - **Windowed-sinc FIR**: linear-phase low-pass, high-pass, and
//!   band-pass at any cutoff with Rectangular / Hamming / Hann / Blackman
//!   windows.
//! - **Butterworth IIR**: order 1 and order 2 low-pass via the
//!   bilinear transform of the analog prototype.
//! - **Chebyshev Type I IIR**: order 1 and order 2 low-pass with
//!   configurable passband ripple via the bilinear transform.
//!
//! ## Conventions
//!
//! - Frequencies are *normalised* to `[0.0, 0.5]` where `0.5` is
//!   the Nyquist frequency (half the sample rate).  E.g.
//!   `cutoff_norm = 0.1` at a 1 kHz sample rate means a 100 Hz
//!   cutoff.  This matches `scipy.signal.firwin` with its
//!   default `fs = 2`.
//! - `num_taps` for FIR designs must be **odd** so the resulting
//!   kernel has linear phase about its centre.  This also keeps
//!   the spectral inversion that turns LP into HP exact.
//! - All FIR kernels are normalised so the DC response is `1`
//!   (low-pass) or so the high-pass passes a pure constant
//!   through unchanged at Nyquist, or unit peak passband gain
//!   for band-pass.
//!
//! ## Validation
//!
//! V1 panics via `assert!` on invalid input (out-of-range cutoff,
//! even `num_taps`, unsupported filter order, non-finite values,
//! or taps exceeding defensive caps).  Callers should validate
//! parameters before calling.

use std::f32::consts::PI;

/// Choice of window function for the windowed-sinc FIR designs.
///
/// `Rectangular` gives the sharpest cutoff but worst sidelobes.
/// `Blackman` gives the smoothest sidelobes at the cost of
/// transition width.  `Hamming` / `Hann` sit in between and are
/// the most common choices.
///
/// Kaiser window (with adjustable `β` parameter) is deferred to
/// a future phase.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WindowType {
    /// `w[n] = 1.0`.  Simplest; produces large Gibbs-phenomenon ripples.
    Rectangular,
    /// `w[n] = 0.54 - 0.46 · cos(2πn / M)`.  Classic for audio
    /// EQ / speech processing — good sidelobe attenuation.
    Hamming,
    /// `w[n] = 0.5 · (1 - cos(2πn / M))`.  Like Hamming but
    /// slightly different sidelobe / main-lobe tradeoff.  Also
    /// known as the Hanning window.
    Hann,
    /// `w[n] = 0.42 - 0.5·cos(2πn/M) + 0.08·cos(4πn/M)`.  Best
    /// sidelobe attenuation of the four; widest main lobe.
    Blackman,
}

// ─────────────────────────── FIR design ───────────────────────────

/// Design a linear-phase low-pass FIR filter via the windowed-sinc
/// method.
///
/// - `cutoff_norm`: normalised cutoff frequency in `[0.0, 0.5]`
///   where `0.5` is Nyquist.
/// - `num_taps`: filter length — must be **odd** and `≥ 1` for
///   linear-phase symmetry.
/// - `window`: which window function to apply.
///
/// The returned kernel sums to `1.0` (DC gain = 1) and is
/// symmetric about its centre tap.  Plug it into [`crate::fir`]
/// to filter a signal.
///
/// # Panics
///
/// Panics if `num_taps` is even or zero, exceeds `MAX_TAPS`, or if
/// `cutoff_norm` is non-finite or outside `[0.0, 0.5]`.
pub fn design_low_pass(
    cutoff_norm: f32,
    num_taps: u32,
    window: WindowType,
) -> Vec<f32> {
    assert!(
        num_taps >= 1 && num_taps % 2 == 1,
        "num_taps must be odd and >= 1; got {}",
        num_taps
    );
    assert!(
        (num_taps as usize) <= crate::MAX_TAPS,
        "num_taps {} exceeds MAX_TAPS {}",
        num_taps,
        crate::MAX_TAPS
    );
    assert!(
        cutoff_norm.is_finite(),
        "cutoff_norm must be finite; got {}",
        cutoff_norm
    );
    assert!(
        (0.0..=0.5).contains(&cutoff_norm),
        "cutoff_norm must be in [0.0, 0.5]; got {}",
        cutoff_norm
    );

    let n = num_taps as usize;
    let m = (num_taps - 1) as f32; // peak symmetry index
    let centre = m / 2.0;

    let fc = cutoff_norm;
    let mut h = vec![0.0f32; n];
    for (k, h_k) in h.iter_mut().enumerate() {
        let x = (k as f32) - centre;
        *h_k = if x == 0.0 {
            2.0 * fc
        } else {
            (2.0 * PI * fc * x).sin() / (PI * x)
        };
    }

    for (k, h_k) in h.iter_mut().enumerate() {
        let n_f = k as f32;
        let w = if m == 0.0 {
            1.0
        } else {
            match window {
                WindowType::Rectangular => 1.0,
                WindowType::Hamming => 0.54 - 0.46 * (2.0 * PI * n_f / m).cos(),
                WindowType::Hann => 0.5 * (1.0 - (2.0 * PI * n_f / m).cos()),
                WindowType::Blackman => {
                    0.42 - 0.5 * (2.0 * PI * n_f / m).cos()
                        + 0.08 * (4.0 * PI * n_f / m).cos()
                }
            }
        };
        *h_k *= w;
    }

    let sum: f32 = h.iter().sum();
    if sum.abs() > 0.0 {
        for v in &mut h {
            *v /= sum;
        }
    }
    h
}

/// Design a linear-phase high-pass FIR filter via spectral
/// inversion of a windowed-sinc low-pass.
pub fn design_high_pass(
    cutoff_norm: f32,
    num_taps: u32,
    window: WindowType,
) -> Vec<f32> {
    let lp = design_low_pass(cutoff_norm, num_taps, window);
    let centre = (num_taps as usize - 1) / 2;
    let mut hp = lp;
    for v in &mut hp {
        *v = -*v;
    }
    hp[centre] += 1.0;
    hp
}

/// Design a linear-phase band-pass FIR filter via the windowed-sinc
/// method.
///
/// - `low_cutoff_norm`: lower cutoff frequency in `[0.0, 0.5]`
/// - `high_cutoff_norm`: upper cutoff frequency in `[0.0, 0.5]`
///   with `low_cutoff_norm < high_cutoff_norm`.
/// - `num_taps`: filter length — must be **odd** and `≥ 1`.
/// - `window`: which window function to apply.
///
/// The returned kernel is normalized for unit peak passband gain
/// at the center frequency and is symmetric about its centre tap.
///
/// # Panics
///
/// Panics if `num_taps` is even, zero, or exceeds `MAX_TAPS`, or if
/// cutoffs are not finite or not in `0.0 <= low < high <= 0.5`.
pub fn design_band_pass(
    low_cutoff_norm: f32,
    high_cutoff_norm: f32,
    num_taps: u32,
    window: WindowType,
) -> Vec<f32> {
    assert!(
        num_taps >= 1 && num_taps % 2 == 1,
        "num_taps must be odd and >= 1; got {}",
        num_taps
    );
    assert!(
        (num_taps as usize) <= crate::MAX_TAPS,
        "num_taps {} exceeds MAX_TAPS {}",
        num_taps,
        crate::MAX_TAPS
    );
    assert!(
        low_cutoff_norm.is_finite() && high_cutoff_norm.is_finite(),
        "cutoffs must be finite"
    );
    assert!(
        low_cutoff_norm >= 0.0 && low_cutoff_norm < high_cutoff_norm && high_cutoff_norm <= 0.5,
        "cutoffs must satisfy 0.0 <= low < high <= 0.5; got low={}, high={}",
        low_cutoff_norm,
        high_cutoff_norm
    );

    let n = num_taps as usize;
    let m = (num_taps - 1) as f32;
    let centre = m / 2.0;

    let f1 = low_cutoff_norm;
    let f2 = high_cutoff_norm;
    let mut h = vec![0.0f32; n];
    for (k, h_k) in h.iter_mut().enumerate() {
        let x = (k as f32) - centre;
        *h_k = if x == 0.0 {
            2.0 * (f2 - f1)
        } else {
            ((2.0 * PI * f2 * x).sin() - (2.0 * PI * f1 * x).sin()) / (PI * x)
        };
    }

    for (k, h_k) in h.iter_mut().enumerate() {
        let n_f = k as f32;
        let w = if m == 0.0 {
            1.0
        } else {
            match window {
                WindowType::Rectangular => 1.0,
                WindowType::Hamming => 0.54 - 0.46 * (2.0 * PI * n_f / m).cos(),
                WindowType::Hann => 0.5 * (1.0 - (2.0 * PI * n_f / m).cos()),
                WindowType::Blackman => {
                    0.42 - 0.5 * (2.0 * PI * n_f / m).cos()
                        + 0.08 * (4.0 * PI * n_f / m).cos()
                }
            }
        };
        *h_k *= w;
    }

    // Normalise gain at center frequency f0 = (f1 + f2) / 2
    let f0 = (f1 + f2) * 0.5;
    let mut gain = 0.0f32;
    for (k, &h_k) in h.iter().enumerate() {
        let x = (k as f32) - centre;
        gain += h_k * (2.0 * PI * f0 * x).cos();
    }
    let gain_abs = gain.abs();
    if gain_abs > 0.0 {
        for v in &mut h {
            *v /= gain_abs;
        }
    }
    h
}

// ─────────────────────────── IIR design ───────────────────────────

/// Design a Butterworth low-pass IIR filter.  Returns `(b, a)`
/// coefficient vectors ready to pass to [`crate::iir`].
///
/// `order` is the filter order (number of poles). Supports orders `1` and `2`.
/// `cutoff_norm` is in `(0.0, 0.5)` where `0.5` is Nyquist.
///
/// # Panics
///
/// Panics if `order` is `0` or `> 2`, or if `cutoff_norm` is
/// outside `(0.0, 0.5)` or not finite.
pub fn butterworth_lowpass(order: u32, cutoff_norm: f32) -> (Vec<f32>, Vec<f32>) {
    assert!(
        order == 1 || order == 2,
        "Butterworth supports orders 1 and 2; got {}",
        order
    );
    assert!(
        cutoff_norm.is_finite() && cutoff_norm > 0.0 && cutoff_norm < 0.5,
        "cutoff_norm must be in (0.0, 0.5); got {}",
        cutoff_norm
    );

    let k = (PI * cutoff_norm).tan();

    match order {
        1 => {
            let alpha = k / (1.0 + k);
            (vec![alpha, alpha], vec![1.0, 2.0 * alpha - 1.0])
        }
        2 => {
            let omega0 = 2.0 * PI * cutoff_norm;
            let (sin_w0, cos_w0) = omega0.sin_cos();
            let q = 1.0 / 2.0_f32.sqrt();
            let alpha = sin_w0 / (2.0 * q);
            let b0 = (1.0 - cos_w0) / 2.0;
            let b1 = 1.0 - cos_w0;
            let b2 = (1.0 - cos_w0) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            (
                vec![b0 / a0, b1 / a0, b2 / a0],
                vec![1.0, a1 / a0, a2 / a0],
            )
        }
        _ => unreachable!("order validated above"),
    }
}

/// Design a Chebyshev Type I low-pass IIR filter.  Returns `(b, a)`
/// coefficient vectors ready to pass to [`crate::iir`].
///
/// `order` is the filter order (number of poles). Supports orders `1` and `2`.
/// `ripple_db` is the passband ripple in decibels (must be `> 0`).
/// `cutoff_norm` is in `(0.0, 0.5)` where `0.5` is Nyquist.
///
/// Uses the bilinear transform with pre-warping.
///
/// # Panics
///
/// Panics if `order` is not 1 or 2, `ripple_db <= 0.0` or not finite,
/// or `cutoff_norm` is outside `(0.0, 0.5)` or not finite.
pub fn chebyshev1_lowpass(
    order: u32,
    ripple_db: f32,
    cutoff_norm: f32,
) -> (Vec<f32>, Vec<f32>) {
    assert!(
        order == 1 || order == 2,
        "Chebyshev-I supports orders 1 and 2; got {}",
        order
    );
    assert!(
        ripple_db.is_finite() && ripple_db > 0.0,
        "ripple_db must be finite and > 0; got {}",
        ripple_db
    );
    assert!(
        cutoff_norm.is_finite() && cutoff_norm > 0.0 && cutoff_norm < 0.5,
        "cutoff_norm must be in (0.0, 0.5); got {}",
        cutoff_norm
    );

    let k = (PI * cutoff_norm).tan();
    let eps = (10.0_f32.powf(ripple_db * 0.1) - 1.0).sqrt();
    let mu = (1.0 / order as f32) * ((1.0 / eps) + ((1.0 / (eps * eps)) + 1.0).sqrt()).ln();

    match order {
        1 => {
            let sinh_mu = mu.sinh();
            let alpha = k * sinh_mu / (1.0 + k * sinh_mu);
            (vec![alpha, alpha], vec![1.0, 2.0 * alpha - 1.0])
        }
        2 => {
            let u = mu.sinh() / 2.0_f32.sqrt();
            let v = mu.cosh() / 2.0_f32.sqrt();
            let sum_sq = u * u + v * v;
            let na = sum_sq / (1.0 + eps * eps).sqrt();

            let a0 = 1.0 + 2.0 * u * k + sum_sq * k * k;
            let a1 = 2.0 * (sum_sq * k * k - 1.0);
            let a2 = 1.0 - 2.0 * u * k + sum_sq * k * k;

            let b0 = na * k * k;
            let b1 = 2.0 * b0;
            let b2 = b0;

            (
                vec![b0 / a0, b1 / a0, b2 / a0],
                vec![1.0, a1 / a0, a2 / a0],
            )
        }
        _ => unreachable!("order validated above"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fir, iir};

    fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
        let scale = a.abs().max(b.abs()).max(1.0);
        (a - b).abs() <= scale * tol
    }

    // ── windowed-sinc tests ────────────────────────────────────

    #[test]
    fn low_pass_sums_to_one() {
        for &cutoff in &[0.05, 0.1, 0.25, 0.4] {
            for &win in &[
                WindowType::Rectangular,
                WindowType::Hamming,
                WindowType::Hann,
                WindowType::Blackman,
            ] {
                let h = design_low_pass(cutoff, 33, win);
                let sum: f32 = h.iter().sum();
                assert!(
                    approx_eq(sum, 1.0, 1e-5),
                    "low-pass kernel sum = {} (expected 1.0)",
                    sum
                );
            }
        }
    }

    #[test]
    fn low_pass_kernel_is_symmetric() {
        let h = design_low_pass(0.2, 33, WindowType::Hamming);
        let n = h.len();
        for i in 0..(n / 2) {
            assert!(
                approx_eq(h[i], h[n - 1 - i], 1e-6),
                "kernel not symmetric at {}: {} vs {}",
                i,
                h[i],
                h[n - 1 - i]
            );
        }
    }

    #[test]
    fn low_pass_passes_dc() {
        let h = design_low_pass(0.2, 33, WindowType::Hamming);
        let dc_signal = vec![2.5f32; 64];
        let out = fir(&dc_signal, &h).unwrap();
        for &v in &out[32..48] {
            assert!(
                approx_eq(v, 2.5, 1e-3),
                "low-pass steady-state DC = {}, expected 2.5",
                v
            );
        }
    }

    #[test]
    fn low_pass_attenuates_high_frequency() {
        let h = design_low_pass(0.1, 65, WindowType::Blackman);
        let n = 256;
        let hi_signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * 0.45 * (i as f32)).cos())
            .collect();
        let out = fir(&hi_signal, &h).unwrap();
        let steady = &out[64..(n - 32)];
        let max_amp = steady.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        assert!(
            max_amp < 0.05,
            "low-pass high-frequency leak = {}, expected < 0.05",
            max_amp
        );
    }

    #[test]
    fn single_tap_filter_designs_are_finite() {
        for window in [
            WindowType::Rectangular,
            WindowType::Hamming,
            WindowType::Hann,
            WindowType::Blackman,
        ] {
            let lp = design_low_pass(0.2, 1, window);
            assert_eq!(lp.len(), 1);
            assert!(lp[0].is_finite() && !lp[0].is_nan());

            let hp = design_high_pass(0.2, 1, window);
            assert_eq!(hp.len(), 1);
            assert!(hp[0].is_finite() && !hp[0].is_nan());

            let bp = design_band_pass(0.2, 0.4, 1, window);
            assert_eq!(bp.len(), 1);
            assert!(bp[0].is_finite() && !bp[0].is_nan());
        }
    }

    #[test]
    fn high_pass_sums_to_zero() {
        for &cutoff in &[0.05, 0.1, 0.25, 0.4] {
            let h = design_high_pass(cutoff, 33, WindowType::Hamming);
            let sum: f32 = h.iter().sum();
            assert!(
                sum.abs() < 1e-5,
                "high-pass kernel sum = {} (expected ~0)",
                sum
            );
        }
    }

    #[test]
    fn band_pass_attenuates_dc_and_high_freq() {
        let bp = design_band_pass(0.15, 0.35, 65, WindowType::Hamming);
        let dc_gain: f32 = bp.iter().sum();
        assert!(
            dc_gain.abs() < 0.05,
            "band-pass DC gain = {} (expected ~0)",
            dc_gain
        );

        // Sinusoid at center frequency 0.25 should pass with ~1.0 gain
        let n = 512;
        let mid_signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * 0.25 * (i as f32)).cos())
            .collect();
        let out = fir(&mid_signal, &bp).unwrap();
        let steady = &out[100..(n - 100)];
        let max_amp = steady.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        assert!(
            (max_amp - 1.0).abs() < 0.15,
            "band-pass center amp = {}, expected ~1.0",
            max_amp
        );
    }

    // ── Butterworth tests ──────────────────────────────────────

    #[test]
    fn butterworth_order_1_dc_gain_is_one() {
        for &cutoff in &[0.05, 0.1, 0.25, 0.4] {
            let (b, a) = butterworth_lowpass(1, cutoff);
            let sum_b: f32 = b.iter().sum();
            let sum_a: f32 = a.iter().sum();
            let dc = sum_b / sum_a;
            assert!(
                approx_eq(dc, 1.0, 1e-5),
                "1st-order Butterworth DC gain = {} (cutoff={})",
                dc,
                cutoff
            );
        }
    }

    #[test]
    fn butterworth_order_2_dc_gain_is_one() {
        for &cutoff in &[0.05, 0.1, 0.25, 0.4] {
            let (b, a) = butterworth_lowpass(2, cutoff);
            let sum_b: f32 = b.iter().sum();
            let sum_a: f32 = a.iter().sum();
            let dc = sum_b / sum_a;
            assert!(
                approx_eq(dc, 1.0, 1e-4),
                "2nd-order Butterworth DC gain = {} (cutoff={})",
                dc,
                cutoff
            );
        }
    }

    #[test]
    fn butterworth_order_1_step_response_asymptotes_to_one() {
        let (b, a) = butterworth_lowpass(1, 0.1);
        let step = vec![1.0f32; 200];
        let out = iir(&step, &b, &a).unwrap();
        assert!(
            approx_eq(out[199], 1.0, 1e-3),
            "1st-order step output = {}, expected ~1.0",
            out[199]
        );
    }

    #[test]
    fn butterworth_order_2_step_response_asymptotes_to_one() {
        let (b, a) = butterworth_lowpass(2, 0.1);
        let step = vec![1.0f32; 200];
        let out = iir(&step, &b, &a).unwrap();
        assert!(
            approx_eq(out[199], 1.0, 1e-3),
            "2nd-order step output = {}, expected ~1.0",
            out[199]
        );
    }

    // ── Chebyshev-I tests ──────────────────────────────────────

    #[test]
    fn chebyshev1_order_1_dc_gain_is_one() {
        let (b, a) = chebyshev1_lowpass(1, 1.0, 0.2);
        let sum_b: f32 = b.iter().sum();
        let sum_a: f32 = a.iter().sum();
        let dc = sum_b / sum_a;
        assert!(approx_eq(dc, 1.0, 1e-4), "Chebyshev-I order 1 DC gain = {}", dc);
    }

    #[test]
    fn chebyshev1_order_2_dc_gain_matches_ripple() {
        let ripple_db = 1.0f32;
        let (b, a) = chebyshev1_lowpass(2, ripple_db, 0.2);
        let sum_b: f32 = b.iter().sum();
        let sum_a: f32 = a.iter().sum();
        let dc = sum_b / sum_a;
        let expected_dc = 10.0_f32.powf(-ripple_db / 20.0);
        assert!(
            approx_eq(dc, expected_dc, 1e-3),
            "Chebyshev-I order 2 DC gain = {}, expected {}",
            dc,
            expected_dc
        );
    }
}
