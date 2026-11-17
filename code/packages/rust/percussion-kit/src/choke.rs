//! Hi-hat choke group dynamics and linear release ramp attenuation (MUS07 §7).

use crate::error::{
    safe_round_sample, validate_duration, validate_non_negative, validate_sample_rate,
    PercussionError, MAX_SAMPLE_COUNT,
};
use crate::profile::PercussionVoiceProfile;

/// Apply a linear release ramp attenuation to a hi-hat sample buffer (MUS07 §7).
///
/// When a closed or pedal hi-hat hit occurs while an open hi-hat is actively
/// ringing, the open hi-hat must be attenuated smoothly using a 15 ms linear
/// release ramp starting at the choke time.
pub fn apply_hihat_choke(
    samples: &[f64],
    hit_start_seconds: f64,
    choke_start_seconds: f64,
    sample_rate_hz: f64,
    choke_ramp_seconds: f64,
) -> Result<Vec<f64>, PercussionError> {
    if samples.len() > MAX_SAMPLE_COUNT {
        return Err(PercussionError::SampleCountTooLarge {
            count: samples.len(),
            max: MAX_SAMPLE_COUNT,
        });
    }

    let hit_start = validate_non_negative("hit_start_seconds", hit_start_seconds)?;
    let choke_start = validate_non_negative("choke_start_seconds", choke_start_seconds)?;
    let rate = validate_sample_rate("sample_rate_hz", sample_rate_hz)?;
    let ramp_dur = validate_duration("choke_ramp_seconds", choke_ramp_seconds)?;

    let total_len = samples.len();
    if total_len == 0 {
        return Ok(Vec::new());
    }

    let mut output = samples.to_vec();

    if choke_start <= hit_start {
        let ramp_samples = safe_round_sample(ramp_dur, rate)?;
        if ramp_samples == 0 {
            output.fill(0.0);
            return Ok(output);
        }
        for (i, item) in output.iter_mut().enumerate().take(total_len.min(ramp_samples)) {
            *item *= (1.0 - i as f64 / ramp_samples as f64).max(0.0);
        }
        for item in output.iter_mut().skip(ramp_samples) {
            *item = 0.0;
        }
        return Ok(output);
    }

    let time_delta = choke_start - hit_start;
    let choke_idx = safe_round_sample(time_delta, rate)?;
    if choke_idx >= total_len {
        return Ok(output);
    }

    let ramp_samples = safe_round_sample(ramp_dur, rate)?;
    if ramp_samples == 0 {
        for item in output.iter_mut().skip(choke_idx) {
            *item = 0.0;
        }
        return Ok(output);
    }

    let ramp_end = total_len.min(choke_idx + ramp_samples);
    for (step, item) in output[choke_idx..ramp_end].iter_mut().enumerate() {
        let fraction = step as f64 / ramp_samples as f64;
        *item *= (1.0 - fraction).clamp(0.0, 1.0);
    }

    for item in output.iter_mut().skip(ramp_end) {
        *item = 0.0;
    }

    Ok(output)
}

/// Determine whether a later percussion hit chokes an active earlier hit (MUS07 §7).
pub fn is_choked_by(
    earlier_voice: &PercussionVoiceProfile,
    earlier_hit_id: &str,
    later_voice: &PercussionVoiceProfile,
    later_hit_id: &str,
) -> bool {
    let (Some(ref earlier_group), Some(ref later_group)) =
        (&earlier_voice.choke_group, &later_voice.choke_group)
    else {
        return false;
    };

    if earlier_group != later_group {
        return false;
    }

    if earlier_group == "hihat" {
        // MUS07 §7: closed/pedal hihat or open hihat chokes active open hihat
        let is_earlier_open = earlier_hit_id == "open_hihat"
            || earlier_voice.id == "open_hihat_naive"
            || earlier_voice.id.to_ascii_lowercase().contains("open");

        let is_later_choker = matches!(later_hit_id, "closed_hihat" | "pedal_hihat" | "open_hihat")
            || matches!(
                later_voice.id.as_str(),
                "closed_hihat_naive" | "pedal_hihat_naive" | "open_hihat_naive"
            )
            || later_voice.id.to_ascii_lowercase().contains("closed")
            || later_voice.id.to_ascii_lowercase().contains("pedal")
            || later_voice.id.to_ascii_lowercase().contains("open");

        return is_earlier_open && is_later_choker;
    }

    true
}
