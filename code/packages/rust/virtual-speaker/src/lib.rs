//! # Virtual Speaker (MUS01 Stage 7)
//!
//! Gives the note-to-sound chain a named, reusable boundary where voltage-like
//! values become sound-pressure-like values:
//!
//! ```text
//! ... -> PCM audio -> DAC -> speaker
//! ```
//!
//! This is intentionally *not* a physics simulation. Two toy models exist:
//!
//! - [`LinearSpeakerSignal`]: `pressure(t) = gain * voltage(t)` (pass-through
//!   when `gain == 1.0`, which is [`DEFAULT_SPEAKER_GAIN`]).
//! - [`ClippingSpeakerSignal`]: the same, but hard-limited to `+-limit`, like a
//!   driver that cannot move beyond its excursion.
//!
//! [`render_speaker_output`] samples any speaker (or any analog signal) onto a
//! uniform grid, bounded by [`MAX_SAMPLES`] and [`MAX_SAMPLE_RATE_HZ`].
//!
//! ## Safety
//!
//! - Gains, limits and query times must be finite (CWE-20); `NaN`/`+-Inf` are
//!   rejected with [`OscillatorError::NonFiniteNumber`].
//! - A finite gain times a finite voltage can still overflow to infinity; such
//!   outputs are rejected rather than propagated.
//! - Sampling is bounded in rate, duration and sample count (CWE-400).

#![forbid(unsafe_code)]

use oscillator::{validate_finite, SampleBuffer, UniformSampler};

pub use oscillator::ContinuousSignal as AnalogSignal;
pub use oscillator::{
    ContinuousSignal, OscillatorError, MAX_DURATION_SECONDS, MAX_SAMPLES, MAX_SAMPLE_RATE_HZ,
};

/// Neutral speaker gain: output equals input.
pub const DEFAULT_SPEAKER_GAIN: f64 = 1.0;

fn finite_output(value: f64) -> Result<f64, OscillatorError> {
    validate_finite("speaker_output", value)
}

/// Toy speaker that scales an input signal into a pressure proxy.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearSpeakerSignal<S: ContinuousSignal> {
    analog_signal: S,
    speaker_gain: f64,
}

impl<S: ContinuousSignal> LinearSpeakerSignal<S> {
    /// Wrap `analog_signal` with an explicit finite `speaker_gain`.
    pub fn new(analog_signal: S, speaker_gain: f64) -> Result<Self, OscillatorError> {
        Ok(Self {
            analog_signal,
            speaker_gain: validate_finite("speaker_gain", speaker_gain)?,
        })
    }

    /// Wrap `analog_signal` with [`DEFAULT_SPEAKER_GAIN`].
    pub fn with_default_gain(analog_signal: S) -> Self {
        Self {
            analog_signal,
            speaker_gain: DEFAULT_SPEAKER_GAIN,
        }
    }

    /// Borrow the wrapped analog signal.
    pub fn analog_signal(&self) -> &S {
        &self.analog_signal
    }

    /// The configured gain.
    pub fn speaker_gain(&self) -> f64 {
        self.speaker_gain
    }
}

impl<S: ContinuousSignal> ContinuousSignal for LinearSpeakerSignal<S> {
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError> {
        let time = validate_finite("time_seconds", time_seconds)?;
        finite_output(self.speaker_gain * self.analog_signal.value_at(time)?)
    }
}

/// Speaker model that applies gain and then hard-clips to `[-limit, +limit]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippingSpeakerSignal<S: ContinuousSignal> {
    analog_signal: S,
    speaker_gain: f64,
    limit: f64,
}

impl<S: ContinuousSignal> ClippingSpeakerSignal<S> {
    /// Wrap `analog_signal` with a finite gain and a finite, strictly positive limit.
    pub fn new(analog_signal: S, speaker_gain: f64, limit: f64) -> Result<Self, OscillatorError> {
        let gain = validate_finite("speaker_gain", speaker_gain)?;
        let limit = validate_finite("limit", limit)?;
        if limit <= 0.0 {
            return Err(OscillatorError::InvalidParameter {
                name: "limit",
                reason: format!("must be > 0.0, got {limit}"),
            });
        }
        Ok(Self {
            analog_signal,
            speaker_gain: gain,
            limit,
        })
    }

    /// Borrow the wrapped analog signal.
    pub fn analog_signal(&self) -> &S {
        &self.analog_signal
    }

    /// The configured gain.
    pub fn speaker_gain(&self) -> f64 {
        self.speaker_gain
    }

    /// The configured clipping limit.
    pub fn limit(&self) -> f64 {
        self.limit
    }
}

impl<S: ContinuousSignal> ContinuousSignal for ClippingSpeakerSignal<S> {
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError> {
        let time = validate_finite("time_seconds", time_seconds)?;
        let raw = self.speaker_gain * self.analog_signal.value_at(time)?;
        if raw.is_nan() {
            return finite_output(raw);
        }
        // +-Inf from overflow clips to the limit, which is the physical behaviour.
        Ok(raw.clamp(-self.limit, self.limit))
    }
}

/// Sample `signal` (typically a speaker) on a uniform grid.
///
/// Rate, duration and sample count are bounded by [`MAX_SAMPLE_RATE_HZ`],
/// [`MAX_DURATION_SECONDS`] and [`MAX_SAMPLES`] before any allocation.
pub fn render_speaker_output(
    signal: &dyn ContinuousSignal,
    duration_seconds: f64,
    sample_rate_hz: f64,
    start_time_seconds: f64,
) -> Result<SampleBuffer, OscillatorError> {
    UniformSampler::new(sample_rate_hz)?.sample_with_start_time(
        signal,
        duration_seconds,
        start_time_seconds,
    )
}
