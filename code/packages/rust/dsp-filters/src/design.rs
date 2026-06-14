//! # Filter design helpers — windowed-sinc + Butterworth
//!
//! **DSP03 Phase 5.**  Produces FIR kernels (for [`crate::fir`])
//! and IIR `(b, a)` coefficient pairs (for [`crate::iir`]) given
//! a target frequency response.  Phase 5 ships:
//!
//! - **Windowed-sinc FIR**: linear-phase low-pass / high-pass at
//!   any cutoff with Rectangular / Hamming / Hann / Blackman
//!   windows.
//! - **Butterworth IIR**: order 1 and order 2 low-pass via the
//!   bilinear transform of the analog prototype.
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
//!   through unchanged at Nyquist.
//!
//! ## Validation
//!
//! V1 panics via `assert!` on invalid input (out-of-range cutoff,
//! even `num_taps`, unsupported Butterworth order).  Callers
//! should validate parameters before calling — the panics
//! exist to catch programmer errors, not user input.

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
    /// `w[n] = 1.0`.  Simplest; produces large Gibbs-phenomenon
    /// ripples.
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
/// to filter a signal:
///
/// ```rust
/// use dsp_filters::{fir, design::{design_low_pass, WindowType}};
///
/// let kernel = design_low_pass(0.2, 33, WindowType::Hamming);
/// let signal: Vec<f32> = (0..100).map(|i| (i as f32) * 0.01).collect();
/// let smoothed = fir(&signal, &kernel).unwrap();
/// ```
///
/// # Panics
///
/// Panics if `num_taps` is even or zero, or if `cutoff_norm` is
/// outside `[0.0, 0.5]`.
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
        (0.0..=0.5).contains(&cutoff_norm),
        "cutoff_norm must be in [0.0, 0.5]; got {}",
        cutoff_norm
    );

    let n = num_taps as usize;
    let m = (num_taps - 1) as f32; // peak symmetry index
    let centre = m / 2.0;

    // ── Step 1: ideal sinc impulse response.
    //
    //   h_ideal[k] = 2·fc · sinc(2·fc·(k - centre))
    //   where sinc(x) = sin(π·x) / (π·x), with sinc(0) = 1.
    //
    //   The 2·fc factor sets the DC gain to 1; the limit at
    //   the centre tap is exactly 2·fc.
    let fc = cutoff_norm;
    let mut h = vec![0.0f32; n];
    for k in 0..n {
        let x = (k as f32) - centre;
        h[k] = if x == 0.0 {
            2.0 * fc
        } else {
            (2.0 * PI * fc * x).sin() / (PI * x)
        };
    }

    // ── Step 2: apply window.
    for k in 0..n {
        let n_f = k as f32;
        let w = match window {
            WindowType::Rectangular => 1.0,
            WindowType::Hamming => 0.54 - 0.46 * (2.0 * PI * n_f / m).cos(),
            WindowType::Hann => 0.5 * (1.0 - (2.0 * PI * n_f / m).cos()),
            WindowType::Blackman => {
                0.42 - 0.5 * (2.0 * PI * n_f / m).cos()
                    + 0.08 * (4.0 * PI * n_f / m).cos()
            }
        };
        h[k] *= w;
    }

    // ── Step 3: normalise so the windowed kernel sums to 1 (DC
    //   gain = 1).  The window perturbs the integral slightly;
    //   this re-scales to compensate.
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
///
/// Algorithm:
///
/// 1. Design the corresponding low-pass kernel `h_lp`.
/// 2. Negate every tap.
/// 3. Add `1.0` to the centre tap (the spectral inversion of an
///    ideal low-pass with cutoff `fc` is an ideal high-pass with
///    the same cutoff, plus a delta impulse).
///
/// Requires odd `num_taps` so the centre tap is at an exact
/// integer index.
///
/// # Panics
///
/// Same conditions as [`design_low_pass`].
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

// ─────────────────────────── IIR design ───────────────────────────

/// Design a Butterworth low-pass IIR filter.  Returns `(b, a)`
/// coefficient vectors ready to pass to [`crate::iir`].
///
/// `order` is the filter order (number of poles).  V1 supports
/// orders `1` and `2`.  Higher orders typically factor into
/// cascaded biquads for numerical stability — that's a future
/// phase.
///
/// `cutoff_norm` is in `[0.0, 0.5]` where `0.5` is Nyquist.
///
/// The bilinear transform is used with **pre-warping** so the
/// digital cutoff matches the requested frequency exactly:
///
/// ```text
///     ω_c = tan(π · cutoff_norm)
/// ```
///
/// DC gain is exactly `1.0` for both orders.
///
/// # Panics
///
/// Panics if `order` is `0` or `> 2`, or if `cutoff_norm` is
/// outside `(0.0, 0.5)`.  (Cutoff of exactly `0` or `0.5` would
/// produce degenerate filters — `tan(π · 0.5)` is infinite.)
pub fn butterworth_lowpass(order: u32, cutoff_norm: f32) -> (Vec<f32>, Vec<f32>) {
    assert!(
        order == 1 || order == 2,
        "Butterworth V1 supports orders 1 and 2; got {}",
        order
    );
    assert!(
        cutoff_norm > 0.0 && cutoff_norm < 0.5,
        "cutoff_norm must be in (0.0, 0.5); got {}",
        cutoff_norm
    );

    let k = (PI * cutoff_norm).tan(); // pre-warped digital cutoff

    match order {
        1 => {
            // 1st-order Butterworth LP: H_s(s) = 1/(s/k + 1).
            // Bilinear transform s = (1-z⁻¹)/(1+z⁻¹) gives:
            //   H_z(z) = α·(1+z⁻¹) / (1 + (2α-1)·z⁻¹)
            // where α = k/(1+k).  DC gain = 2α/(2α) = 1.
            let alpha = k / (1.0 + k);
            (vec![alpha, alpha], vec![1.0, 2.0 * alpha - 1.0])
        }
        2 => {
            // 2nd-order Butterworth LP via RBJ-style biquad
            // (Q = 1/√2 for Butterworth response):
            //
            //   ω0 = 2π · cutoff_norm  (digital angular freq)
            //
            //   b0 = (1 - cos ω0) / 2
            //   b1 =  1 - cos ω0
            //   b2 = (1 - cos ω0) / 2
            //   a0 =  1 + α
            //   a1 = -2 cos ω0
            //   a2 =  1 - α
            //
            //   α = sin(ω0) / (2·Q) = sin(ω0) · √2 / 2 = sin(ω0)/√2.
            //
            // After dividing by a0 to normalise:
            //   DC gain = (b0 + b1 + b2) / (a0 + a1 + a2)
            //           = 2(1 - cos ω0) / (2(1 - cos ω0)) = 1.
            //
            // (Bilinear pre-warping is baked into the RBJ formula
            // — no extra `k` needed when expressed via ω0
            // directly.)
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
        // The DC-gain-1 normalisation ensures the kernel sums to
        // 1 regardless of cutoff or window choice.
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
                    "LP sum = {}, expected 1.0 (cutoff={}, window={:?})",
                    sum,
                    cutoff,
                    win
                );
            }
        }
    }

    #[test]
    fn low_pass_kernel_is_symmetric() {
        // Linear-phase property: h[k] == h[M - k] for all k.
        let h = design_low_pass(0.2, 21, WindowType::Hamming);
        let m = h.len() - 1;
        for k in 0..=m / 2 {
            assert!(
                approx_eq(h[k], h[m - k], 1e-6),
                "asymmetry at k={}: h[{}]={} vs h[{}]={}",
                k,
                k,
                h[k],
                m - k,
                h[m - k]
            );
        }
    }

    #[test]
    fn high_pass_sums_to_zero() {
        // High-pass should reject DC entirely — kernel sums to 0.
        let h = design_high_pass(0.2, 33, WindowType::Hamming);
        let sum: f32 = h.iter().sum();
        assert!(
            approx_eq(sum, 0.0, 1e-5),
            "HP kernel sum = {}, expected ~0",
            sum
        );
    }

    #[test]
    fn high_pass_plus_low_pass_is_centred_impulse() {
        // The spectral-inversion construction guarantees
        // LP[k] + HP[k] = δ[k - centre] exactly.
        let cutoff = 0.2;
        let num_taps = 21;
        let lp = design_low_pass(cutoff, num_taps, WindowType::Hamming);
        let hp = design_high_pass(cutoff, num_taps, WindowType::Hamming);
        let centre = (num_taps as usize - 1) / 2;
        for k in 0..lp.len() {
            let sum = lp[k] + hp[k];
            let expected = if k == centre { 1.0 } else { 0.0 };
            assert!(
                approx_eq(sum, expected, 1e-6),
                "LP+HP at k={}: {} (expected {})",
                k,
                sum,
                expected
            );
        }
    }

    #[test]
    fn low_pass_attenuates_high_frequency() {
        // Apply a low-pass to a high-frequency sinusoid; output
        // amplitude should be much smaller than input.
        let cutoff = 0.1;
        let h = design_low_pass(cutoff, 51, WindowType::Blackman);
        // High frequency: 0.4 * Nyquist (way above cutoff).
        let n = 256;
        let signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * 0.4 * (i as f32)).cos())
            .collect();
        let out = fir(&signal, &h).unwrap();
        // Check steady-state region (skip transient at start/end).
        let steady = &out[80..(n - 80)];
        let max_amplitude = steady.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        assert!(
            max_amplitude < 0.1,
            "high-freq sinusoid amplitude after LP = {}, expected « 1.0",
            max_amplitude
        );
    }

    #[test]
    fn low_pass_passes_dc() {
        // Constant signal through low-pass: steady-state output
        // should equal the input amplitude (DC gain = 1).
        let h = design_low_pass(0.2, 31, WindowType::Hamming);
        let signal = vec![1.0f32; 200];
        let out = fir(&signal, &h).unwrap();
        // Past the transient at the start, output should be 1.0.
        let final_value = out[150];
        assert!(
            approx_eq(final_value, 1.0, 1e-4),
            "LP DC output = {}, expected ~1.0",
            final_value
        );
    }

    // ── Butterworth tests ──────────────────────────────────────

    #[test]
    fn butterworth_order_1_dc_gain_is_one() {
        for &cutoff in &[0.05, 0.1, 0.25, 0.4] {
            let (b, a) = butterworth_lowpass(1, cutoff);
            // DC gain = H(z=1) = Σb / Σa.
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
        // Pass a unit step through a 1st-order Butterworth LP;
        // the output should converge to 1.0 (DC gain).
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

    #[test]
    fn butterworth_order_1_attenuates_high_freq() {
        // Apply 1st-order LP at cutoff = 0.1 to a high-freq
        // sinusoid (0.45 · Nyquist).  Output should be attenuated.
        let (b, a) = butterworth_lowpass(1, 0.1);
        let n = 512;
        let signal: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * 0.45 * (i as f32)).cos())
            .collect();
        let out = iir(&signal, &b, &a).unwrap();
        // Steady-state region.
        let steady = &out[200..(n - 50)];
        let max_amp = steady.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        assert!(
            max_amp < 0.3,
            "1st-order LP high-freq amp = {}, expected < 0.3",
            max_amp
        );
    }
}
