//! Shared numerical routines, DSP helpers, framing validation, and CWE-190 guards (VER15).

use dsp_fft::rfft_scalar;

use crate::types::{
    validate_positive_float, validate_samples, AnalysisError, MAX_FFT_LEN, MAX_FRAMES,
    MAX_SAMPLE_RATE_HZ, MAX_SAMPLES, MIN_SAMPLE_RATE_HZ,
};

/// Build a symmetric Hann analysis window of length `n`.
pub fn build_hann_window(n: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![1.0];
    }
    let m = (n - 1) as f64;
    let two_pi = 2.0 * std::f64::consts::PI;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let val = 0.5 * (1.0 - (two_pi * (i as f64) / m).cos());
        out.push(val);
    }
    out
}

/// Refine a discrete peak position `peak_x` using 3-point parabolic interpolation.
///
/// Refines around $(peak\_x - 1, y_0)$, $(peak\_x, y_1)$, $(peak\_x + 1, y_2)$.
pub fn parabolic_interpolation(y0: f64, y1: f64, y2: f64, peak_x: usize) -> f64 {
    let denom = 2.0 * (y0 - 2.0 * y1 + y2);
    if denom.abs() > 1e-12 {
        let delta = (y0 - y2) / denom;
        if delta.abs() < 1.0 {
            (peak_x as f64) + delta
        } else {
            peak_x as f64
        }
    } else {
        peak_x as f64
    }
}

/// Compute the real FFT magnitude spectrum of windowed time-domain audio samples.
///
/// Returns `Vec<f64>` of length `fft_len / 2 + 1`.
pub fn rfft_magnitudes(windowed: &[f64]) -> Result<Vec<f64>, AnalysisError> {
    let fft_len = windowed.len();
    if fft_len == 0 {
        return Err(AnalysisError::InvalidInput(
            "windowed buffer must be non-empty".to_string(),
        ));
    }
    if fft_len > MAX_FFT_LEN {
        return Err(AnalysisError::BudgetExceeded(format!(
            "windowed buffer length {fft_len} exceeds MAX_FFT_LEN={MAX_FFT_LEN}"
        )));
    }

    let mut windowed_f32 = Vec::with_capacity(fft_len);
    for &s in windowed {
        windowed_f32.push(s as f32);
    }

    let spec = rfft_scalar(&windowed_f32)
        .map_err(|e| AnalysisError::InvalidInput(format!("FFT error: {e:?}")))?;

    let k_bins = fft_len
        .checked_div(2)
        .and_then(|k| k.checked_add(1))
        .ok_or_else(|| {
            AnalysisError::BudgetExceeded("FFT bin count overflowed usize".to_string())
        })?;

    let mut mags = Vec::with_capacity(k_bins);
    for k in 0..k_bins {
        let re_idx = k.checked_mul(2).ok_or_else(|| {
            AnalysisError::BudgetExceeded("FFT spectrum index overflow".to_string())
        })?;
        let im_idx = re_idx.checked_add(1).ok_or_else(|| {
            AnalysisError::BudgetExceeded("FFT spectrum index overflow".to_string())
        })?;
        let re = spec[re_idx] as f64;
        let im = spec[im_idx] as f64;
        mags.push((re * re + im * im).sqrt());
    }

    Ok(mags)
}

/// Compute single-step half-wave rectified or squared spectral flux between two frames.
pub fn compute_spectral_flux_step(prev: &[f64], curr: &[f64], rectified: bool) -> f64 {
    let n_bins = prev.len().min(curr.len());
    let mut flux = 0.0;
    for k in 0..n_bins {
        let diff = curr[k] - prev[k];
        if rectified {
            if diff > 0.0 {
                flux += diff;
            }
        } else {
            flux += diff * diff;
        }
    }
    flux
}

/// Validate framing parameters and compute total frame count with CWE-400 / CWE-190 enforcement.
///
/// Returns `Ok(Some(num_frames))` if `signal_len >= frame_length`, or `Ok(None)` if `signal_len < frame_length`.
pub fn validate_framing(
    signal_len: usize,
    frame_length: usize,
    hop_length: usize,
) -> Result<Option<usize>, AnalysisError> {
    if frame_length == 0 || hop_length == 0 {
        return Err(AnalysisError::InvalidInput(
            "frame_length and hop_length must be positive integers".to_string(),
        ));
    }
    if frame_length > MAX_SAMPLES {
        return Err(AnalysisError::BudgetExceeded(format!(
            "frame_length {frame_length} exceeds MAX_SAMPLES={MAX_SAMPLES}"
        )));
    }
    if hop_length > MAX_SAMPLES {
        return Err(AnalysisError::BudgetExceeded(format!(
            "hop_length {hop_length} exceeds MAX_SAMPLES={MAX_SAMPLES}"
        )));
    }

    if signal_len < frame_length {
        return Ok(None);
    }

    let diff = signal_len
        .checked_sub(frame_length)
        .ok_or_else(|| AnalysisError::BudgetExceeded("framing subtraction underflow".to_string()))?;

    let num_frames = 1usize
        .checked_add(diff / hop_length)
        .ok_or_else(|| AnalysisError::BudgetExceeded("num_frames calculation overflow".to_string()))?;

    if num_frames > MAX_FRAMES {
        return Err(AnalysisError::BudgetExceeded(format!(
            "num_frames {num_frames} exceeds MAX_FRAMES={MAX_FRAMES}"
        )));
    }

    Ok(Some(num_frames))
}

/// Safely calculate `(start, end)` slice boundaries for frame `m` with `checked_mul` and `checked_add`.
pub fn checked_frame_bounds(
    m: usize,
    hop_length: usize,
    frame_length: usize,
    signal_len: usize,
) -> Result<(usize, usize), AnalysisError> {
    let start = m.checked_mul(hop_length).ok_or_else(|| {
        AnalysisError::BudgetExceeded("frame start offset overflowed usize".to_string())
    })?;
    let end = start.checked_add(frame_length).ok_or_else(|| {
        AnalysisError::BudgetExceeded("frame end offset overflowed usize".to_string())
    })?;
    if end > signal_len {
        return Err(AnalysisError::BudgetExceeded(format!(
            "frame end {end} exceeds signal length {signal_len}"
        )));
    }
    Ok((start, end))
}

/// Common validation helper for audio sample buffer and sample rate.
pub fn validate_audio_inputs(
    samples: &[f64],
    sample_rate_hz: f64,
) -> Result<f64, AnalysisError> {
    let valid_sr = validate_positive_float(
        sample_rate_hz,
        "sample_rate_hz",
        Some(MAX_SAMPLE_RATE_HZ),
    )?;
    validate_samples(samples, "samples")?;
    Ok(valid_sr)
}

/// Standardized validation helper for `TranscribePcmSource` implementations.
///
/// Enforces sample rate bounds ([`MIN_SAMPLE_RATE_HZ`], [`MAX_SAMPLE_RATE_HZ`])
/// and sample count bounds ([1, [`MAX_SAMPLES`]]).
pub fn validate_pcm_source_bounds(len: usize, sample_rate_hz: f64) -> Result<f64, AnalysisError> {
    let valid_sr = validate_positive_float(
        sample_rate_hz,
        "sample_rate_hz",
        Some(MAX_SAMPLE_RATE_HZ),
    )?;
    if valid_sr < MIN_SAMPLE_RATE_HZ {
        return Err(AnalysisError::InvalidInput(format!(
            "sample_rate_hz {valid_sr} below minimum {MIN_SAMPLE_RATE_HZ}"
        )));
    }
    if len == 0 {
        return Err(AnalysisError::InvalidInput(
            "samples must be non-empty".to_string(),
        ));
    }
    if len > MAX_SAMPLES {
        return Err(AnalysisError::BudgetExceeded(format!(
            "sample count {len} exceeds MAX_SAMPLES={MAX_SAMPLES}"
        )));
    }
    Ok(valid_sr)
}
