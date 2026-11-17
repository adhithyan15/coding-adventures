//! Iterative pitch detection: YIN and Harmonic Product Spectrum (MUS08, VER15).

use crate::dsp_utils::{
    build_hann_window, checked_frame_bounds, parabolic_interpolation, rfft_magnitudes,
    validate_framing,
};
use crate::features::rms_energy;
use crate::types::{
    validate_positive_float, validate_samples, AnalysisError, MAX_FFT_LEN, MAX_PARTIALS,
    MAX_SAMPLE_RATE_HZ,
};

/// Estimate fundamental pitch using strictly iterative YIN algorithm.
///
/// # Arguments
/// * `samples` - Time-domain audio sample sequence.
/// * `sample_rate_hz` - Sampling frequency in Hertz.
/// * `f_min` - Minimum expected fundamental frequency in Hertz (default 40.0 Hz).
/// * `f_max` - Maximum expected fundamental frequency in Hertz (default 2000.0 Hz).
/// * `threshold` - YIN absolute threshold for cumulative normalized difference (default 0.15).
///
/// # Returns
/// Estimated fundamental frequency in Hertz, or 0.0 if unpitched/silent.
pub fn yin_pitch(
    samples: &[f64],
    sample_rate_hz: f64,
    f_min: f64,
    f_max: f64,
    threshold: f64,
) -> Result<f64, AnalysisError> {
    let valid_sr = validate_positive_float(
        sample_rate_hz,
        "sample_rate_hz",
        Some(MAX_SAMPLE_RATE_HZ),
    )?;
    let valid_fmin = validate_positive_float(f_min, "f_min", Some(valid_sr / 2.0))?;
    let valid_fmax = validate_positive_float(f_max, "f_max", Some(valid_sr / 2.0))?;
    if valid_fmin >= valid_fmax {
        return Err(AnalysisError::InvalidInput(format!(
            "f_min ({valid_fmin}) must be < f_max ({valid_fmax})"
        )));
    }
    let valid_thresh = validate_positive_float(threshold, "threshold", Some(1.0))?;
    validate_samples(samples, "samples")?;
    let n = samples.len();

    // Silence check
    if rms_energy(samples)? < 1e-4 {
        return Ok(0.0);
    }

    let tau_min = 2.max((valid_sr / valid_fmax).floor() as usize);
    let tau_max = (n / 2).min((valid_sr / valid_fmin).ceil() as usize);

    if tau_max <= tau_min {
        return Ok(0.0);
    }

    let w_size = n
        .checked_sub(tau_max)
        .ok_or_else(|| AnalysisError::BudgetExceeded("YIN window size underflow".to_string()))?;

    // Step 1: Difference function d(tau)
    let mut d = vec![0.0; tau_max + 1];
    for (tau, slot) in d.iter_mut().enumerate().take(tau_max + 1).skip(1) {
        let mut diff_sum = 0.0;
        for j in 0..w_size {
            let j2 = j.checked_add(tau).ok_or_else(|| {
                AnalysisError::BudgetExceeded("YIN sample index overflow".to_string())
            })?;
            let diff = samples[j] - samples[j2];
            diff_sum += diff * diff;
        }
        *slot = diff_sum;
    }

    // Step 2: Cumulative mean normalized difference d'(tau)
    let mut d_prime = vec![1.0; tau_max + 1];
    let mut running_sum = 0.0;
    for tau in 1..=tau_max {
        running_sum += d[tau];
        if running_sum > 0.0 {
            d_prime[tau] = d[tau] / (running_sum / (tau as f64));
        } else {
            d_prime[tau] = 1.0;
        }
    }

    // Step 3: Absolute threshold search
    let mut best_tau: Option<usize> = None;
    for tau in tau_min..=tau_max {
        if d_prime[tau] < valid_thresh {
            let mut t = tau;
            while t < tau_max && d_prime[t + 1] < d_prime[t] {
                t += 1;
            }
            best_tau = Some(t);
            break;
        }
    }

    let chosen_tau = match best_tau {
        Some(t) => t,
        None => {
            let mut min_val = f64::INFINITY;
            let mut min_tau = tau_min;
            for (tau, &val) in d_prime.iter().enumerate().take(tau_max + 1).skip(tau_min) {
                if val < min_val {
                    min_val = val;
                    min_tau = tau;
                }
            }
            if min_val > 0.85 {
                return Ok(0.0);
            }
            min_tau
        }
    };

    // Step 4: Parabolic interpolation
    let refined_tau = if tau_min < chosen_tau && chosen_tau < tau_max {
        parabolic_interpolation(
            d_prime[chosen_tau - 1],
            d_prime[chosen_tau],
            d_prime[chosen_tau + 1],
            chosen_tau,
        )
    } else {
        chosen_tau as f64
    };

    if refined_tau <= 0.0 {
        return Ok(0.0);
    }

    let pitch = valid_sr / refined_tau;
    if valid_fmin <= pitch && pitch <= valid_fmax {
        Ok(pitch)
    } else {
        Ok(0.0)
    }
}

/// Convenience wrapper for [`yin_pitch`] with standard defaults (40 Hz to 2000 Hz, threshold 0.15).
pub fn yin_pitch_simple(samples: &[f64], sample_rate_hz: f64) -> Result<f64, AnalysisError> {
    yin_pitch(samples, sample_rate_hz, 40.0, 2000.0, 0.15)
}

/// Track fundamental pitch over time using windowed YIN analysis.
#[allow(clippy::too_many_arguments)]
pub fn yin_pitch_track(
    samples: &[f64],
    sample_rate_hz: f64,
    frame_length: usize,
    hop_length: usize,
    f_min: f64,
    f_max: f64,
    threshold: f64,
) -> Result<Vec<f64>, AnalysisError> {
    validate_samples(samples, "samples")?;
    let maybe_frames = validate_framing(samples.len(), frame_length, hop_length)?;
    let num_frames = match maybe_frames {
        None => {
            let p = yin_pitch(samples, sample_rate_hz, f_min, f_max, threshold)?;
            return Ok(vec![p]);
        }
        Some(nf) => nf,
    };

    let mut pitches = Vec::with_capacity(num_frames);
    for m in 0..num_frames {
        let (start, end) = checked_frame_bounds(m, hop_length, frame_length, samples.len())?;
        let frame = &samples[start..end];
        let p = yin_pitch(frame, sample_rate_hz, f_min, f_max, threshold)?;
        pitches.push(p);
    }

    Ok(pitches)
}

/// Estimate fundamental pitch via Harmonic Product Spectrum (HPS).
///
/// # Arguments
/// * `samples` - Time-domain audio sample sequence.
/// * `sample_rate_hz` - Sampling frequency in Hertz.
/// * `num_harmonics` - Number of harmonic spectra to compress (default 5, up to 64).
/// * `f_min` - Minimum frequency search bound.
/// * `f_max` - Maximum frequency search bound.
/// * `n_fft` - Optional FFT length (defaults to power of two >= len(samples), capped at 8192).
pub fn hps_pitch(
    samples: &[f64],
    sample_rate_hz: f64,
    num_harmonics: usize,
    f_min: f64,
    f_max: f64,
    n_fft: Option<usize>,
) -> Result<f64, AnalysisError> {
    let valid_sr = validate_positive_float(
        sample_rate_hz,
        "sample_rate_hz",
        Some(MAX_SAMPLE_RATE_HZ),
    )?;
    if num_harmonics < 1 {
        return Err(AnalysisError::InvalidInput(
            "num_harmonics must be >= 1".to_string(),
        ));
    }
    if num_harmonics > MAX_PARTIALS {
        return Err(AnalysisError::BudgetExceeded(format!(
            "num_harmonics {num_harmonics} exceeds MAX_PARTIALS={MAX_PARTIALS}"
        )));
    }
    let valid_fmin = validate_positive_float(f_min, "f_min", Some(valid_sr / 2.0))?;
    let valid_fmax = validate_positive_float(f_max, "f_max", Some(valid_sr / 2.0))?;
    if valid_fmin >= valid_fmax {
        return Err(AnalysisError::InvalidInput(
            "f_min must be < f_max".to_string(),
        ));
    }
    validate_samples(samples, "samples")?;
    let n = samples.len();

    if rms_energy(samples)? < 1e-4 {
        return Ok(0.0);
    }

    let fft_len = match n_fft {
        None => {
            let p2 = if n <= 1 { 1 } else { n.next_power_of_two() };
            1024.max(p2).min(8192)
        }
        Some(len) => {
            if len == 0 {
                return Err(AnalysisError::InvalidInput(
                    "n_fft must be positive".to_string(),
                ));
            }
            if len > MAX_FFT_LEN {
                return Err(AnalysisError::BudgetExceeded(format!(
                    "n_fft {len} exceeds MAX_FFT_LEN={MAX_FFT_LEN}"
                )));
            }
            len
        }
    };

    let take_len = n.min(fft_len);
    let win = build_hann_window(take_len);
    let mut windowed = vec![0.0f64; fft_len];
    for i in 0..take_len {
        windowed[i] = samples[i] * win[i];
    }

    let mags = rfft_magnitudes(&windowed)?;
    let k_bins = mags.len();

    let freq_step = valid_sr / (fft_len as f64);
    let k_min = 1.max((valid_fmin / freq_step).floor() as usize);
    let k_max = (k_bins / num_harmonics).min((valid_fmax / freq_step).ceil() as usize);

    if k_max <= k_min {
        return Ok(0.0);
    }

    let mut best_k = k_min;
    let mut max_hps = f64::NEG_INFINITY;
    let mut hps_scores = vec![0.0f64; k_max + 2];

    for (k, score_slot) in hps_scores
        .iter_mut()
        .enumerate()
        .take(k_max + 1)
        .skip(k_min)
    {
        let mut log_prod = 0.0;
        for h in 1..=num_harmonics {
            let kh = k.checked_mul(h).ok_or_else(|| {
                AnalysisError::BudgetExceeded("HPS harmonic bin index overflow".to_string())
            })?;
            if kh < k_bins {
                log_prod += (mags[kh] + 1e-9).ln();
            }
        }
        *score_slot = log_prod;
        if log_prod > max_hps {
            max_hps = log_prod;
            best_k = k;
        }
    }

    // Parabolic interpolation
    let refined_k = if k_min < best_k && best_k < k_max {
        parabolic_interpolation(hps_scores[best_k - 1], max_hps, hps_scores[best_k + 1], best_k)
    } else {
        best_k as f64
    };

    let pitch = refined_k * freq_step;
    if valid_fmin <= pitch && pitch <= valid_fmax {
        Ok(pitch)
    } else {
        Ok(0.0)
    }
}

/// Convenience wrapper for [`hps_pitch`] with standard defaults (5 harmonics, 40 Hz to 2000 Hz).
pub fn hps_pitch_simple(samples: &[f64], sample_rate_hz: f64) -> Result<f64, AnalysisError> {
    hps_pitch(samples, sample_rate_hz, 5, 40.0, 2000.0, None)
}
