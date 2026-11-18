//! # `dsp-filters` — FIR / IIR filters for the DSP layer
//!
//! **DSP03 Phases 1-5.** Pure-Rust scalar reference for FIR (finite
//! impulse response) filtering via direct linear convolution and FFT-based
//! convolution ([`fir_fft`]), IIR filtering via direct-form-II Transposed
//! ([`iir`]), and canonical filter design helpers (windowed-sinc low/high/band-pass
//! and Butterworth/Chebyshev-I).

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

mod biquad;
pub mod design;
pub mod iir;
pub use biquad::{sosfilt, Biquad};
pub use design::{
    butterworth_lowpass, chebyshev1_lowpass, design_band_pass, design_high_pass, design_low_pass,
    try_design_band_pass, try_design_high_pass, try_design_low_pass, WindowType,
};
pub use iir::iir;

use std::fmt;

/// Maximum signal length in samples (16,777,216 samples = 64 MB of f32).
pub const MAX_SAMPLES: usize = 1 << 24;

/// Maximum filter taps (1,048,576 taps).
pub const MAX_TAPS: usize = 1 << 20;

/// Maximum IIR filter order.
pub const MAX_ORDER: usize = 1024;

/// Finite-impulse-response filter via direct linear convolution.
///
/// `signal` is a length-`N` real signal; `kernel` is a length-`K`
/// real impulse response.  Returns a `Vec<f32>` of length
/// `N + K - 1` holding the full linear convolution
/// (`numpy.convolve(signal, kernel, mode='full')`).
///
/// Boundary handling: the input is implicitly zero-padded outside
/// `[0, N)`.
///
/// Returns [`FilterError::EmptySignal`] or [`FilterError::EmptyKernel`]
/// when either input is empty, or [`FilterError::InvalidCoefficient`]
/// on non-finite floats or inputs exceeding defensive caps (`MAX_SAMPLES`, `MAX_TAPS`).
pub fn fir(signal: &[f32], kernel: &[f32]) -> Result<Vec<f32>, FilterError> {
    if signal.is_empty() {
        return Err(FilterError::EmptySignal);
    }
    if kernel.is_empty() {
        return Err(FilterError::EmptyKernel);
    }
    if signal.len() > MAX_SAMPLES {
        return Err(FilterError::InvalidCoefficient(format!(
            "signal length {} exceeds MAX_SAMPLES {}",
            signal.len(),
            MAX_SAMPLES
        )));
    }
    if kernel.len() > MAX_TAPS {
        return Err(FilterError::InvalidCoefficient(format!(
            "kernel length {} exceeds MAX_TAPS {}",
            kernel.len(),
            MAX_TAPS
        )));
    }
    let n = signal.len();
    let k = kernel.len();
    let out_len = n
        .checked_add(k)
        .and_then(|x| x.checked_sub(1))
        .ok_or_else(|| {
            FilterError::InvalidCoefficient("convolution output length overflow".into())
        })?;
    if out_len > MAX_SAMPLES {
        return Err(FilterError::InvalidCoefficient(format!(
            "convolution output length {} exceeds MAX_SAMPLES {}",
            out_len, MAX_SAMPLES
        )));
    }

    // Validate finite floats BEFORE any allocation
    for &x in signal {
        if !x.is_finite() {
            return Err(FilterError::InvalidCoefficient(
                "non-finite float in signal".into(),
            ));
        }
    }
    for &h in kernel {
        if !h.is_finite() {
            return Err(FilterError::InvalidCoefficient(
                "non-finite float in kernel".into(),
            ));
        }
    }

    let mut out = vec![0.0f32; out_len];

    for i in 0..out_len {
        let j_lo = (i + 1).saturating_sub(n);
        let j_hi = if i + 1 < k { i + 1 } else { k };
        let mut acc = 0.0f32;
        for j in j_lo..j_hi {
            acc += kernel[j] * signal[i - j];
        }
        out[i] = acc;
    }
    Ok(out)
}

/// FIR filtering via FFT-based convolution for long kernels.
///
/// Computes the exact linear convolution of `signal` (length $N$) and
/// `kernel` (length $K$) returning length $N + K - 1$, matching [`fir`]
/// within numerical tolerance.
pub fn fir_fft(signal: &[f32], kernel: &[f32]) -> Result<Vec<f32>, FilterError> {
    if signal.is_empty() {
        return Err(FilterError::EmptySignal);
    }
    if kernel.is_empty() {
        return Err(FilterError::EmptyKernel);
    }
    if signal.len() > MAX_SAMPLES {
        return Err(FilterError::InvalidCoefficient(format!(
            "signal length {} exceeds MAX_SAMPLES {}",
            signal.len(),
            MAX_SAMPLES
        )));
    }
    if kernel.len() > MAX_TAPS {
        return Err(FilterError::InvalidCoefficient(format!(
            "kernel length {} exceeds MAX_TAPS {}",
            kernel.len(),
            MAX_TAPS
        )));
    }
    let n = signal.len();
    let k = kernel.len();
    let out_len = n
        .checked_add(k)
        .and_then(|x| x.checked_sub(1))
        .ok_or_else(|| {
            FilterError::InvalidCoefficient("convolution output length overflow".into())
        })?;
    if out_len > MAX_SAMPLES {
        return Err(FilterError::InvalidCoefficient(format!(
            "convolution output length {} exceeds MAX_SAMPLES {}",
            out_len, MAX_SAMPLES
        )));
    }

    for &x in signal {
        if !x.is_finite() {
            return Err(FilterError::InvalidCoefficient(
                "non-finite float in signal".into(),
            ));
        }
    }
    for &h in kernel {
        if !h.is_finite() {
            return Err(FilterError::InvalidCoefficient(
                "non-finite float in kernel".into(),
            ));
        }
    }

    let fft_len = out_len
        .checked_next_power_of_two()
        .ok_or_else(|| FilterError::InvalidCoefficient("FFT length overflow".into()))?
        .max(2);
    if fft_len > MAX_SAMPLES {
        return Err(FilterError::InvalidCoefficient(format!(
            "FFT length {} exceeds MAX_SAMPLES {}",
            fft_len, MAX_SAMPLES
        )));
    }

    let mut sig_pad = vec![0.0f32; fft_len];
    sig_pad[..n].copy_from_slice(signal);

    let mut ker_pad = vec![0.0f32; fft_len];
    ker_pad[..k].copy_from_slice(kernel);

    let sig_spec = dsp_fft::rfft_scalar(&sig_pad).map_err(|e| FilterError::Fft(e.to_string()))?;
    let ker_spec = dsp_fft::rfft_scalar(&ker_pad).map_err(|e| FilterError::Fft(e.to_string()))?;

    let bins = fft_len / 2 + 1;
    let mut prod = vec![0.0f32; bins * 2];
    for b in 0..bins {
        let r1 = sig_spec[2 * b];
        let i1 = sig_spec[2 * b + 1];
        let r2 = ker_spec[2 * b];
        let i2 = ker_spec[2 * b + 1];
        prod[2 * b] = r1 * r2 - i1 * i2;
        prod[2 * b + 1] = r1 * i2 + i1 * r2;
    }

    let mut out = dsp_fft::irfft_scalar(&prod, fft_len as u32)
        .map_err(|e| FilterError::Fft(e.to_string()))?;
    out.truncate(out_len);
    Ok(out)
}

/// Errors produced by filter primitives.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterError {
    /// `signal` is empty.
    EmptySignal,
    /// `kernel` is empty.
    EmptyKernel,
    /// Invalid coefficient, non-finite float, or parameter exceeding defensive caps.
    InvalidCoefficient(String),
    /// FFT failure.
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

impl From<dsp_fft::FftError> for FilterError {
    fn from(e: dsp_fft::FftError) -> Self {
        FilterError::Fft(e.to_string())
    }
}

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

    fn assert_bits(actual: &[f32], expected: &[u32]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (value, bits)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                value.to_bits(),
                *bits,
                "coefficient bit mismatch at {index}"
            );
        }
    }

    fn sample_f32(rng: &mut SplitMix64) -> f32 {
        let u = rng.next_u64();
        let mantissa = (u & 0x00ff_ffff) as f32 / (0x0100_0000 as f32);
        (mantissa * 2.0) - 1.0
    }

    #[test]
    fn checked_designers_match_legacy_coefficient_bits() {
        for &window in &[
            WindowType::Rectangular,
            WindowType::Hamming,
            WindowType::Hann,
            WindowType::Blackman,
        ] {
            for &cutoff in &[0.0, 0.1, 0.25, 0.5] {
                let legacy = design_low_pass(cutoff, 5, window);
                let checked = try_design_low_pass(cutoff, 5, window).unwrap();
                assert_eq!(
                    legacy.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    checked.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
                );

                let legacy = design_high_pass(cutoff, 5, window);
                let checked = try_design_high_pass(cutoff, 5, window).unwrap();
                assert_eq!(
                    legacy.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    checked.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
                );
            }
            let legacy = design_band_pass(0.0, 0.5, 5, window);
            let checked = try_design_band_pass(0.0, 0.5, 5, window).unwrap();
            assert_eq!(
                legacy.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                checked.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn legacy_designers_match_prechecked_algebraic_bit_baselines() {
        // Captured from the public functions at baseline
        // 7e9f7d57a70fd446288888e48e8fea8e868019dc. These one-tap and
        // signed-zero cases do not depend on platform libm precision.
        assert_bits(
            &design_low_pass(-0.0, 1, WindowType::Rectangular),
            &[0x8000_0000],
        );
        assert_bits(
            &design_low_pass(0.25, 1, WindowType::Rectangular),
            &[0x3f80_0000],
        );
        assert_bits(
            &design_high_pass(0.0, 1, WindowType::Rectangular),
            &[0x3f80_0000],
        );
        assert_bits(
            &design_band_pass(0.0, 0.5, 1, WindowType::Rectangular),
            &[0x3f80_0000],
        );
    }

    #[test]
    fn checked_designers_validate_before_tap_allocation() {
        assert!(matches!(
            try_design_low_pass(f32::NAN, 0, WindowType::Rectangular),
            Err(FilterError::InvalidCoefficient(_))
        ));
        assert!(matches!(
            try_design_high_pass(0.2, 2, WindowType::Rectangular),
            Err(FilterError::InvalidCoefficient(_))
        ));
        assert!(matches!(
            try_design_band_pass(f32::NAN, 0.2, 0, WindowType::Rectangular),
            Err(FilterError::InvalidCoefficient(_))
        ));

        let cap_error =
            match try_design_low_pass(0.2, (MAX_TAPS + 1) as u32, WindowType::Rectangular) {
                Err(error) => error,
                Ok(_) => panic!("MAX_TAPS + 1 must be rejected before building a kernel"),
            };
        assert!(matches!(cap_error, FilterError::InvalidCoefficient(_)));
        let bp_cap_error = match try_design_band_pass(
            0.1,
            f32::INFINITY,
            (MAX_TAPS + 1) as u32,
            WindowType::Rectangular,
        ) {
            Err(error) => error,
            Ok(_) => panic!("band-pass tap cap must precede cutoff validation"),
        };
        assert_eq!(
            bp_cap_error,
            FilterError::InvalidCoefficient(format!(
                "num_taps {} exceeds MAX_TAPS {}",
                MAX_TAPS + 1,
                MAX_TAPS
            ))
        );

        // Exercise the largest accepted odd count once; this is a
        // bounded allocation below the documented tap cap.
        let largest_odd =
            try_design_low_pass(0.25, (MAX_TAPS - 1) as u32, WindowType::Rectangular).unwrap();
        assert_eq!(largest_odd.len(), MAX_TAPS - 1);
    }

    #[test]
    fn checked_designers_report_validation_order_and_error_text() {
        assert_eq!(
            try_design_low_pass(f32::NAN, 0, WindowType::Rectangular).unwrap_err(),
            FilterError::InvalidCoefficient("num_taps must be odd and >= 1; got 0".into())
        );
        assert_eq!(
            try_design_high_pass(0.6, 3, WindowType::Rectangular).unwrap_err(),
            FilterError::InvalidCoefficient("cutoff_norm must be in [0.0, 0.5]; got 0.6".into())
        );
        assert_eq!(
            try_design_band_pass(0.2, 0.2, 3, WindowType::Rectangular).unwrap_err(),
            FilterError::InvalidCoefficient(
                "cutoffs must satisfy 0.0 <= low < high <= 0.5; got low=0.2, high=0.2".into()
            )
        );
        assert_eq!(
            try_design_band_pass(0.2, f32::NAN, 3, WindowType::Rectangular).unwrap_err(),
            FilterError::InvalidCoefficient("cutoffs must be finite".into())
        );
    }

    #[test]
    fn checked_designers_match_legacy_for_seeded_valid_domain_samples() {
        let windows = [
            WindowType::Rectangular,
            WindowType::Hamming,
            WindowType::Hann,
            WindowType::Blackman,
        ];
        let mut calls = 0usize;
        for seed in [0x000a_0d10, 0x51ba_da55] {
            let mut rng = SplitMix64::new(seed);
            for _ in 0..64 {
                let word = rng.next_u64();
                let unit = ((word >> 40) as u32) as f32 / 16_777_216.0;
                let cutoff = unit * 0.5;
                let taps = 1 + 2 * ((rng.next_u64() % 16) as u32);
                let window = windows[(rng.next_u64() % windows.len() as u64) as usize];
                let lp = try_design_low_pass(cutoff, taps, window).unwrap();
                let legacy_lp = design_low_pass(cutoff, taps, window);
                assert_eq!(
                    lp.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    legacy_lp.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
                );
                let hp = try_design_high_pass(cutoff, taps, window).unwrap();
                let legacy_hp = design_high_pass(cutoff, taps, window);
                assert_eq!(
                    hp.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    legacy_hp.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
                );
                let high = 0.01 + cutoff * 0.98;
                let low = high * 0.2;
                let bp = try_design_band_pass(low, high, taps, window).unwrap();
                let legacy_bp = design_band_pass(low, high, taps, window);
                assert_eq!(
                    bp.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    legacy_bp.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
                );
                calls += 3;
            }
        }
        assert_eq!(calls, 384);
    }

    #[test]
    fn legacy_designers_preserve_panic_payload_type_and_message() {
        let formatted =
            std::panic::catch_unwind(|| design_low_pass(0.2, 0, WindowType::Rectangular))
                .unwrap_err();
        assert_eq!(
            formatted.downcast_ref::<String>().map(String::as_str),
            Some("num_taps must be odd and >= 1; got 0")
        );

        let literal = std::panic::catch_unwind(|| {
            design_band_pass(0.1, f32::INFINITY, 3, WindowType::Rectangular)
        })
        .unwrap_err();
        assert_eq!(
            literal.downcast_ref::<&'static str>().copied(),
            Some("cutoffs must be finite")
        );

        let formatted_bp =
            std::panic::catch_unwind(|| design_band_pass(0.2, 0.2, 3, WindowType::Rectangular))
                .unwrap_err();
        assert_eq!(
            formatted_bp.downcast_ref::<String>().map(String::as_str),
            Some("cutoffs must satisfy 0.0 <= low < high <= 0.5; got low=0.2, high=0.2")
        );
    }

    #[test]
    fn fir_rejects_empty_signal() {
        let err = fir(&[], &[1.0, 2.0]).unwrap_err();
        assert_eq!(err, FilterError::EmptySignal);
    }

    #[test]
    fn fir_rejects_empty_kernel() {
        let err = fir(&[1.0, 2.0], &[]).unwrap_err();
        assert_eq!(err, FilterError::EmptyKernel);
    }

    #[test]
    fn fir_multiplies_single_sample_coefficients() {
        assert_eq!(fir(&[2.0], &[2.0]).unwrap(), vec![4.0]);
    }

    #[test]
    fn fir_output_length_equals_n_plus_k_minus_one() {
        for n in 1..=10 {
            for k in 1..=10 {
                let signal = vec![1.0f32; n];
                let kernel = vec![1.0f32; k];
                let out = fir(&signal, &kernel).unwrap();
                assert_eq!(out.len(), n + k - 1);
            }
        }
    }

    #[test]
    fn fir_identity_kernel_returns_signal() {
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let kernel = vec![1.0f32];
        let out = fir(&signal, &kernel).unwrap();
        assert_close(&out, &signal, 1e-7);
    }

    #[test]
    fn fir_fft_matches_fir_direct() {
        let signal = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let kernel = vec![0.25f32, 0.5, 0.25];
        let out_dir = fir(&signal, &kernel).unwrap();
        let out_fft = fir_fft(&signal, &kernel).unwrap();
        assert_close(&out_dir, &out_fft, 1e-4);
    }

    #[test]
    fn hostile_non_finite_rejected() {
        assert!(matches!(
            fir(&[1.0, f32::NAN], &[1.0]),
            Err(FilterError::InvalidCoefficient(_))
        ));
        assert!(matches!(
            fir(&[1.0], &[f32::INFINITY]),
            Err(FilterError::InvalidCoefficient(_))
        ));
        assert!(matches!(
            fir_fft(&[1.0, f32::NAN], &[1.0]),
            Err(FilterError::InvalidCoefficient(_))
        ));
        assert!(matches!(
            fir_fft(&[1.0], &[f32::NEG_INFINITY]),
            Err(FilterError::InvalidCoefficient(_))
        ));
    }

    #[test]
    fn pbt_splitmix64_fir_direct_vs_fft() {
        let mut rng = SplitMix64::new(0x1234_5678_9abc_def0);
        let pairs = [(8, 3), (16, 5), (32, 8), (64, 17), (128, 33), (100, 20)];
        for (n, k) in pairs {
            let mut sig = Vec::with_capacity(n);
            for _ in 0..n {
                sig.push(sample_f32(&mut rng));
            }
            let mut ker = Vec::with_capacity(k);
            for _ in 0..k {
                ker.push(sample_f32(&mut rng));
            }

            let direct = fir(&sig, &ker).unwrap();
            let via_fft = fir_fft(&sig, &ker).unwrap();
            assert_close(&direct, &via_fft, 1e-4);
        }
    }
}
