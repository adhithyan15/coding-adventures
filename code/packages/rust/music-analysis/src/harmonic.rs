//! Harmonic profile and ADSR timbre estimation (MUS08, MUS05, VER15).

use musical_instruments::{
    AdsrEnvelope, HarmonicPartial, InstrumentProfile, VariationProfile,
};

use crate::dsp_utils::{build_hann_window, rfft_magnitudes};
use crate::features::{rms_energy, rms_energy_framed};
use crate::pitch::{hps_pitch, yin_pitch};
use crate::types::{
    validate_positive_float, validate_samples, AnalysisError, MAX_FFT_LEN, MAX_PARTIALS,
    MAX_SAMPLE_RATE_HZ,
};

/// Extract fundamental, harmonic partial amplitudes, and ADSR envelope from single-note audio.
///
/// Returns a MUS05 `InstrumentProfile` suitable for additive re-synthesis in `musical-instruments`
/// and portable score playback in `music-machine-core`.
///
/// # Arguments
/// * `samples` - Time-domain audio sample buffer of a single note.
/// * `sample_rate_hz` - Sampling frequency in Hertz.
/// * `fundamental_hz` - Optional known fundamental frequency (Hz). If `None`, estimated.
/// * `max_partials` - Maximum number of harmonic partials to extract (up to 64).
/// * `instrument_id` - Identifier string for the resulting instrument profile.
/// * `display_name` - Human-readable display label.
pub fn extract_harmonic_profile(
    samples: &[f64],
    sample_rate_hz: f64,
    fundamental_hz: Option<f64>,
    max_partials: usize,
    instrument_id: &str,
    display_name: &str,
) -> Result<InstrumentProfile, AnalysisError> {
    let valid_sr = validate_positive_float(
        sample_rate_hz,
        "sample_rate_hz",
        Some(MAX_SAMPLE_RATE_HZ),
    )?;
    if max_partials == 0 {
        return Err(AnalysisError::InvalidInput(
            "max_partials must be >= 1".to_string(),
        ));
    }
    if max_partials > MAX_PARTIALS {
        return Err(AnalysisError::BudgetExceeded(format!(
            "max_partials {max_partials} exceeds MAX_PARTIALS={MAX_PARTIALS}"
        )));
    }
    validate_samples(samples, "samples")?;
    let n = samples.len();

    let overall_rms = rms_energy(samples)?;
    if overall_rms < 1e-4 {
        return Err(AnalysisError::InvalidInput(
            "signal is silent, cannot extract harmonic profile".to_string(),
        ));
    }

    // 1. Determine fundamental frequency
    let f0 = match fundamental_hz {
        Some(hz) => validate_positive_float(hz, "fundamental_hz", Some(valid_sr / 2.0))?,
        None => {
            let mut pitch = yin_pitch(samples, valid_sr, 40.0, 2000.0, 0.15)?;
            if pitch <= 0.0 {
                pitch = hps_pitch(samples, valid_sr, 5, 40.0, 2000.0, None)?;
            }
            if pitch <= 0.0 {
                return Err(AnalysisError::InvalidInput(
                    "failed to detect fundamental pitch in signal".to_string(),
                ));
            }
            pitch
        }
    };

    // 2. Extract harmonic partials from sustained portion
    let target_fft = 4096usize;
    let (analysis_slice, fft_len) = if n > target_fft {
        let start = (n - target_fft) / 2;
        (samples[start..start + target_fft].to_vec(), target_fft)
    } else {
        let p2 = if n <= 1 { 1 } else { n.next_power_of_two() };
        let fl = 512.max(p2).min(MAX_FFT_LEN);
        let mut padded = vec![0.0f64; fl];
        padded[..n].copy_from_slice(samples);
        (padded, fl)
    };

    let win = build_hann_window(fft_len);
    let mut windowed = vec![0.0f64; fft_len];
    for i in 0..fft_len {
        windowed[i] = analysis_slice[i] * win[i];
    }

    let mags = rfft_magnitudes(&windowed)?;
    let k_bins = mags.len();

    let freq_step = valid_sr / (fft_len as f64);

    // Find fundamental amplitude
    let k1 = (f0 / freq_step).round() as usize;
    let search_w = 4usize;
    let k1_low = k1.saturating_sub(search_w);
    let k1_high_bound = k1
        .checked_add(search_w)
        .and_then(|w| w.checked_add(1))
        .unwrap_or(k_bins);
    let k1_high = k_bins.min(k1_high_bound);
    let mut a1 = 0.0f64;
    if k1_low < k1_high {
        for &mag in &mags[k1_low..k1_high] {
            if mag > a1 {
                a1 = mag;
            }
        }
    }
    if a1 <= 0.0 {
        a1 = 1.0;
    }

    let mut partials = Vec::new();
    for h in 1..=max_partials {
        let fh = (h as f64) * f0;
        if fh >= valid_sr / 2.0 {
            break;
        }
        let kh = (fh / freq_step).round() as usize;
        let kh_low = kh.saturating_sub(search_w);
        let kh_high_bound = kh
            .checked_add(search_w)
            .and_then(|w| w.checked_add(1))
            .unwrap_or(k_bins);
        let kh_high = k_bins.min(kh_high_bound);
        let rel_amp = if kh_low < kh_high {
            let mut ah = 0.0f64;
            for &mag in &mags[kh_low..kh_high] {
                if mag > ah {
                    ah = mag;
                }
            }
            (ah / a1).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let rounded_amp = (rel_amp * 10000.0).round() / 10000.0;
        let partial = HarmonicPartial::new(h as f64, rounded_amp)
            .map_err(|e| AnalysisError::InvalidInput(format!("Invalid partial: {e}")))?;
        partials.push(partial);
    }

    if partials.is_empty() {
        partials.push(
            HarmonicPartial::new(1.0, 1.0)
                .map_err(|e| AnalysisError::InvalidInput(format!("Invalid partial: {e}")))?,
        );
    }

    // 3. Estimate ADSR Envelope
    let frame_len = 256usize;
    let hop_len = 128usize;
    let rms_track = if n >= frame_len {
        rms_energy_framed(samples, frame_len, hop_len)?
    } else {
        vec![overall_rms]
    };

    let mut max_rms = 0.0f64;
    let mut peak_idx = 0usize;
    for (idx, &r) in rms_track.iter().enumerate() {
        if r > max_rms {
            max_rms = r;
            peak_idx = idx;
        }
    }
    if max_rms <= 0.0 {
        max_rms = 1.0;
    }

    let total_dur_sec = (n as f64) / valid_sr;
    let attack_sec = 0.005f64.max(
        (total_dur_sec * 0.4).min(((peak_idx * hop_len) as f64) / valid_sr),
    );

    // Release detection: when energy drops below 15% of max near end
    let mut rel_frame = rms_track.len().saturating_sub(1);
    while rel_frame > peak_idx && rms_track[rel_frame] < 0.15 * max_rms {
        rel_frame -= 1;
    }

    let release_sec = 0.005f64.max(
        (total_dur_sec * 0.4)
            .min((((rms_track.len().saturating_sub(1) - rel_frame) * hop_len) as f64) / valid_sr),
    );

    // Sustain level: average between peak and release
    let sustain_start = (peak_idx + 1).min(rms_track.len().saturating_sub(1));
    let sustain_end = sustain_start.max(rel_frame);
    let sustain_level = if sustain_start < sustain_end {
        let slice = &rms_track[sustain_start..sustain_end];
        let avg = slice.iter().sum::<f64>() / (slice.len() as f64);
        (avg / max_rms).clamp(0.05, 1.0)
    } else {
        0.7
    };

    let decay_sec = 0.005f64.max((total_dur_sec * 0.3).min(attack_sec * 0.5));

    let envelope = AdsrEnvelope::new(
        (attack_sec.min(60.0) * 10000.0).round() / 10000.0,
        (decay_sec.min(60.0) * 10000.0).round() / 10000.0,
        (sustain_level * 1000.0).round() / 1000.0,
        (release_sec.min(60.0) * 10000.0).round() / 10000.0,
    )
    .map_err(|e| AnalysisError::InvalidInput(format!("Envelope error: {e}")))?;

    let mut peak_amp = 0.0f64;
    for &s in samples {
        let a = s.abs();
        if a > peak_amp {
            peak_amp = a;
        }
    }
    let gain = (peak_amp.clamp(0.05, 1.0) * 1000.0).round() / 1000.0;

    InstrumentProfile::new(
        instrument_id,
        display_name,
        "additive",
        gain,
        partials,
        envelope,
        VariationProfile::default(),
        None,
        "custom",
    )
    .map_err(|e| AnalysisError::InvalidInput(format!("InstrumentProfile error: {e}")))
}

/// Convenience wrapper for [`extract_harmonic_profile`] with standard default labels.
pub fn extract_harmonic_profile_simple(
    samples: &[f64],
    sample_rate_hz: f64,
) -> Result<InstrumentProfile, AnalysisError> {
    extract_harmonic_profile(
        samples,
        sample_rate_hz,
        None,
        16,
        "extracted_instrument",
        "Extracted Instrument",
    )
}
