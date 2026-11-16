//! Continuous signal generator for individual percussion hits (MUS07 §4, §9).

use oscillator::{
    sample_count_for_duration, ContinuousSignal, OscillatorError, SampleBuffer, UniformSampler,
};

use crate::error::{
    safe_round_sample, validate_duration, validate_sample_rate, validate_unit_float,
    PercussionError, DEFAULT_SAMPLE_RATE_HZ, MAX_DURATION_SECONDS, MAX_SAMPLE_COUNT,
};
use crate::profile::PercussionVoiceProfile;

/// Continuous signal generator for one unpitched percussion hit.
#[derive(Debug, Clone, PartialEq)]
pub struct PercussionSignal {
    pub voice: PercussionVoiceProfile,
    pub hit_duration_seconds: f64,
    pub amplitude: f64,
    pub sample_rate_hz: f64,
}

impl PercussionSignal {
    /// Construct a new percussion hit continuous signal with validated boundaries.
    pub fn new(
        voice: PercussionVoiceProfile,
        hit_duration_seconds: f64,
        amplitude: f64,
        sample_rate_hz: f64,
    ) -> Result<Self, PercussionError> {
        let dur = validate_duration("hit_duration_seconds", hit_duration_seconds)?;
        let amp = validate_unit_float("amplitude", amplitude)?;
        let rate = validate_sample_rate("sample_rate_hz", sample_rate_hz)?;

        Ok(Self {
            voice,
            hit_duration_seconds: dur,
            amplitude: amp,
            sample_rate_hz: rate,
        })
    }

    /// Construct a signal with standard default CD sample rate (44.1 kHz) and unit amplitude.
    pub fn standard(
        voice: PercussionVoiceProfile,
        hit_duration_seconds: f64,
    ) -> Result<Self, PercussionError> {
        Self::new(voice, hit_duration_seconds, 1.0, DEFAULT_SAMPLE_RATE_HZ)
    }

    /// Return the hit duration in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.hit_duration_seconds
    }

    /// Sample this continuous signal onto a uniform time grid for a given duration.
    pub fn sample(
        &self,
        duration_seconds: f64,
        max_sample_count: usize,
    ) -> Result<SampleBuffer, PercussionError> {
        let dur = validate_duration("duration_seconds", duration_seconds)?;
        if dur > MAX_DURATION_SECONDS * 2.0 {
            return Err(PercussionError::DurationTooLong {
                duration: dur,
                max: MAX_DURATION_SECONDS * 2.0,
            });
        }
        if max_sample_count > MAX_SAMPLE_COUNT {
            return Err(PercussionError::SampleCountTooLarge {
                count: max_sample_count,
                max: MAX_SAMPLE_COUNT,
            });
        }

        let needed_count = sample_count_for_duration(dur, self.sample_rate_hz)?;
        if needed_count > max_sample_count {
            return Err(PercussionError::SampleCountTooLarge {
                count: needed_count,
                max: max_sample_count,
            });
        }

        let sampler = UniformSampler::new(self.sample_rate_hz)?;
        let buffer = sampler.sample(self, dur)?;
        Ok(buffer)
    }
}

impl ContinuousSignal for PercussionSignal {
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError> {
        if !time_seconds.is_finite() {
            return Err(OscillatorError::NonFiniteNumber {
                name: "time_seconds",
                value: time_seconds,
            });
        }
        if time_seconds < 0.0 {
            return Err(OscillatorError::NegativeNumber {
                name: "time_seconds",
                value: time_seconds,
            });
        }

        let envelope = self
            .voice
            .envelope
            .value_at(time_seconds, self.hit_duration_seconds)
            .map_err(|e| OscillatorError::InvalidParameter {
                name: "envelope",
                reason: e.to_string(),
            })?;

        if envelope <= 0.0 {
            return Ok(0.0);
        }

        let sample_index = safe_round_sample(time_seconds, self.sample_rate_hz).map_err(|e| {
            OscillatorError::InvalidParameter {
                name: "time_seconds",
                reason: e.to_string(),
            }
        })?;

        let mut raw = 0.0;
        if self.voice.click_amplitude > 0.0 && time_seconds < 0.002 {
            raw += self.voice.click_amplitude * (1.0 - time_seconds / 0.002);
        }

        for mode in &self.voice.modes {
            raw += mode.amplitude
                * (-time_seconds / mode.decay_seconds).exp()
                * trig::sin(2.0 * trig::PI * (mode.frequency_hz * time_seconds + mode.phase_cycles));
        }

        if let Some(ref noise) = self.voice.noise_profile {
            raw += noise.amplitude
                * (-time_seconds / noise.decay_seconds).exp()
                * noise.sample_noise(sample_index);
        }

        let norm = self.voice.amplitude_normalizer();
        let sample_val = self.amplitude * self.voice.gain * envelope * raw / norm;
        if !sample_val.is_finite() {
            return Ok(0.0);
        }

        Ok(sample_val)
    }
}
