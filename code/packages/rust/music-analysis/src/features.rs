//! Spectral and time-domain audio feature extraction (MUS08, VER15).

use crate::dsp_utils::{checked_frame_bounds, compute_spectral_flux_step, validate_framing};
use crate::types::{
    validate_positive_float, validate_samples, validate_spectrum, AnalysisError, MAX_FRAMES,
    MAX_SAMPLE_RATE_HZ,
};

/// Compute the spectral centroid (center of mass / brightness) of a spectrum.
///
/// # Arguments
/// * `spectrum` - Magnitude or power spectrum bins from 0 Hz to Nyquist.
/// * `sample_rate_hz` - Sampling frequency in Hertz (up to 192 kHz).
///
/// # Returns
/// Spectral centroid frequency in Hertz.
pub fn spectral_centroid(spectrum: &[f64], sample_rate_hz: f64) -> Result<f64, AnalysisError> {
    let valid_sr = validate_positive_float(
        sample_rate_hz,
        "sample_rate_hz",
        Some(MAX_SAMPLE_RATE_HZ),
    )?;
    validate_spectrum(spectrum, "spectrum")?;
    let k_bins = spectrum.len();
    if k_bins <= 1 {
        return Ok(0.0);
    }

    let nyquist = valid_sr / 2.0;
    let freq_step = nyquist / (k_bins - 1) as f64;

    let mut weighted_sum = 0.0;
    let mut total_mag = 0.0;

    // Iterative center of mass computation (CWE-674 zero recursion)
    for (k, &val) in spectrum.iter().enumerate() {
        let mag = val.abs();
        let freq = (k as f64) * freq_step;
        weighted_sum += freq * mag;
        total_mag += mag;
    }

    if total_mag <= 0.0 {
        return Ok(0.0);
    }
    Ok(weighted_sum / total_mag)
}

/// Compute spectral flux between two consecutive spectral frames.
///
/// # Arguments
/// * `spectrum_prev` - Previous frame spectrum.
/// * `spectrum_curr` - Current frame spectrum.
/// * `rectified` - If true, only positive energy increases are summed (half-wave rectified).
pub fn spectral_flux(
    spectrum_prev: &[f64],
    spectrum_curr: &[f64],
    rectified: bool,
) -> Result<f64, AnalysisError> {
    validate_spectrum(spectrum_prev, "spectrum_prev")?;
    validate_spectrum(spectrum_curr, "spectrum_curr")?;
    Ok(compute_spectral_flux_step(spectrum_prev, spectrum_curr, rectified))
}

/// Compute spectral flux across a sequence of successive frames.
pub fn spectral_flux_sequence<T: AsRef<[f64]>>(
    frames: &[T],
    rectified: bool,
) -> Result<Vec<f64>, AnalysisError> {
    let n_frames = frames.len();
    if n_frames == 0 {
        return Err(AnalysisError::InvalidInput(
            "frame count must be >= 1".to_string(),
        ));
    }
    if n_frames > MAX_FRAMES {
        return Err(AnalysisError::BudgetExceeded(format!(
            "frame count {n_frames} exceeds MAX_FRAMES={MAX_FRAMES}"
        )));
    }
    if n_frames == 1 {
        validate_spectrum(frames[0].as_ref(), "frames[0]")?;
        return Ok(Vec::new());
    }

    let mut fluxes = Vec::with_capacity(n_frames - 1);
    let mut prev = frames[0].as_ref();
    validate_spectrum(prev, "frames[0]")?;

    for (m, frame) in frames.iter().enumerate().skip(1) {
        let curr = frame.as_ref();
        validate_spectrum(curr, &format!("frames[{m}]"))?;
        let flux = compute_spectral_flux_step(prev, curr, rectified);
        fluxes.push(flux);
        prev = curr;
    }

    Ok(fluxes)
}

/// Compute spectral rolloff frequency below which `rolloff_percent` energy lies.
///
/// # Arguments
/// * `spectrum` - Magnitude or power spectrum bins from 0 Hz to Nyquist.
/// * `sample_rate_hz` - Sampling frequency in Hertz.
/// * `rolloff_percent` - Fraction of total energy in (0.0, 1.0] (default 0.85).
pub fn spectral_rolloff(
    spectrum: &[f64],
    sample_rate_hz: f64,
    rolloff_percent: f64,
) -> Result<f64, AnalysisError> {
    let valid_sr = validate_positive_float(
        sample_rate_hz,
        "sample_rate_hz",
        Some(MAX_SAMPLE_RATE_HZ),
    )?;
    let valid_pct = validate_positive_float(rolloff_percent, "rolloff_percent", Some(1.0))?;
    validate_spectrum(spectrum, "spectrum")?;
    let k_bins = spectrum.len();
    if k_bins <= 1 {
        return Ok(0.0);
    }

    let mut total_energy = 0.0;
    for &val in spectrum {
        total_energy += val.abs();
    }

    if total_energy <= 0.0 {
        return Ok(0.0);
    }

    let target = total_energy * valid_pct;
    let nyquist = valid_sr / 2.0;
    let freq_step = nyquist / (k_bins - 1) as f64;

    let mut accum = 0.0;
    for (k, &val) in spectrum.iter().enumerate() {
        accum += val.abs();
        if accum >= target {
            return Ok((k as f64) * freq_step);
        }
    }

    Ok(nyquist)
}

/// Compute zero-crossing rate of a signal.
pub fn zero_crossing_rate(samples: &[f64]) -> Result<f64, AnalysisError> {
    validate_samples(samples, "samples")?;
    let n = samples.len();
    if n <= 1 {
        return Ok(0.0);
    }

    let mut crossings = 0usize;
    for i in 1..n {
        let s_curr = samples[i];
        let s_prev = samples[i - 1];
        if (s_curr >= 0.0 && s_prev < 0.0) || (s_curr < 0.0 && s_prev >= 0.0) {
            crossings += 1;
        }
    }

    Ok((crossings as f64) / ((n - 1) as f64))
}

/// Compute zero-crossing rate across sliding frames.
pub fn zero_crossing_rate_framed(
    samples: &[f64],
    frame_length: usize,
    hop_length: usize,
) -> Result<Vec<f64>, AnalysisError> {
    validate_samples(samples, "samples")?;
    let maybe_frames = validate_framing(samples.len(), frame_length, hop_length)?;
    let num_frames = match maybe_frames {
        None => {
            let val = zero_crossing_rate(samples)?;
            return Ok(vec![val]);
        }
        Some(nf) => nf,
    };

    if frame_length <= 1 {
        return Ok(vec![0.0; num_frames]);
    }

    let denom = (frame_length - 1) as f64;
    let mut out = Vec::with_capacity(num_frames);
    for m in 0..num_frames {
        let (start, end) = checked_frame_bounds(m, hop_length, frame_length, samples.len())?;
        let frame = &samples[start..end];
        let mut crossings = 0usize;
        for i in 1..frame.len() {
            let s_curr = frame[i];
            let s_prev = frame[i - 1];
            if (s_curr >= 0.0 && s_prev < 0.0) || (s_curr < 0.0 && s_prev >= 0.0) {
                crossings += 1;
            }
        }
        out.push((crossings as f64) / denom);
    }

    Ok(out)
}

/// Compute root-mean-square (RMS) energy of a signal.
pub fn rms_energy(samples: &[f64]) -> Result<f64, AnalysisError> {
    validate_samples(samples, "samples")?;
    let n = samples.len();
    let mut sum_sq = 0.0;
    for &s in samples {
        sum_sq += s * s;
    }
    Ok((sum_sq / (n as f64)).sqrt())
}

/// Compute root-mean-square (RMS) energy across sliding frames.
pub fn rms_energy_framed(
    samples: &[f64],
    frame_length: usize,
    hop_length: usize,
) -> Result<Vec<f64>, AnalysisError> {
    validate_samples(samples, "samples")?;
    let maybe_frames = validate_framing(samples.len(), frame_length, hop_length)?;
    let num_frames = match maybe_frames {
        None => {
            let val = rms_energy(samples)?;
            return Ok(vec![val]);
        }
        Some(nf) => nf,
    };

    let denom = frame_length as f64;
    let mut out = Vec::with_capacity(num_frames);
    for m in 0..num_frames {
        let (start, end) = checked_frame_bounds(m, hop_length, frame_length, samples.len())?;
        let frame = &samples[start..end];
        let mut sum_sq = 0.0;
        for &s in frame {
            sum_sq += s * s;
        }
        out.push((sum_sq / denom).sqrt());
    }

    Ok(out)
}
