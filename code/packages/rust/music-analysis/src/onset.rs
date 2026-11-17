//! Onset detection via spectral flux and wavelet detail energy (MUS08, VER15).

use dsp_stft::{stft, WindowType};
use dsp_wavelets::{dwt_1d, slice_level, Band, WaveletBoundary, WaveletType};

use crate::dsp_utils::compute_spectral_flux_step;
use crate::features::rms_energy;
use crate::types::{
    validate_positive_float, validate_samples, AnalysisError, MAX_FFT_LEN, MAX_FRAMES, MAX_ONSETS,
    MAX_SAMPLE_RATE_HZ, MAX_SAMPLES,
};

/// Detect musical note onset timestamps (in seconds) combining STFT flux and DWT energy.
///
/// # Arguments
/// * `samples` - Time-domain audio sample buffer.
/// * `sample_rate_hz` - Audio sampling frequency in Hertz.
/// * `hop_length` - Step size between analysis windows (default 512).
/// * `n_fft` - Window size for Short-Time Fourier Transform (default 1024).
/// * `threshold_factor` - Multiplier for adaptive standard-deviation threshold (default 1.5).
/// * `min_distance_seconds` - Refractory lockout duration between consecutive onsets (default 0.08 s).
///
/// # Returns
/// List of detected onset timestamps in seconds.
pub fn detect_onsets(
    samples: &[f64],
    sample_rate_hz: f64,
    hop_length: usize,
    n_fft: usize,
    threshold_factor: f64,
    min_distance_seconds: f64,
) -> Result<Vec<f64>, AnalysisError> {
    let valid_sr = validate_positive_float(
        sample_rate_hz,
        "sample_rate_hz",
        Some(MAX_SAMPLE_RATE_HZ),
    )?;
    let sample_onsets = detect_onsets_samples(
        samples,
        sample_rate_hz,
        hop_length,
        n_fft,
        threshold_factor,
        min_distance_seconds,
    )?;
    Ok(sample_onsets
        .into_iter()
        .map(|s| (s as f64) / valid_sr)
        .collect())
}

/// Convenience wrapper for [`detect_onsets`] using standard defaults.
pub fn detect_onsets_simple(
    samples: &[f64],
    sample_rate_hz: f64,
) -> Result<Vec<f64>, AnalysisError> {
    detect_onsets(samples, sample_rate_hz, 512, 1024, 1.5, 0.08)
}

/// Detect musical note onset sample indices combining STFT flux and DWT energy.
pub fn detect_onsets_samples(
    samples: &[f64],
    sample_rate_hz: f64,
    hop_length: usize,
    n_fft: usize,
    threshold_factor: f64,
    min_distance_seconds: f64,
) -> Result<Vec<usize>, AnalysisError> {
    let valid_sr = validate_positive_float(
        sample_rate_hz,
        "sample_rate_hz",
        Some(MAX_SAMPLE_RATE_HZ),
    )?;
    validate_samples(samples, "samples")?;
    let n = samples.len();

    if hop_length == 0 {
        return Err(AnalysisError::InvalidInput(
            "hop_length must be positive".to_string(),
        ));
    }
    if hop_length > MAX_SAMPLES {
        return Err(AnalysisError::BudgetExceeded(format!(
            "hop_length {hop_length} exceeds MAX_SAMPLES={MAX_SAMPLES}"
        )));
    }
    if n_fft == 0 {
        return Err(AnalysisError::InvalidInput(
            "n_fft must be positive".to_string(),
        ));
    }
    if n_fft > MAX_FFT_LEN {
        return Err(AnalysisError::BudgetExceeded(format!(
            "n_fft {n_fft} exceeds MAX_FFT_LEN={MAX_FFT_LEN}"
        )));
    }
    let t_factor = validate_positive_float(threshold_factor, "threshold_factor", Some(10.0))?;
    let min_dist_sec =
        validate_positive_float(min_distance_seconds, "min_distance_seconds", Some(10.0))?;

    if rms_energy(samples)? < 1e-4 {
        return Ok(Vec::new());
    }

    // If clip is shorter than one FFT window, return single onset at 0 if active
    if n < n_fft {
        return Ok(vec![0]);
    }

    // Pre-flight frame budget enforcement BEFORE allocating
    let expected_frames = 1 + (n - n_fft) / hop_length;
    if expected_frames > MAX_FRAMES {
        return Err(AnalysisError::BudgetExceeded(format!(
            "frame count {expected_frames} exceeds MAX_FRAMES={MAX_FRAMES}"
        )));
    }

    // --- 1. Spectral Flux Novelty Curve ---
    let sig_f32: Vec<f32> = samples.iter().map(|&s| s as f32).collect();
    let stft_matrix = stft(&sig_f32, n_fft as u32, hop_length as u32, WindowType::Hann)
        .map_err(|e| AnalysisError::InvalidInput(format!("STFT error: {e:?}")))?;

    let bins = n_fft
        .checked_div(2)
        .and_then(|b| b.checked_add(1))
        .ok_or_else(|| AnalysisError::BudgetExceeded("bins calculation overflow".to_string()))?;
    let frame_floats = bins
        .checked_mul(2)
        .ok_or_else(|| AnalysisError::BudgetExceeded("frame_floats calculation overflow".to_string()))?;
    let num_frames = stft_matrix.len() / frame_floats;

    if num_frames <= 1 {
        return Ok(vec![0]);
    }

    let mut mags = Vec::with_capacity(num_frames);
    for m in 0..num_frames {
        let offset = m.checked_mul(frame_floats).ok_or_else(|| {
            AnalysisError::BudgetExceeded("STFT frame offset overflow".to_string())
        })?;
        let mut frame_mag = vec![0.0f64; bins];
        for (k, slot) in frame_mag.iter_mut().enumerate() {
            let k2 = k.checked_mul(2).ok_or_else(|| {
                AnalysisError::BudgetExceeded("STFT bin index overflow".to_string())
            })?;
            let re_idx = offset.checked_add(k2).ok_or_else(|| {
                AnalysisError::BudgetExceeded("STFT bin index overflow".to_string())
            })?;
            let im_idx = re_idx.checked_add(1).ok_or_else(|| {
                AnalysisError::BudgetExceeded("STFT bin index overflow".to_string())
            })?;
            let re = stft_matrix[re_idx] as f64;
            let im = stft_matrix[im_idx] as f64;
            *slot = (re * re + im * im).sqrt();
        }
        mags.push(frame_mag);
    }

    let mut sf = vec![0.0f64; num_frames];
    for m in 1..num_frames {
        sf[m] = compute_spectral_flux_step(&mags[m - 1], &mags[m], true);
    }

    let mut max_sf = 0.0f64;
    for &val in &sf {
        if val > max_sf {
            max_sf = val;
        }
    }
    if max_sf > 0.0 {
        for val in &mut sf {
            *val /= max_sf;
        }
    }

    // --- 2. Wavelet Detail Energy Difference Curve ---
    let cd_result = dwt_1d(&sig_f32, WaveletType::Haar, 1, WaveletBoundary::Symmetric);
    let mut we_diff = vec![0.0f64; num_frames];

    if let Ok(ref coeffs) = cd_result {
        if let Ok(cd) = slice_level(coeffs, n, 1, 1, Band::Detail) {
            let d_hop = 1.max(hop_length / 2);
            let n_cd = cd.len();
            let mut raw_we = vec![0.0f64; num_frames];
            for (m, slot) in raw_we.iter_mut().enumerate() {
                let c_start = m.checked_mul(d_hop).ok_or_else(|| {
                    AnalysisError::BudgetExceeded(
                        "wavelet coefficient start offset overflow".to_string(),
                    )
                })?;
                let c_end_bound = c_start.checked_add(d_hop).ok_or_else(|| {
                    AnalysisError::BudgetExceeded(
                        "wavelet coefficient end offset overflow".to_string(),
                    )
                })?;
                let c_end = n_cd.min(c_end_bound);
                if c_start < n_cd {
                    let mut e_sum = 0.0;
                    for &val in &cd[c_start..c_end] {
                        let v = val as f64;
                        e_sum += v * v;
                    }
                    *slot = e_sum;
                }
            }

            for m in 1..num_frames {
                let diff = raw_we[m] - raw_we[m - 1];
                if diff > 0.0 {
                    we_diff[m] = diff;
                }
            }

            let mut max_we = 0.0f64;
            for &val in &we_diff {
                if val > max_we {
                    max_we = val;
                }
            }
            if max_we > 0.0 {
                for val in &mut we_diff {
                    *val /= max_we;
                }
            }
        }
    }

    // --- 3. Combined Novelty Curve ---
    let mut novelty = vec![0.0f64; num_frames];
    for m in 0..num_frames {
        novelty[m] = 0.5 * sf[m] + 0.5 * we_diff[m];
    }

    // --- 4. Adaptive Peak Picking ---
    let window_half = 3usize; // 7-frame moving window
    let mut detected_frames = Vec::new();

    // Check if frame 0 starts with immediate active energy
    let initial_check_len = n.min(n_fft);
    if rms_energy(&samples[..initial_check_len])? > 0.02 {
        detected_frames.push(0);
    }

    let min_dist_frames =
        1.max((min_dist_sec * valid_sr / (hop_length as f64)).round() as usize);

    for m in 1..num_frames - 1 {
        let val = novelty[m];
        // Must be local peak
        if val <= novelty[m - 1] || val < novelty[m + 1] {
            continue;
        }

        let w_start = m.saturating_sub(window_half);
        let w_end_bound = m
            .checked_add(window_half)
            .and_then(|w| w.checked_add(1))
            .ok_or_else(|| {
                AnalysisError::BudgetExceeded("moving window end overflow".to_string())
            })?;
        let w_end = num_frames.min(w_end_bound);
        let w_len = (w_end - w_start) as f64;
        let mean_val = novelty[w_start..w_end].iter().sum::<f64>() / w_len;
        let var_sum: f64 = novelty[w_start..w_end]
            .iter()
            .map(|&x| (x - mean_val) * (x - mean_val))
            .sum();
        let std_val = (var_sum / w_len).sqrt();

        let threshold = mean_val + t_factor * std_val;
        let is_far_enough = match detected_frames.last() {
            None => true,
            Some(&last) => (m - last) >= min_dist_frames,
        };

        if val >= threshold && val > 0.15 && is_far_enough {
            detected_frames.push(m);
        }
    }

    if detected_frames.len() > MAX_ONSETS {
        return Err(AnalysisError::BudgetExceeded(format!(
            "detected onsets {} exceeds MAX_ONSETS={MAX_ONSETS}",
            detected_frames.len()
        )));
    }

    if detected_frames.is_empty() {
        detected_frames.push(0);
    }

    let mut out_onsets = Vec::with_capacity(detected_frames.len());
    for f in detected_frames {
        let sample_idx = f.checked_mul(hop_length).ok_or_else(|| {
            AnalysisError::BudgetExceeded("detected onset sample index overflow".to_string())
        })?;
        out_onsets.push(sample_idx);
    }

    Ok(out_onsets)
}
