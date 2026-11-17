//! # Note Audio (MUS01 Stages 1-5)
//!
//! Orchestrates the note-to-sound signal chain from a high-level note
//! representation (note names, MIDI note numbers, or raw frequencies) through
//! continuous mathematical oscillators to discrete PCM audio buffers:
//!
//! ```text
//! typed note -> frequency -> oscillator -> sampler -> PCM audio
//! ```
//!
//! This crate is strictly pure computation with zero filesystem, network, or
//! operating system access (`[]` capabilities).
//!
//! ## Stages Covered
//!
//! 1. **Note to Frequency** (MUS00): Note names (e.g. `"A4"`, `"C#5"`), MIDI numbers
//!    (`0..=127`), and validated frequencies are mapped to Hertz.
//! 2. **Frequency to Oscillator** (OSC00): Waveform generators ([`SineOscillator`],
//!    [`SquareOscillator`], [`PulseOscillator`], [`SawtoothOscillator`], [`TriangleOscillator`]).
//! 3. **Oscillator to Sampler** (OSC00): Discretization onto a uniform time grid
//!    using [`UniformSampler`].
//! 4. **Floating Samples to PCM Audio** (MUS01 Stage 5): Quantization and clipping
//!    into signed 16-bit little-endian PCM via [`pcm_audio::PcmAudio`].
//!
//! ## Verification & Security Guarantees
//!
//! - **Safety**: `#![forbid(unsafe_code)]`, zero external dependencies.
//! - **CWE-400 (Resource Exhaustion)**: Rate, duration, and sample counts are
//!   bounded strictly before any buffer allocation. Sample budgets are enforced
//!   with [`DEFAULT_MAX_SAMPLE_COUNT`] and [`MAX_SAMPLES`].
//! - **CWE-190 (Integer Overflow)**: Checked arithmetic on sample counts and sample rates.
//! - **CWE-20 (Improper Input Validation)**: Rejects `NaN`, `+-Inf`, negative durations,
//!   and note frequencies at or above the Nyquist frequency (`sample_rate / 2.0`).

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

use note_frequency::Note;
use oscillator::{
    sample_count_for_duration, validate_finite, validate_non_negative, PulseOscillator,
    SampleBuffer, SawtoothOscillator, SineOscillator, SquareOscillator, TriangleOscillator,
    UniformSampler,
};
use pcm_audio::{encode_sample_buffer, PcmAudio, PcmError, PcmFormat};

pub use note_frequency::Note as ParsedNote;
pub use oscillator::{
    ContinuousSignal, ContinuousSignal as AnalogSignal, OscillatorError, MAX_DURATION_SECONDS,
    MAX_SAMPLES, MAX_SAMPLE_RATE_HZ,
};
pub use pcm_audio::{
    DEFAULT_BIT_DEPTH, DEFAULT_CHANNEL_COUNT, DEFAULT_FULL_SCALE_VOLTAGE, DEFAULT_SAMPLE_RATE_HZ,
    PCM16_MAX, PCM16_MIN,
};

/// Type alias for Python/TypeScript parity.
pub type PCMBuffer = PcmAudio;

/// Type alias for Python/TypeScript parity.
pub type PCMFormat = PcmFormat;

/// Default normalized audio amplitude (0.8).
pub const DEFAULT_AMPLITUDE: f64 = 0.8;

/// Default maximum allowed samples in a single note render (10,000,000).
pub const DEFAULT_MAX_SAMPLE_COUNT: usize = 10_000_000;

/// Standard MIDI tuning frequency for A4 (MIDI note 69).
pub const MIDI_A4_FREQUENCY_HZ: f64 = 440.0;

/// Standard MIDI note number for A4.
pub const MIDI_A4_NOTE_NUMBER: u8 = 69;

/// Errors produced during note audio rendering and parameter validation.
#[derive(Debug, Clone, PartialEq)]
pub enum NoteAudioError {
    /// Note spelling or syntax was invalid.
    InvalidNote(String),
    /// MIDI note number was out of range (0..=127).
    InvalidMidi { midi: u8, reason: String },
    /// Frequency was non-finite, zero, or negative.
    InvalidFrequency { frequency_hz: f64, reason: String },
    /// Note frequency violated the Nyquist limit for the given sample rate.
    NyquistViolation {
        frequency_hz: f64,
        nyquist_hz: f64,
    },
    /// Render would produce more samples than the configured budget.
    SampleBudgetExceeded { requested: usize, limit: usize },
    /// A float value was not finite (NaN, +-Inf).
    NonFiniteNumber { name: &'static str, value: f64 },
    /// A parameter failed validation.
    InvalidParameter {
        name: &'static str,
        reason: String,
    },
    /// An underlying oscillator error occurred.
    Oscillator(OscillatorError),
    /// An underlying PCM encoding error occurred.
    Pcm(PcmError),
}

impl fmt::Display for NoteAudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNote(msg) => write!(f, "invalid note: {msg}"),
            Self::InvalidMidi { midi, reason } => {
                write!(f, "invalid MIDI note number {midi}: {reason}")
            }
            Self::InvalidFrequency {
                frequency_hz,
                reason,
            } => write!(f, "invalid frequency {frequency_hz} Hz: {reason}"),
            Self::NyquistViolation {
                frequency_hz,
                nyquist_hz,
            } => write!(
                f,
                "frequency {frequency_hz} Hz must be strictly below Nyquist frequency {nyquist_hz} Hz"
            ),
            Self::SampleBudgetExceeded { requested, limit } => write!(
                f,
                "render would create {requested} samples, exceeding budget of {limit}"
            ),
            Self::NonFiniteNumber { name, value } => {
                write!(f, "{name} must be finite, got {value}")
            }
            Self::InvalidParameter { name, reason } => {
                write!(f, "invalid parameter {name}: {reason}")
            }
            Self::Oscillator(err) => write!(f, "oscillator error: {err}"),
            Self::Pcm(err) => write!(f, "PCM error: {err}"),
        }
    }
}

impl Error for NoteAudioError {}

impl From<OscillatorError> for NoteAudioError {
    fn from(error: OscillatorError) -> Self {
        Self::Oscillator(error)
    }
}

impl From<PcmError> for NoteAudioError {
    fn from(error: PcmError) -> Self {
        Self::Pcm(error)
    }
}

/// Convert a MIDI note number (0..=127) to frequency in Hertz.
///
/// Uses standard equal temperament with A4 (MIDI 69) = 440.0 Hz:
///
/// $$f = 440.0 \times 2^{(m - 69) / 12}$$
pub fn midi_to_frequency(midi: u8) -> Result<f64, NoteAudioError> {
    if midi > 127 {
        return Err(NoteAudioError::InvalidMidi {
            midi,
            reason: format!("must be in 0..=127, got {midi}"),
        });
    }
    let semitones = f64::from(midi) - f64::from(MIDI_A4_NOTE_NUMBER);
    let freq = MIDI_A4_FREQUENCY_HZ * 2.0f64.powf(semitones / 12.0);
    Ok(freq)
}

/// Convert a note name (e.g. `"A4"`, `"C#5"`, `"Db3"`) to frequency in Hertz.
pub fn note_to_frequency(name: &str) -> Result<f64, NoteAudioError> {
    note_frequency::note_to_frequency(name).map_err(NoteAudioError::InvalidNote)
}

/// Validate a frequency in Hertz, ensuring it is finite and strictly positive.
pub fn validate_frequency(freq: f64) -> Result<f64, NoteAudioError> {
    if !freq.is_finite() {
        return Err(NoteAudioError::NonFiniteNumber {
            name: "frequency_hz",
            value: freq,
        });
    }
    if freq <= 0.0 {
        return Err(NoteAudioError::InvalidFrequency {
            frequency_hz: freq,
            reason: format!("must be > 0.0, got {freq}"),
        });
    }
    Ok(freq)
}

/// Pitch representation for musical note rendering.
#[derive(Debug, Clone, PartialEq)]
pub enum NotePitch {
    /// Note by name, e.g. `"A4"`, `"C#5"`, `"Eb3"`.
    Name(String),
    /// Pre-parsed `Note` struct.
    Parsed(Note),
    /// MIDI note number (`0..=127`).
    Midi(u8),
    /// Direct frequency in Hertz.
    Frequency(f64),
}

impl NotePitch {
    /// Resolve the pitch to a concrete frequency in Hertz.
    pub fn to_frequency(&self) -> Result<f64, NoteAudioError> {
        match self {
            Self::Name(name) => note_to_frequency(name),
            Self::Parsed(note) => {
                let freq = note.frequency();
                validate_frequency(freq)
            }
            Self::Midi(midi) => midi_to_frequency(*midi),
            Self::Frequency(freq) => validate_frequency(*freq),
        }
    }
}

impl From<&str> for NotePitch {
    fn from(s: &str) -> Self {
        Self::Name(s.to_string())
    }
}

impl From<String> for NotePitch {
    fn from(s: String) -> Self {
        Self::Name(s)
    }
}

impl From<Note> for NotePitch {
    fn from(note: Note) -> Self {
        Self::Parsed(note)
    }
}

impl From<u8> for NotePitch {
    fn from(midi: u8) -> Self {
        Self::Midi(midi)
    }
}

impl From<f64> for NotePitch {
    fn from(freq: f64) -> Self {
        Self::Frequency(freq)
    }
}

/// Supported waveform families for note rendering.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Waveform {
    /// Smooth sinusoidal wave (standard MUS01 tone).
    #[default]
    Sine,
    /// Square wave (50% duty cycle).
    Square,
    /// Rectangular pulse wave with configurable duty cycle in `(0.0, 1.0)`.
    Pulse(f64),
    /// Sawtooth wave (linear rising ramp).
    Sawtooth,
    /// Triangle wave (symmetric triangle).
    Triangle,
}

/// Concrete oscillator signal wrapping any supported waveform model.
#[derive(Debug, Clone, PartialEq)]
pub enum OscillatorSignal {
    /// Sine wave oscillator.
    Sine(SineOscillator),
    /// Square wave oscillator.
    Square(SquareOscillator),
    /// Rectangular pulse wave oscillator.
    Pulse(PulseOscillator),
    /// Sawtooth wave oscillator.
    Sawtooth(SawtoothOscillator),
    /// Triangle wave oscillator.
    Triangle(TriangleOscillator),
}

impl ContinuousSignal for OscillatorSignal {
    fn value_at(&self, time_seconds: f64) -> Result<f64, OscillatorError> {
        match self {
            Self::Sine(s) => s.value_at(time_seconds),
            Self::Square(s) => s.value_at(time_seconds),
            Self::Pulse(s) => s.value_at(time_seconds),
            Self::Sawtooth(s) => s.value_at(time_seconds),
            Self::Triangle(s) => s.value_at(time_seconds),
        }
    }
}

impl Waveform {
    /// Build a concrete [`OscillatorSignal`] with the given frequency, amplitude, and phase.
    pub fn build_oscillator(
        &self,
        frequency_hz: f64,
        amplitude: f64,
        phase_cycles: f64,
    ) -> Result<OscillatorSignal, NoteAudioError> {
        let osc = match self {
            Self::Sine => OscillatorSignal::Sine(SineOscillator::with_options(
                frequency_hz,
                amplitude,
                phase_cycles,
                0.0,
            )?),
            Self::Square => OscillatorSignal::Square(SquareOscillator::with_options(
                frequency_hz,
                -amplitude,
                amplitude,
                0.5,
                phase_cycles,
            )?),
            Self::Pulse(duty) => OscillatorSignal::Pulse(PulseOscillator::with_options(
                frequency_hz,
                amplitude,
                *duty,
                phase_cycles,
                0.0,
            )?),
            Self::Sawtooth => OscillatorSignal::Sawtooth(SawtoothOscillator::with_options(
                frequency_hz,
                amplitude,
                phase_cycles,
                0.0,
            )?),
            Self::Triangle => OscillatorSignal::Triangle(TriangleOscillator::with_options(
                frequency_hz,
                amplitude,
                phase_cycles,
                0.0,
            )?),
        };
        Ok(osc)
    }
}

/// A musical note event combining pitch, duration, amplitude, start time, and waveform.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteEvent {
    pitch: NotePitch,
    duration_seconds: f64,
    amplitude: f64,
    start_time_seconds: f64,
    waveform: Waveform,
}

impl NoteEvent {
    /// Create a new note event with default amplitude (0.8), start time (0.0), and sine waveform.
    pub fn new(
        pitch: impl Into<NotePitch>,
        duration_seconds: f64,
    ) -> Result<Self, NoteAudioError> {
        let duration = validate_non_negative("duration_seconds", duration_seconds)
            .map_err(NoteAudioError::from)?;
        if duration > MAX_DURATION_SECONDS {
            return Err(NoteAudioError::from(OscillatorError::DurationTooLong {
                duration,
                max: MAX_DURATION_SECONDS,
            }));
        }
        let note_pitch = pitch.into();
        // Validate pitch immediately fail-closed.
        let _ = note_pitch.to_frequency()?;

        Ok(Self {
            pitch: note_pitch,
            duration_seconds: duration,
            amplitude: DEFAULT_AMPLITUDE,
            start_time_seconds: 0.0,
            waveform: Waveform::Sine,
        })
    }

    /// Set a custom amplitude (must be non-negative and finite).
    pub fn with_amplitude(mut self, amplitude: f64) -> Result<Self, NoteAudioError> {
        self.amplitude = validate_non_negative("amplitude", amplitude)?;
        Ok(self)
    }

    /// Set a custom start time in seconds (must be finite).
    pub fn with_start_time(mut self, start_time_seconds: f64) -> Result<Self, NoteAudioError> {
        self.start_time_seconds = validate_finite("start_time_seconds", start_time_seconds)?;
        Ok(self)
    }

    /// Set a custom waveform.
    pub fn with_waveform(mut self, waveform: Waveform) -> Self {
        self.waveform = waveform;
        self
    }

    /// The note's pitch specification.
    pub fn pitch(&self) -> &NotePitch {
        &self.pitch
    }

    /// The note duration in seconds.
    pub fn duration_seconds(&self) -> f64 {
        self.duration_seconds
    }

    /// The note normalized amplitude.
    pub fn amplitude(&self) -> f64 {
        self.amplitude
    }

    /// The note start time in seconds.
    pub fn start_time_seconds(&self) -> f64 {
        self.start_time_seconds
    }

    /// The note waveform.
    pub fn waveform(&self) -> Waveform {
        self.waveform
    }

    /// Resolve this note's frequency in Hertz.
    pub fn frequency(&self) -> Result<f64, NoteAudioError> {
        self.pitch.to_frequency()
    }
}

/// The inspectable educational chain representing a rendered note.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedNote {
    note_event: NoteEvent,
    frequency_hz: f64,
    oscillator: OscillatorSignal,
    floating_samples: SampleBuffer,
    pcm_audio: PcmAudio,
}

impl RenderedNote {
    /// Borrow the originating [`NoteEvent`].
    pub fn note_event(&self) -> &NoteEvent {
        &self.note_event
    }

    /// The resolved fundamental frequency in Hertz.
    pub fn frequency_hz(&self) -> f64 {
        self.frequency_hz
    }

    /// Borrow the continuous [`OscillatorSignal`].
    pub fn oscillator(&self) -> &OscillatorSignal {
        &self.oscillator
    }

    /// Borrow the discrete floating-point [`SampleBuffer`].
    pub fn floating_samples(&self) -> &SampleBuffer {
        &self.floating_samples
    }

    /// Borrow the quantized [`PcmAudio`] buffer.
    pub fn pcm_audio(&self) -> &PcmAudio {
        &self.pcm_audio
    }

    /// Consume this rendered note and return its [`PcmAudio`] buffer.
    pub fn into_pcm_audio(self) -> PcmAudio {
        self.pcm_audio
    }
}

/// Render a [`NoteEvent`] into a complete, inspectable [`RenderedNote`] chain.
///
/// Uses [`DEFAULT_MAX_SAMPLE_COUNT`] as the sample budget limit.
pub fn render_note(
    event: &NoteEvent,
    sample_rate_hz: f64,
) -> Result<RenderedNote, NoteAudioError> {
    render_note_with_limit(event, sample_rate_hz, DEFAULT_MAX_SAMPLE_COUNT)
}

/// Render a [`NoteEvent`] into a [`RenderedNote`] with an explicit sample budget.
pub fn render_note_with_limit(
    event: &NoteEvent,
    sample_rate_hz: f64,
    max_sample_count: usize,
) -> Result<RenderedNote, NoteAudioError> {
    let rate = validate_finite("sample_rate_hz", sample_rate_hz)?;
    if rate <= 0.0 {
        return Err(NoteAudioError::InvalidParameter {
            name: "sample_rate_hz",
            reason: format!("must be > 0.0, got {rate}"),
        });
    }
    if rate > MAX_SAMPLE_RATE_HZ {
        return Err(NoteAudioError::from(OscillatorError::SampleRateTooHigh {
            rate,
            max: MAX_SAMPLE_RATE_HZ,
        }));
    }

    let frequency = event.frequency()?;
    let nyquist = rate / 2.0;
    if frequency >= nyquist {
        return Err(NoteAudioError::NyquistViolation {
            frequency_hz: frequency,
            nyquist_hz: nyquist,
        });
    }

    let count = sample_count_for_duration(event.duration_seconds, rate)?;
    if count > max_sample_count {
        return Err(NoteAudioError::SampleBudgetExceeded {
            requested: count,
            limit: max_sample_count,
        });
    }

    let osc = event
        .waveform
        .build_oscillator(frequency, event.amplitude, 0.0)?;
    let floating_samples = UniformSampler::new(rate)?.sample_with_start_time(
        &osc,
        event.duration_seconds,
        event.start_time_seconds,
    )?;

    let pcm = encode_sample_buffer(&floating_samples)?;

    Ok(RenderedNote {
        note_event: event.clone(),
        frequency_hz: frequency,
        oscillator: osc,
        floating_samples,
        pcm_audio: pcm,
    })
}

/// Convenience helper to render a note name/pitch and duration with default parameters.
///
/// Returns the full [`RenderedNote`] chain sampled at 44,100 Hz.
pub fn render_note_to_sound_chain(
    pitch: impl Into<NotePitch>,
    duration_seconds: f64,
) -> Result<RenderedNote, NoteAudioError> {
    let event = NoteEvent::new(pitch, duration_seconds)?;
    render_note(&event, DEFAULT_SAMPLE_RATE_HZ)
}
