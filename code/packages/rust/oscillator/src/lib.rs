//! # Oscillator and Sampler (OSC00)
//!
//! An oscillator is anything that repeats over time.
//!
//! In this crate, an oscillator is not a sound card, timer, DAC, or circuit. It
//! is a pure mathematical signal: given an exact time in seconds, it answers
//! what value the ideal signal would have at that instant:
//!
//! ```text
//! x(t) -> value
//! ```
//!
//! A sampler sits one layer above the oscillator. It chooses concrete, evenly
//! spaced moments in time and evaluates the continuous signal onto a uniform
//! time grid:
//!
//! ```text
//! t_n = start_time_seconds + n / sample_rate_hz
//! ```
//!
//! Storing these measurements in a [`SampleBuffer`] creates the digital bridge
//! from continuous mathematical models to discrete buffers consumed by later
//! audio codecs (PCM, WAV), DACs, digital clocks, or synthesizers.
//!
//! ## Mathematical Models of Supported Waveforms
//!
//! 1. **Sine Oscillator**:
//!    $$x(t) = \text{offset} + A \sin(2\pi(f \cdot t + \phi))$$
//!    Smooth harmonic wave, fundamental building block of audio tones and Fourier synthesis.
//!
//! 2. **Square Oscillator**:
//!    Alternates between `high` and `low` levels with a configurable `duty_cycle`:
//!    $$\text{pos} = \text{fractional\_part}(f \cdot t + \phi)$$
//!    $$x(t) = \begin{cases} \text{high} & \text{if pos} < \text{duty\_cycle} \\ \text{low} & \text{otherwise} \end{cases}$$
//!
//! 3. **Pulse Oscillator**:
//!    Rectangular pulse wave parameterized by `duty_cycle` and peak amplitude $A$:
//!    $$x(t) = \begin{cases} \text{offset} + A & \text{if pos} < \text{duty\_cycle} \\ \text{offset} - A & \text{otherwise} \end{cases}$$
//!
//! 4. **Sawtooth Oscillator**:
//!    Linear rising ramp from $-1.0$ to $+1.0$ over each cycle:
//!    $$x(t) = \text{offset} + A \cdot (2 \cdot \text{pos} - 1.0)$$
//!    Contains all integer harmonics (both odd and even), fundamental to subtractive synthesis.
//!
//! 5. **Triangle Oscillator**:
//!    Linear symmetric triangular wave in phase with $\sin(2\pi t)$:
//!    - Rising from $0.0$ to $+1.0$ on $[0.0, 0.25)$: $4 \cdot \text{pos}$
//!    - Falling from $+1.0$ to $-1.0$ on $[0.25, 0.75)$: $2.0 - 4 \cdot \text{pos}$
//!    - Rising from $-1.0$ to $0.0$ on $[0.75, 1.0)$: $4 \cdot \text{pos} - 4.0$
//!
//!    Contains only odd harmonics decaying rapidly as $1/n^2$.
//!
//! ## Verification & Security Guarantees
//!
//! - **Safety**: `#![forbid(unsafe_code)]`, pure `std` dependencies.
//! - **CWE-400 (Resource Exhaustion)**:
//!   - `MAX_SAMPLE_RATE_HZ = 192_000.0`
//!   - `MAX_DURATION_SECONDS = 60.0`
//!   - `MAX_SAMPLES = 16_777_216` (2^24)
//!   - Allocation sizing is verified **strictly before** any allocation.
//! - **CWE-190 (Integer Overflow)**: Checked arithmetic on sample counts and buffer sizes.
//! - **CWE-20 (Improper Input Validation)**: All inputs (`frequency_hz`, `amplitude`, `time`, etc.)
//!   are checked for finiteness. `NaN` and `±Inf` are rejected fail-closed with [`OscillatorError`].

#![forbid(unsafe_code)]

use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

/// Maximum allowable sample rate in Hertz (CWE-400 resource consumption guard).
pub const MAX_SAMPLE_RATE_HZ: f64 = 192_000.0;

/// Maximum allowable duration for a single sampling operation in seconds (CWE-400 guard).
pub const MAX_DURATION_SECONDS: f64 = 60.0;

/// Maximum allowable number of samples in a single buffer (2^24 samples, ~128 MiB at f64).
pub const MAX_SAMPLES: usize = 16_777_216;

/// Epsilon tolerance for identifying integer sample count boundaries.
pub const INTEGER_TOLERANCE: f64 = 1e-9;

const TWO_PI: f64 = 2.0 * PI;

/// Structured error type for oscillator and sampler validation failures.
#[derive(Debug, Clone, PartialEq)]
pub enum OscillatorError {
    /// A parameter failed general validation rules.
    InvalidParameter {
        name: &'static str,
        reason: String,
    },
    /// A float value was not finite (NaN, +Inf, -Inf) (CWE-20).
    NonFiniteNumber {
        name: &'static str,
        value: f64,
    },
    /// A parameter expected to be non-negative was negative.
    NegativeNumber {
        name: &'static str,
        value: f64,
    },
    /// Requested sample rate exceeds [`MAX_SAMPLE_RATE_HZ`] (CWE-400).
    SampleRateTooHigh {
        rate: f64,
        max: f64,
    },
    /// Requested duration exceeds [`MAX_DURATION_SECONDS`] (CWE-400).
    DurationTooLong {
        duration: f64,
        max: f64,
    },
    /// Requested sample count exceeds [`MAX_SAMPLES`] (CWE-400).
    SampleCountTooLarge {
        count: usize,
        max: usize,
    },
    /// An index was outside the valid range [0, count).
    IndexOutOfBounds {
        index: usize,
        count: usize,
    },
}

impl fmt::Display for OscillatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter { name, reason } => {
                write!(f, "invalid parameter '{name}': {reason}")
            }
            Self::NonFiniteNumber { name, value } => {
                write!(f, "parameter '{name}' must be finite, got {value}")
            }
            Self::NegativeNumber { name, value } => {
                write!(f, "parameter '{name}' must be non-negative, got {value}")
            }
            Self::SampleRateTooHigh { rate, max } => {
                write!(f, "sample rate {rate} Hz exceeds maximum {max} Hz")
            }
            Self::DurationTooLong { duration, max } => {
                write!(f, "duration {duration} s exceeds maximum {max} s")
            }
            Self::SampleCountTooLarge { count, max } => {
                write!(f, "sample count {count} exceeds maximum allowable {max}")
            }
            Self::IndexOutOfBounds { index, count } => {
                write!(f, "index {index} out of bounds for buffer of size {count}")
            }
        }
    }
}

impl Error for OscillatorError {}

// --- Numeric validation helpers ---

/// Validate that a floating-point value is finite.
#[inline]
pub fn validate_finite(name: &'static str, value: f64) -> Result<f64, OscillatorError> {
    if !value.is_finite() {
        return Err(OscillatorError::NonFiniteNumber { name, value });
    }
    Ok(value)
}

/// Validate that a floating-point value is finite and >= 0.0.
#[inline]
pub fn validate_non_negative(name: &'static str, value: f64) -> Result<f64, OscillatorError> {
    let v = validate_finite(name, value)?;
    if v < 0.0 {
        return Err(OscillatorError::NegativeNumber { name, value: v });
    }
    Ok(v)
}

/// Validate that a floating-point value is finite, > 0.0, and within allowable limits.
#[inline]
pub fn validate_sample_rate(name: &'static str, value: f64) -> Result<f64, OscillatorError> {
    let v = validate_finite(name, value)?;
    if v <= 0.0 {
        return Err(OscillatorError::InvalidParameter {
            name,
            reason: format!("must be > 0.0, got {v}"),
        });
    }
    if v > MAX_SAMPLE_RATE_HZ {
        return Err(OscillatorError::SampleRateTooHigh {
            rate: v,
            max: MAX_SAMPLE_RATE_HZ,
        });
    }
    Ok(v)
}

/// Return the fractional part of a real number in the half-open interval `[0.0, 1.0)`.
///
/// Direct implementation of `x - floor(x)` as specified in OSC00 §8, which correctly
/// handles negative numbers:
/// - `fractional_part(0.25) == 0.25`
/// - `fractional_part(-0.25) == 0.75`
#[inline]
pub fn fractional_part(value: f64) -> f64 {
    value - value.floor()
}

/// Return half the sample rate: the highest cleanly representable frequency (Nyquist limit).
#[inline]
pub fn nyquist_frequency(sample_rate_hz: f64) -> Result<f64, OscillatorError> {
    let rate = validate_sample_rate("sample_rate_hz", sample_rate_hz)?;
    Ok(rate / 2.0)
}

/// Calculate the number of samples for a given duration and sample rate.
///
/// Uses an integer tolerance check (`1e-9`) to avoid floating-point off-by-one errors
/// where e.g. `0.01 * 48000.0 = 479.99999999999994` would truncate to 479 instead of 480.
pub fn sample_count_for_duration(
    duration_seconds: f64,
    sample_rate_hz: f64,
) -> Result<usize, OscillatorError> {
    let duration = validate_non_negative("duration_seconds", duration_seconds)?;
    if duration > MAX_DURATION_SECONDS {
        return Err(OscillatorError::DurationTooLong {
            duration,
            max: MAX_DURATION_SECONDS,
        });
    }
    let sample_rate = validate_sample_rate("sample_rate_hz", sample_rate_hz)?;

    let raw_count = duration * sample_rate;
    if !raw_count.is_finite() {
        return Err(OscillatorError::NonFiniteNumber {
            name: "sample_count",
            value: raw_count,
        });
    }

    let nearest = raw_count.round();
    let count_f64 = if (raw_count - nearest).abs() <= INTEGER_TOLERANCE {
        nearest
    } else {
        raw_count.floor()
    };

    if count_f64 < 0.0 {
        return Err(OscillatorError::NegativeNumber {
            name: "sample_count",
            value: count_f64,
        });
    }

    if count_f64 > MAX_SAMPLES as f64 {
        return Err(OscillatorError::SampleCountTooLarge {
            count: count_f64 as usize,
            max: MAX_SAMPLES,
        });
    }

    Ok(count_f64 as usize)
}

/// Return the time in seconds corresponding to sample index `index` on a uniform grid.
#[inline]
pub fn time_at_sample(
    index: usize,
    sample_rate_hz: f64,
    start_time_seconds: f64,
) -> Result<f64, OscillatorError> {
    let sample_rate = validate_sample_rate("sample_rate_hz", sample_rate_hz)?;
    let start_time = validate_finite("start_time_seconds", start_time_seconds)?;
    Ok(start_time + (index as f64) / sample_rate)
}

// --- Continuous Signal Trait ---

/// Trait implemented by any virtual continuous signal $x(t)$.
///
/// An implementor is a pure function of continuous time: given a timestamp in
/// seconds, it yields the instantaneous amplitude without side-effects or state mutation.
pub trait ContinuousSignal {
    /// Return the signal amplitude at `time_seconds`.
    ///
    /// Rejects non-finite times (`NaN`, `±Inf`) with [`OscillatorError::NonFiniteNumber`].
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError>;
}

// --- Waveforms ---

/// Smooth sine oscillator:
/// $$x(t) = \text{offset} + \text{amplitude} \cdot \sin(2\pi(\text{frequency\_hz} \cdot t + \text{phase\_cycles}))$$
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SineOscillator {
    frequency_hz: f64,
    amplitude: f64,
    phase_cycles: f64,
    offset: f64,
}

impl SineOscillator {
    /// Construct a new sine oscillator with default `amplitude = 1.0`, `phase_cycles = 0.0`, `offset = 0.0`.
    pub fn new(frequency_hz: f64) -> Result<Self, OscillatorError> {
        Self::with_options(frequency_hz, 1.0, 0.0, 0.0)
    }

    /// Construct a new sine oscillator with custom amplitude, phase, and offset.
    pub fn with_options(
        frequency_hz: f64,
        amplitude: f64,
        phase_cycles: f64,
        offset: f64,
    ) -> Result<Self, OscillatorError> {
        let freq = validate_non_negative("frequency_hz", frequency_hz)?;
        let amp = validate_non_negative("amplitude", amplitude)?;
        let phase = validate_finite("phase_cycles", phase_cycles)?;
        let off = validate_finite("offset", offset)?;

        Ok(Self {
            frequency_hz: freq,
            amplitude: amp,
            phase_cycles: phase,
            offset: off,
        })
    }

    pub fn frequency_hz(&self) -> f64 {
        self.frequency_hz
    }

    pub fn amplitude(&self) -> f64 {
        self.amplitude
    }

    pub fn phase_cycles(&self) -> f64 {
        self.phase_cycles
    }

    pub fn offset(&self) -> f64 {
        self.offset
    }
}

impl ContinuousSignal for SineOscillator {
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError> {
        let t = validate_finite("time_seconds", time_seconds)?;
        let phase = self.frequency_hz * t + self.phase_cycles;
        Ok(self.offset + self.amplitude * (TWO_PI * phase).sin())
    }
}

/// Square oscillator switching between two levels with configurable duty cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SquareOscillator {
    frequency_hz: f64,
    low: f64,
    high: f64,
    duty_cycle: f64,
    phase_cycles: f64,
}

impl SquareOscillator {
    /// Construct a standard bipolar square oscillator (`low = -1.0`, `high = 1.0`, `duty_cycle = 0.5`).
    pub fn new(frequency_hz: f64) -> Result<Self, OscillatorError> {
        Self::with_options(frequency_hz, -1.0, 1.0, 0.5, 0.0)
    }

    /// Construct a square oscillator with custom levels, duty cycle, and phase.
    pub fn with_options(
        frequency_hz: f64,
        low: f64,
        high: f64,
        duty_cycle: f64,
        phase_cycles: f64,
    ) -> Result<Self, OscillatorError> {
        let freq = validate_non_negative("frequency_hz", frequency_hz)?;
        let l = validate_finite("low", low)?;
        let h = validate_finite("high", high)?;
        let duty = validate_finite("duty_cycle", duty_cycle)?;
        let phase = validate_finite("phase_cycles", phase_cycles)?;

        if duty <= 0.0 || duty >= 1.0 {
            return Err(OscillatorError::InvalidParameter {
                name: "duty_cycle",
                reason: format!("duty_cycle must satisfy 0.0 < duty_cycle < 1.0, got {duty}"),
            });
        }

        Ok(Self {
            frequency_hz: freq,
            low: l,
            high: h,
            duty_cycle: duty,
            phase_cycles: phase,
        })
    }

    pub fn frequency_hz(&self) -> f64 {
        self.frequency_hz
    }

    pub fn low(&self) -> f64 {
        self.low
    }

    pub fn high(&self) -> f64 {
        self.high
    }

    pub fn duty_cycle(&self) -> f64 {
        self.duty_cycle
    }

    pub fn phase_cycles(&self) -> f64 {
        self.phase_cycles
    }
}

impl ContinuousSignal for SquareOscillator {
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError> {
        let t = validate_finite("time_seconds", time_seconds)?;
        let pos = fractional_part(self.frequency_hz * t + self.phase_cycles);
        if pos < self.duty_cycle {
            Ok(self.high)
        } else {
            Ok(self.low)
        }
    }
}

/// Pulse oscillator: rectangular pulse wave with duty cycle, amplitude, and offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PulseOscillator {
    frequency_hz: f64,
    duty_cycle: f64,
    amplitude: f64,
    phase_cycles: f64,
    offset: f64,
}

impl PulseOscillator {
    /// Construct a pulse oscillator with default `amplitude = 1.0`, `phase_cycles = 0.0`, `offset = 0.0`.
    pub fn new(frequency_hz: f64, duty_cycle: f64) -> Result<Self, OscillatorError> {
        Self::with_options(frequency_hz, duty_cycle, 1.0, 0.0, 0.0)
    }

    /// Construct a pulse oscillator with custom options.
    pub fn with_options(
        frequency_hz: f64,
        duty_cycle: f64,
        amplitude: f64,
        phase_cycles: f64,
        offset: f64,
    ) -> Result<Self, OscillatorError> {
        let freq = validate_non_negative("frequency_hz", frequency_hz)?;
        let duty = validate_finite("duty_cycle", duty_cycle)?;
        let amp = validate_non_negative("amplitude", amplitude)?;
        let phase = validate_finite("phase_cycles", phase_cycles)?;
        let off = validate_finite("offset", offset)?;

        if duty <= 0.0 || duty >= 1.0 {
            return Err(OscillatorError::InvalidParameter {
                name: "duty_cycle",
                reason: format!("duty_cycle must satisfy 0.0 < duty_cycle < 1.0, got {duty}"),
            });
        }

        Ok(Self {
            frequency_hz: freq,
            duty_cycle: duty,
            amplitude: amp,
            phase_cycles: phase,
            offset: off,
        })
    }

    pub fn frequency_hz(&self) -> f64 {
        self.frequency_hz
    }

    pub fn duty_cycle(&self) -> f64 {
        self.duty_cycle
    }

    pub fn amplitude(&self) -> f64 {
        self.amplitude
    }

    pub fn phase_cycles(&self) -> f64 {
        self.phase_cycles
    }

    pub fn offset(&self) -> f64 {
        self.offset
    }
}

impl ContinuousSignal for PulseOscillator {
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError> {
        let t = validate_finite("time_seconds", time_seconds)?;
        let pos = fractional_part(self.frequency_hz * t + self.phase_cycles);
        if pos < self.duty_cycle {
            Ok(self.offset + self.amplitude)
        } else {
            Ok(self.offset - self.amplitude)
        }
    }
}

/// Sawtooth oscillator: linear rising ramp wave from -1 to +1 over each cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SawtoothOscillator {
    frequency_hz: f64,
    amplitude: f64,
    phase_cycles: f64,
    offset: f64,
}

impl SawtoothOscillator {
    /// Construct a standard sawtooth oscillator (`amplitude = 1.0`, `phase_cycles = 0.0`, `offset = 0.0`).
    pub fn new(frequency_hz: f64) -> Result<Self, OscillatorError> {
        Self::with_options(frequency_hz, 1.0, 0.0, 0.0)
    }

    /// Construct a sawtooth oscillator with custom amplitude, phase, and offset.
    pub fn with_options(
        frequency_hz: f64,
        amplitude: f64,
        phase_cycles: f64,
        offset: f64,
    ) -> Result<Self, OscillatorError> {
        let freq = validate_non_negative("frequency_hz", frequency_hz)?;
        let amp = validate_non_negative("amplitude", amplitude)?;
        let phase = validate_finite("phase_cycles", phase_cycles)?;
        let off = validate_finite("offset", offset)?;

        Ok(Self {
            frequency_hz: freq,
            amplitude: amp,
            phase_cycles: phase,
            offset: off,
        })
    }

    pub fn frequency_hz(&self) -> f64 {
        self.frequency_hz
    }

    pub fn amplitude(&self) -> f64 {
        self.amplitude
    }

    pub fn phase_cycles(&self) -> f64 {
        self.phase_cycles
    }

    pub fn offset(&self) -> f64 {
        self.offset
    }
}

impl ContinuousSignal for SawtoothOscillator {
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError> {
        let t = validate_finite("time_seconds", time_seconds)?;
        let pos = fractional_part(self.frequency_hz * t + self.phase_cycles);
        // Standard bipolar ramp: -1.0 at pos = 0.0, rising to +1.0 at pos approaching 1.0
        let raw = 2.0 * pos - 1.0;
        Ok(self.offset + self.amplitude * raw)
    }
}

/// Triangle oscillator: linear symmetric triangular wave in phase with sine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriangleOscillator {
    frequency_hz: f64,
    amplitude: f64,
    phase_cycles: f64,
    offset: f64,
}

impl TriangleOscillator {
    /// Construct a standard triangle oscillator (`amplitude = 1.0`, `phase_cycles = 0.0`, `offset = 0.0`).
    pub fn new(frequency_hz: f64) -> Result<Self, OscillatorError> {
        Self::with_options(frequency_hz, 1.0, 0.0, 0.0)
    }

    /// Construct a triangle oscillator with custom amplitude, phase, and offset.
    pub fn with_options(
        frequency_hz: f64,
        amplitude: f64,
        phase_cycles: f64,
        offset: f64,
    ) -> Result<Self, OscillatorError> {
        let freq = validate_non_negative("frequency_hz", frequency_hz)?;
        let amp = validate_non_negative("amplitude", amplitude)?;
        let phase = validate_finite("phase_cycles", phase_cycles)?;
        let off = validate_finite("offset", offset)?;

        Ok(Self {
            frequency_hz: freq,
            amplitude: amp,
            phase_cycles: phase,
            offset: off,
        })
    }

    pub fn frequency_hz(&self) -> f64 {
        self.frequency_hz
    }

    pub fn amplitude(&self) -> f64 {
        self.amplitude
    }

    pub fn phase_cycles(&self) -> f64 {
        self.phase_cycles
    }

    pub fn offset(&self) -> f64 {
        self.offset
    }
}

impl ContinuousSignal for TriangleOscillator {
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError> {
        let t = validate_finite("time_seconds", time_seconds)?;
        let pos = fractional_part(self.frequency_hz * t + self.phase_cycles);
        // Symmetric continuous triangle wave matching sine phase:
        // [0.00, 0.25): 0 to +1
        // [0.25, 0.75): +1 to -1
        // [0.75, 1.00): -1 to 0
        let raw = if pos < 0.25 {
            4.0 * pos
        } else if pos < 0.75 {
            2.0 - 4.0 * pos
        } else {
            4.0 * pos - 4.0
        };
        Ok(self.offset + self.amplitude * raw)
    }
}

// --- Sample Buffer and Uniform Sampler ---

/// Buffer of sampled floating-point values and grid timing metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleBuffer {
    samples: Vec<f64>,
    sample_rate_hz: f64,
    start_time_seconds: f64,
}

impl SampleBuffer {
    /// Construct a new sample buffer, validating sample finiteness and capacity bounds.
    pub fn new(
        samples: Vec<f64>,
        sample_rate_hz: f64,
        start_time_seconds: f64,
    ) -> Result<Self, OscillatorError> {
        if samples.len() > MAX_SAMPLES {
            return Err(OscillatorError::SampleCountTooLarge {
                count: samples.len(),
                max: MAX_SAMPLES,
            });
        }
        for (i, &s) in samples.iter().enumerate() {
            if !s.is_finite() {
                return Err(OscillatorError::NonFiniteNumber {
                    name: "sample",
                    value: s,
                });
            }
            let _ = i;
        }

        let rate = validate_sample_rate("sample_rate_hz", sample_rate_hz)?;
        let start_time = validate_finite("start_time_seconds", start_time_seconds)?;

        Ok(Self {
            samples,
            sample_rate_hz: rate,
            start_time_seconds: start_time,
        })
    }

    /// Return the number of stored samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Return the sample period in seconds ($1 / f_s$).
    pub fn sample_period_seconds(&self) -> f64 {
        1.0 / self.sample_rate_hz
    }

    /// Return the covered duration of this half-open sample buffer in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.samples.len() as f64 / self.sample_rate_hz
    }

    /// Return the timestamp corresponding to sample index `index`.
    pub fn time_at(&self, index: usize) -> Result<f64, OscillatorError> {
        if index >= self.samples.len() {
            return Err(OscillatorError::IndexOutOfBounds {
                index,
                count: self.samples.len(),
            });
        }
        time_at_sample(index, self.sample_rate_hz, self.start_time_seconds)
    }

    /// Immutably borrow the slice of floating-point samples.
    pub fn samples(&self) -> &[f64] {
        &self.samples
    }

    /// Consume the buffer and return the underlying sample vector.
    pub fn into_samples(self) -> Vec<f64> {
        self.samples
    }

    /// Return the sampling rate in Hertz.
    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }

    /// Return the start timestamp in seconds.
    pub fn start_time_seconds(&self) -> f64 {
        self.start_time_seconds
    }
}

/// Uniform sampler that evaluates a continuous signal at regular time intervals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UniformSampler {
    sample_rate_hz: f64,
}

impl UniformSampler {
    /// Construct a uniform sampler with a valid sample rate.
    pub fn new(sample_rate_hz: f64) -> Result<Self, OscillatorError> {
        let rate = validate_sample_rate("sample_rate_hz", sample_rate_hz)?;
        Ok(Self { sample_rate_hz: rate })
    }

    /// Sample `signal` for `duration_seconds` starting at `start_time_seconds = 0.0`.
    pub fn sample(
        &self,
        signal: &dyn ContinuousSignal,
        duration_seconds: f64,
    ) -> Result<SampleBuffer, OscillatorError> {
        self.sample_with_start_time(signal, duration_seconds, 0.0)
    }

    /// Sample `signal` for `duration_seconds` starting at `start_time_seconds`.
    pub fn sample_with_start_time(
        &self,
        signal: &dyn ContinuousSignal,
        duration_seconds: f64,
        start_time_seconds: f64,
    ) -> Result<SampleBuffer, OscillatorError> {
        let count = sample_count_for_duration(duration_seconds, self.sample_rate_hz)?;
        self.sample_count_with_start_time(signal, count, start_time_seconds)
    }

    /// Sample exactly `count` samples from `signal` starting at `start_time_seconds = 0.0`.
    pub fn sample_count(
        &self,
        signal: &dyn ContinuousSignal,
        count: usize,
    ) -> Result<SampleBuffer, OscillatorError> {
        self.sample_count_with_start_time(signal, count, 0.0)
    }

    /// Sample exactly `count` samples from `signal` starting at `start_time_seconds`.
    pub fn sample_count_with_start_time(
        &self,
        signal: &dyn ContinuousSignal,
        count: usize,
        start_time_seconds: f64,
    ) -> Result<SampleBuffer, OscillatorError> {
        if count > MAX_SAMPLES {
            return Err(OscillatorError::SampleCountTooLarge {
                count,
                max: MAX_SAMPLES,
            });
        }
        let start_time = validate_finite("start_time_seconds", start_time_seconds)?;

        // Checked pre-allocation strictly adhering to CWE-400 bounds
        let mut samples = Vec::with_capacity(count);
        for index in 0..count {
            let t = time_at_sample(index, self.sample_rate_hz, start_time)?;
            let sample_val = signal.value_at(t)?;
            samples.push(sample_val);
        }

        SampleBuffer::new(samples, self.sample_rate_hz, start_time)
    }

    /// Return this sampler's Nyquist frequency.
    pub fn nyquist_frequency(&self) -> f64 {
        self.sample_rate_hz / 2.0
    }

    /// Return the sampling rate in Hertz.
    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }
}
