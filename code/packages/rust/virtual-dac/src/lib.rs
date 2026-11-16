//! # Virtual DAC (MUS01 Stage 6)
//!
//! Turns the discrete PCM integers of a [`PcmAudio`] buffer back into a
//! *virtual continuous* voltage signal that can be queried at any time `t`:
//!
//! ```text
//! typed note -> frequency -> oscillator -> sampler -> PCM audio -> DAC -> speaker
//! ```
//!
//! Two reconstruction models are provided, both implementing
//! [`oscillator::ContinuousSignal`] (re-exported as [`AnalogSignal`]):
//!
//! - [`ZeroOrderHoldDACSignal`]: each PCM integer becomes a voltage that is
//!   held until the next sample time (a staircase).
//! - [`LinearInterpolationDACSignal`]: neighbouring PCM voltages are joined by
//!   straight lines.
//!
//! Outside the half-open buffer interval `[start, start + duration)` both
//! models output `0.0`.
//!
//! ## Safety
//!
//! - Non-finite query times (`NaN`, `+-Inf`) are rejected with
//!   [`OscillatorError::NonFiniteNumber`] (CWE-20).
//! - The sample index derived from floating-point time is clamped with
//!   `min(max(idx, 0), count - 1)` ([`clamp_sample_index`]) so float rounding
//!   at the right edge of the buffer can never index out of bounds.
//! - No `unsafe`, no allocation, no I/O.

#![forbid(unsafe_code)]

use oscillator::{validate_finite, OscillatorError};
use pcm_audio::{PcmAudio, PcmFormat, PCM16_MAX, PCM16_MIN};

pub use oscillator::ContinuousSignal as AnalogSignal;
pub use oscillator::{ContinuousSignal, MAX_SAMPLES, MAX_SAMPLE_RATE_HZ};

/// Map a signed 16-bit PCM integer to a virtual DAC voltage.
///
/// Positive codes are scaled by `PCM16_MAX` and negative codes by
/// `|PCM16_MIN|`, so `32767 -> +full_scale` and `-32768 -> -full_scale`.
pub fn pcm16_to_voltage(sample: i16, format: &PcmFormat) -> f64 {
    let full_scale = format.full_scale_voltage();
    if sample >= 0 {
        f64::from(sample) / f64::from(PCM16_MAX) * full_scale
    } else {
        f64::from(sample) / f64::from(PCM16_MIN).abs() * full_scale
    }
}

/// Clamp a (possibly out-of-range, possibly NaN) floating-point sample index
/// into `0..count` using `min(max(idx, 0), count - 1)`.
///
/// Returns `0` for an empty buffer (callers must check `count == 0` first).
pub fn clamp_sample_index(index: f64, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let last = count - 1;
    // `!(index > 0.0)` deliberately also catches NaN.
    if index.is_nan() || index <= 0.0 {
        0
    } else if index >= last as f64 {
        last
    } else {
        index as usize
    }
}

/// Compute the fractional sample position of `time_seconds` within `audio`,
/// or `None` when the time lies outside `[start, end)` or the buffer is empty.
fn position_in_buffer(audio: &PcmAudio, time_seconds: f64) -> Result<Option<f64>, OscillatorError> {
    let time = validate_finite("time_seconds", time_seconds)?;
    let count = audio.sample_count();
    if count == 0 {
        return Ok(None);
    }
    let start = audio.start_time_seconds();
    let end = start + audio.duration_seconds();
    if time < start || time >= end {
        return Ok(None);
    }
    let position = (time - start) * audio.format().sample_rate_hz();
    if !position.is_finite() {
        return Ok(None);
    }
    Ok(Some(position))
}

/// Virtual DAC output using a zero-order-hold staircase.
#[derive(Debug, Clone, PartialEq)]
pub struct ZeroOrderHoldDACSignal {
    pcm_audio: PcmAudio,
}

impl ZeroOrderHoldDACSignal {
    /// Wrap a PCM buffer as a zero-order-hold DAC signal.
    pub fn new(pcm_audio: PcmAudio) -> Self {
        Self { pcm_audio }
    }

    /// Borrow the underlying PCM buffer.
    pub fn pcm_audio(&self) -> &PcmAudio {
        &self.pcm_audio
    }

    /// Consume the signal and return its PCM buffer.
    pub fn into_pcm_audio(self) -> PcmAudio {
        self.pcm_audio
    }
}

impl ContinuousSignal for ZeroOrderHoldDACSignal {
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError> {
        let Some(position) = position_in_buffer(&self.pcm_audio, time_seconds)? else {
            return Ok(0.0);
        };
        let index = clamp_sample_index(position.floor(), self.pcm_audio.sample_count());
        Ok(pcm16_to_voltage(
            self.pcm_audio.samples()[index],
            self.pcm_audio.format(),
        ))
    }
}

/// Virtual DAC output that linearly interpolates between adjacent samples.
///
/// After the final sample the last voltage is held until the end of the buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearInterpolationDACSignal {
    pcm_audio: PcmAudio,
}

impl LinearInterpolationDACSignal {
    /// Wrap a PCM buffer as a linearly interpolating DAC signal.
    pub fn new(pcm_audio: PcmAudio) -> Self {
        Self { pcm_audio }
    }

    /// Borrow the underlying PCM buffer.
    pub fn pcm_audio(&self) -> &PcmAudio {
        &self.pcm_audio
    }

    /// Consume the signal and return its PCM buffer.
    pub fn into_pcm_audio(self) -> PcmAudio {
        self.pcm_audio
    }
}

impl ContinuousSignal for LinearInterpolationDACSignal {
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError> {
        let Some(position) = position_in_buffer(&self.pcm_audio, time_seconds)? else {
            return Ok(0.0);
        };
        let count = self.pcm_audio.sample_count();
        let floor = position.floor();
        let lower = clamp_sample_index(floor, count);
        let upper = clamp_sample_index(floor + 1.0, count);
        let format = self.pcm_audio.format();
        let samples = self.pcm_audio.samples();
        let v0 = pcm16_to_voltage(samples[lower], format);
        let v1 = pcm16_to_voltage(samples[upper], format);
        let fraction = (position - floor).clamp(0.0, 1.0);
        Ok(v0 + fraction * (v1 - v0))
    }
}
