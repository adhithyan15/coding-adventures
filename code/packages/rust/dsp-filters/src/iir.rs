//! # IIR (Infinite Impulse Response) filter — Direct-Form-II Transposed
//!
//! **DSP03 Phase 4.**  Recursive filtering via the canonical
//! direct-form-II Transposed structure (the form `scipy.signal.lfilter`
//! and MATLAB's `filter` use).  Operates on `f32` signals with
//! `b` (numerator / feed-forward) and `a` (denominator / feedback)
//! coefficient vectors.
//!
//! ## Mathematical model
//!
//! An IIR filter realises a rational transfer function:
//!
//! ```text
//!     H(z) =  b[0] + b[1]z⁻¹ + b[2]z⁻² + … + b[M]z⁻ᴹ
//!            ─────────────────────────────────────────
//!             a[0] + a[1]z⁻¹ + a[2]z⁻² + … + a[N]z⁻ᴺ
//! ```
//!
//! In the time domain:
//!
//! ```text
//!     a[0] · y[n] =   b[0] · x[n] + b[1] · x[n-1] + … + b[M] · x[n-M]
//!                   - a[1] · y[n-1] - a[2] · y[n-2] - … - a[N] · y[n-N]
//! ```
//!
//! The recursion makes it "infinite": each output depends on past
//! outputs going back as far as `N`, even after the input stops.
//! That's how IIR filters squeeze a sharper magnitude response
//! into far fewer coefficients than an equivalent FIR — and also
//! how they can blow up if the poles drift outside the unit circle.
//!
//! ## Direct-Form-II Transposed
//!
//! The transposed canonical form keeps a state vector `z` of
//! length `order = max(M, N)` and processes one sample at a time:
//!
//! ```text
//!     y[n] = (b[0] · x[n] + z[0]) / a[0]
//!     for k in 0..order - 1:
//!         z[k] = b[k+1] · x[n] - a[k+1] · y[n] + z[k+1]
//!     z[order - 1] = b[order] · x[n] - a[order] · y[n]
//! ```
//!
//! Where both `b` and `a` are conceptually zero-padded to length
//! `order + 1` (so accesses like `b[k+1]` past the actual length
//! pull in implicit zeros).  The state is initialised to zero
//! (assuming the signal starts at rest).
//!
//! Why this form?  Two reasons:
//!
//! 1. **Numerical stability.**  The transposed form keeps the
//!    accumulator running through fewer multiplications per
//!    state slot than the non-transposed form, so noise grows
//!    more slowly at higher orders.
//! 2. **One pass.**  No need to store past `x` and `y`
//!    separately; the state encodes both, and we read it in
//!    one place and write it back in another within the same
//!    inner loop.
//!
//! ## Stability
//!
//! V1 does not validate pole locations.  If the caller passes
//! `a` coefficients whose roots are outside the unit circle,
//! the output diverges.  Phase 5's design helpers will produce
//! stable coefficients by construction.

use crate::FilterError;

/// Infinite-impulse-response filter via direct-form-II Transposed.
///
/// `signal` is a length-`N` real signal.  `b` is the feed-forward
/// (numerator) polynomial; `a` is the feedback (denominator)
/// polynomial.  The result is divided by `a[0]` (so passing
/// `a[0] != 1.0` is fine, scipy-style).
///
/// Matches `scipy.signal.lfilter(b, a, x)` exactly.  Output length
/// equals `signal.len()`.
///
/// # Errors
///
/// - `EmptyKernel` — `a` or `b` is empty.
/// - `InvalidCoefficient` — `a[0]` is zero, NaN, or infinite
///   (can't safely divide).
pub fn iir(
    signal: &[f32],
    b: &[f32],
    a: &[f32],
) -> Result<Vec<f32>, FilterError> {
    if signal.is_empty() {
        return Err(FilterError::EmptySignal);
    }
    if b.is_empty() || a.is_empty() {
        return Err(FilterError::EmptyKernel);
    }
    let a0 = a[0];
    if a0 == 0.0 {
        return Err(FilterError::InvalidCoefficient(
            "a[0] must be non-zero".into(),
        ));
    }
    if !a0.is_finite() {
        return Err(FilterError::InvalidCoefficient(format!(
            "a[0] must be finite; got {}",
            a0
        )));
    }

    // ── Effective order.  Both `b` and `a` are conceptually
    //   padded to length `order + 1` with zeros.
    //
    //   For the FIR special case (`a = [a[0]]`), order = M (the
    //   number of taps minus one).  For a 2nd-order biquad
    //   (`b.len() == 3, a.len() == 3`), order = 2 with two
    //   state variables.
    let m = b.len();
    let n_coef = a.len();
    let order = m.max(n_coef).saturating_sub(1);

    // ── Pre-scale `b` and `a` by `1 / a[0]` so the inner loop
    //   doesn't divide every sample.  This also normalises
    //   convention-mismatches where a caller might pass
    //   `a[0] != 1`.
    let inv_a0 = 1.0 / a0;
    let b_scaled: Vec<f32> = (0..=order)
        .map(|k| if k < m { b[k] * inv_a0 } else { 0.0 })
        .collect();
    let a_scaled: Vec<f32> = (0..=order)
        .map(|k| if k < n_coef { a[k] * inv_a0 } else { 0.0 })
        .collect();
    // After scaling, a_scaled[0] == 1.0 exactly (or near it
    // modulo FP precision).  We don't need it in the inner loop.

    // ── Process samples.  `state` has length `order`; for
    //   `order == 0` (pure gain filter, `b = [g], a = [1]`),
    //   the state vector is empty and we skip the update loop.
    let mut state = vec![0.0f32; order];
    let mut out = Vec::with_capacity(signal.len());

    for &x in signal {
        // y[n] = b_scaled[0] · x[n] + z[0]
        //   (the divide by a[0] is already folded into the
        //   scaled coefficients, so this is the correctly-
        //   normalised output.)
        let y = b_scaled[0] * x + if order > 0 { state[0] } else { 0.0 };
        out.push(y);

        // State update.  For each slot except the last:
        //   z[k] = b_scaled[k+1] · x - a_scaled[k+1] · y + z[k+1]
        // For the last slot:
        //   z[order-1] = b_scaled[order] · x - a_scaled[order] · y
        if order > 0 {
            for k in 0..(order - 1) {
                state[k] = b_scaled[k + 1] * x
                    - a_scaled[k + 1] * y
                    + state[k + 1];
            }
            state[order - 1] = b_scaled[order] * x - a_scaled[order] * y;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fir;

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
    fn iir_rejects_empty_signal() {
        let err = iir(&[], &[1.0], &[1.0]).unwrap_err();
        assert_eq!(err, FilterError::EmptySignal);
    }

    #[test]
    fn iir_rejects_empty_b() {
        let err = iir(&[1.0, 2.0], &[], &[1.0]).unwrap_err();
        assert_eq!(err, FilterError::EmptyKernel);
    }

    #[test]
    fn iir_rejects_empty_a() {
        let err = iir(&[1.0, 2.0], &[1.0], &[]).unwrap_err();
        assert_eq!(err, FilterError::EmptyKernel);
    }

    #[test]
    fn iir_rejects_zero_a0() {
        let err = iir(&[1.0, 2.0], &[1.0], &[0.0, 0.5]).unwrap_err();
        assert!(matches!(err, FilterError::InvalidCoefficient(_)));
    }

    #[test]
    fn iir_rejects_nan_a0() {
        let err = iir(&[1.0, 2.0], &[1.0], &[f32::NAN, 0.5]).unwrap_err();
        assert!(matches!(err, FilterError::InvalidCoefficient(_)));
    }

    #[test]
    fn iir_rejects_inf_a0() {
        let err = iir(&[1.0, 2.0], &[1.0], &[f32::INFINITY]).unwrap_err();
        assert!(matches!(err, FilterError::InvalidCoefficient(_)));
    }

    // ── closed-form known vectors ──────────────────────────────

    #[test]
    fn iir_identity_filter_passes_through() {
        // y[n] = x[n] (no state, no feedback).
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let out = iir(&signal, &[1.0], &[1.0]).unwrap();
        assert_close(&out, &signal, 1e-7);
    }

    #[test]
    fn iir_pure_gain_scales_signal() {
        // y[n] = 2 · x[n].  a[0] != 1 to exercise the
        // normalisation path.
        let signal = vec![1.0f32, 2.0, 3.0];
        let out = iir(&signal, &[2.0], &[1.0]).unwrap();
        assert_close(&out, &[2.0, 4.0, 6.0], 1e-7);
    }

    #[test]
    fn iir_pure_gain_via_a0_normalisation() {
        // y[n] = (1.0 / 2.0) · x[n] — `a[0] = 2.0` is rescaled
        // internally to make the divide-by-a[0] a no-op for the
        // inner loop.
        let signal = vec![2.0f32, 4.0, 6.0];
        let out = iir(&signal, &[1.0], &[2.0]).unwrap();
        assert_close(&out, &[1.0, 2.0, 3.0], 1e-7);
    }

    #[test]
    fn iir_single_pole_low_pass_step_response_asymptotes() {
        // b = [1.0], a = [1.0, -0.9] is the single-pole low-pass
        // y[n] = x[n] + 0.9 · y[n-1].  Step input asymptotes to
        // 1 / (1 - 0.9) = 10.0.
        let n = 200;
        let step: Vec<f32> = vec![1.0f32; n];
        let out = iir(&step, &[1.0], &[1.0, -0.9]).unwrap();
        let final_value = out[n - 1];
        assert!(
            approx_eq(final_value, 10.0, 1e-3),
            "single-pole step asymptote = {}, expected ~10.0",
            final_value
        );
    }

    #[test]
    fn iir_single_pole_low_pass_first_few_samples() {
        // Same filter, but verify the early trajectory matches the
        // closed-form geometric series:
        //   y[0] = 1
        //   y[1] = 1 + 0.9 · 1 = 1.9
        //   y[2] = 1 + 0.9 · 1.9 = 2.71
        //   y[3] = 1 + 0.9 · 2.71 = 3.439
        let step = vec![1.0f32; 4];
        let out = iir(&step, &[1.0], &[1.0, -0.9]).unwrap();
        assert_close(&out, &[1.0, 1.9, 2.71, 3.439], 1e-5);
    }

    #[test]
    fn iir_with_only_feedforward_matches_fir() {
        // When a = [1.0] (no feedback), iir(x, b, [1.0]) should
        // match the FIR direct convolution `fir(x, b)` truncated
        // to `x.len()`.  This is the cleanest cross-check between
        // the two paths.
        let signal: Vec<f32> = (0..20).map(|i| ((i as f32) * 0.3).sin()).collect();
        let kernel = vec![0.25f32, 0.5, 0.25];
        let via_iir = iir(&signal, &kernel, &[1.0]).unwrap();
        let via_fir = fir(&signal, &kernel).unwrap();
        // FIR returns N + K - 1; IIR returns N.  Compare the
        // overlapping prefix of length N.
        assert_close(&via_iir, &via_fir[..signal.len()], 1e-5);
    }

    #[test]
    fn iir_impulse_response_of_single_pole_is_geometric() {
        // For b = [1.0], a = [1.0, -p]: impulse response is
        // [1, p, p², p³, …].  We check this for p = 0.5.
        let mut impulse = vec![0.0f32; 8];
        impulse[0] = 1.0;
        let out = iir(&impulse, &[1.0], &[1.0, -0.5]).unwrap();
        let expected: Vec<f32> =
            (0..8).map(|i| 0.5f32.powi(i as i32)).collect();
        assert_close(&out, &expected, 1e-6);
    }

    #[test]
    fn iir_two_pole_dc_gain() {
        // For a stable IIR filter, DC gain is
        // (Σ b) / (Σ a).  For b = [0.0675, 0.135, 0.0675],
        // a = [1.0, -1.143, 0.413] (a 2nd-order Butterworth-ish):
        //   Σ b = 0.27
        //   Σ a = 0.27
        //   DC gain = 1.0
        // Verify step response converges to ~1.0.
        let b = vec![0.0675f32, 0.135, 0.0675];
        let a = vec![1.0f32, -1.143, 0.413];
        let step = vec![1.0f32; 200];
        let out = iir(&step, &b, &a).unwrap();
        let final_value = out[199];
        let sum_b: f32 = b.iter().sum();
        let sum_a: f32 = a.iter().sum();
        let dc_gain = sum_b / sum_a;
        assert!(
            approx_eq(final_value, dc_gain, 1e-3),
            "two-pole step asymptote = {}, expected DC gain ~{}",
            final_value,
            dc_gain
        );
    }

    #[test]
    fn iir_output_length_equals_input_length() {
        for n in [1usize, 5, 17, 64, 200] {
            let signal: Vec<f32> = (0..n).map(|i| (i as f32) * 0.1).collect();
            let out = iir(&signal, &[1.0], &[1.0, -0.5]).unwrap();
            assert_eq!(out.len(), n, "wrong output length for N = {}", n);
        }
    }
}
